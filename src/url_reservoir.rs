use stats;

use futures;
use hyper;
use hyper::rt::Stream;
use url;

use std::collections::VecDeque;
use std::sync;
use std::str::FromStr;
use std::thread;
use std::time;


pub struct UrlReservoir {
    url_deque: sync::Mutex<Option<VecDeque<url::Url>>>,
    stats: sync::Arc<stats::Stats>,
}

enum GetUrlError {
    IsLocked,
    IsEmpty,
    IllFormattedUrl,
    IsClosed,
}

impl UrlReservoir {
    pub fn new(stats: sync::Arc<stats::Stats>, capacity: usize) -> sync::Arc<UrlReservoir>{
        sync::Arc::new(UrlReservoir{
            url_deque: sync::Mutex::new(Some(VecDeque::with_capacity(capacity))),
            stats: stats,
        })
    }

    pub fn close(&self){
        match self.url_deque.lock(){
            Err(e) => panic!(format!("add_html_urls tried to get a poisoned lock: {:?}", e)),
            Ok(mut maybe_url_deque) => {
                maybe_url_deque.take();
            },
        }
    }

    // fn get_len_capacity(&self) -> (usize, usize){
    //     match self.url_deque.lock(){
    //         Err(e) => (panic!(format!("get_len_capacity tried to get a poisoned lock: {:?}", e))),
    //         Ok(mut url_deque) => {
    //             if let Some(url_deque)=url_deque.as_mut(){
    //                 (url_deque.len(), url_deque.capacity())
    //             } else{
    //                 panic!("get_len_capacity called on None deque");
    //             }
    //         },
    //     }
    // }

    pub fn add_urls(&self, url_vec: Vec<url::Url>){
        self.stats.urlreservoir_add_calls.fetch_add(1, sync::atomic::Ordering::Relaxed);

        let url_vec_len=url_vec.len();
        match self.url_deque.lock(){
            Err(e) => panic!(format!("add_html_urls tried to get a poisoned lock: {:?}", e)),
            Ok(mut url_deque) => {
                if let Some(url_deque)=url_deque.as_mut(){
                    url_deque.extend(url_vec);
                    self.stats.urlreservoir_size.store(url_deque.len(), sync::atomic::Ordering::Relaxed);
                }
            },
        }

        self.stats.urlreservoir_add_addedurls.fetch_add(url_vec_len, sync::atomic::Ordering::Relaxed);
    }

    pub fn add_urls_if_len_lt_cap(&self, url_vec: Vec<url::Url>){
        self.stats.urlreservoir_add_calls.fetch_add(1, sync::atomic::Ordering::Relaxed);

        let url_vec_len=url_vec.len();
        match self.url_deque.lock(){
            Err(e) => panic!(format!("add_urls_if_len_lt_cap tried to get a poisoned lock: {:?}", e)),
            Ok(mut url_deque) => {
                if let Some(url_deque)=url_deque.as_mut(){
                    // If the capacity would be exceeded, which would trigger a doubling of the buffer: return early
                    if url_deque.len()+url_vec_len>=url_deque.capacity(){
                        return;
                    }
                    url_deque.extend(url_vec);
                    self.stats.urlreservoir_size.store(url_deque.len(), sync::atomic::Ordering::Relaxed);
                }
            },
        }

        self.stats.urlreservoir_add_addedurls.fetch_add(url_vec_len, sync::atomic::Ordering::Relaxed);
    }

    fn try_get_url_and_uri(&self) -> Result<(url::Url, hyper::Uri), GetUrlError>{
        self.stats.urlreservoir_getuu_calls.fetch_add(1, sync::atomic::Ordering::Relaxed);

        // Get one url if deque is not empty, none otherwise; also get deque length
        let (popped_url, url_deque_len)=match self.url_deque.try_lock() {
            Ok(mut maybe_url_deque) => {
                match maybe_url_deque.as_mut() {
                    Some(url_deque) => (url_deque.pop_front(), url_deque.len()),
                    None => {
                        return Err(GetUrlError::IsClosed);
                    },
                }
            },
            Err(_) => {
                self.stats.urlreservoir_getuu_locked.fetch_add(1, sync::atomic::Ordering::Relaxed);
                return Err(GetUrlError::IsLocked);
            },
        };
        self.stats.urlreservoir_size.store(url_deque_len, sync::atomic::Ordering::Relaxed);

        // Unwrap url if one was popped from deque; return early otherwise
        let url=match popped_url {
            Some(url) => url,
            None => {
                self.stats.urlreservoir_getuu_emptyqueue.fetch_add(1, sync::atomic::Ordering::Relaxed);
                return Err(GetUrlError::IsEmpty);
            },
        };

        // Produce uri from url; return early on any error
        let uri=match hyper::Uri::from_str(url.as_str()) {
            Err(_) => {
                self.stats.urlreservoir_getuu_illformat.fetch_add(1, sync::atomic::Ordering::Relaxed);
                return Err(GetUrlError::IllFormattedUrl);
            },
            Ok(uri) => {
                if uri.authority_part().map(|auth| auth.as_str().contains("@")).unwrap_or(true){ //Gets around a bug in the http crate
                    self.stats.urlreservoir_getuu_illformat.fetch_add(1, sync::atomic::Ordering::Relaxed);
                    return Err(GetUrlError::IllFormattedUrl);
                } else{
                    uri
                }
            },
        };

        self.stats.urlreservoir_getuu_success.fetch_add(1, sync::atomic::Ordering::Relaxed);
        return Ok((url,uri));
    }
}

impl Drop for UrlReservoir {
    fn drop(&mut self){
        println!("UrlReservoir dropped.");
    }
}


pub struct UrlReservoirUrlUriStream{
    url_reservoir: sync::Arc<UrlReservoir>
}

impl UrlReservoirUrlUriStream{
    pub fn new(url_reservoir: sync::Arc<UrlReservoir>) -> UrlReservoirUrlUriStream{
        UrlReservoirUrlUriStream{
            url_reservoir: url_reservoir
        }
    }
}

impl Stream for UrlReservoirUrlUriStream {
    type Item=(url::Url, hyper::Uri);
    type Error=();

    fn poll(&mut self) -> Result<futures::Async<Option<Self::Item>>, Self::Error>{
        match self.url_reservoir.try_get_url_and_uri() {
            Ok(urluri) => Ok(futures::Async::Ready(Some(urluri))),
            Err(e) => {
                match e {
                    GetUrlError::IsClosed => Ok(futures::Async::Ready(None)),
                    GetUrlError::IsEmpty => {
                        let task=futures::task::current();
                        thread::spawn(move||{
                            thread::sleep(time::Duration::from_millis(2000));
                            task.notify();
                        });
                        Ok(futures::Async::NotReady)
                    },
                    _ => {
                        futures::task::current().notify();
                        Ok(futures::Async::NotReady)
                    },
                }
            },
        }
    }
}


// #[cfg(test)]
// mod tests {
//     use super::*;
//     use std::str;

//     #[test]
//     fn test_add_urls_if_len_lt_cap() {
//         for capacity in 1..100{
//             let stats=stats::Stats::new();
//             let url_reservoir=UrlReservoir::new(stats.clone(), capacity);
//             let (_,capacity)=url_reservoir.get_len_capacity();
//                 println!("asd");

//             for step in 1..(capacity+1){

//                 for i in 0..(capacity/step+1){
//                     let new_urls=vec![url::Url::parse("http://www.site.com").unwrap();step];
//                     url_reservoir.add_urls_if_len_lt_cap(new_urls);
//                     let (_,new_capacity)=url_reservoir.get_len_capacity();
//                     println!("capacity: {}, new_capacity: {}, step: {}, i: {}", capacity, new_capacity, step, i);
//                     assert!(new_capacity==capacity);
//                 }
//             }
//         }
//     }
// }