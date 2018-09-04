use stats;

use futures;
use hyper;
use hyper::rt::Stream;
use url;

use std::collections::VecDeque;
use std::sync;
use std::str::FromStr;



const URL_RESERVOIR_STARTING_CAPACITY: usize = 1<<20;
pub struct UrlReservoir {
    url_deque: sync::Mutex<VecDeque<url::Url>>,
    stats: sync::Arc<stats::Stats>,
}

enum GetUrlError {
    IsLocked,
    IsEmpty,
    IllFormattedUrl,
}

impl UrlReservoir {
    pub fn new(stats: sync::Arc<stats::Stats>) -> sync::Arc<UrlReservoir>{
        sync::Arc::new(UrlReservoir{
            url_deque: sync::Mutex::new(VecDeque::with_capacity(URL_RESERVOIR_STARTING_CAPACITY)),
            stats: stats,
        })
    }

    pub fn add_urls(&self, url_vec: Vec<url::Url>){
        self.stats.urlreservoir_add_calls.fetch_add(1, sync::atomic::Ordering::Relaxed);

        let url_vec_len=url_vec.len();
        match self.url_deque.lock(){
            Err(e) => panic!(format!("add_html_urls tried to get a poisoned lock: {:?}", e)),
            Ok(mut url_deque) => {
                self.stats.urlreservoir_size.store(url_deque.len(), sync::atomic::Ordering::Relaxed);
                url_deque.extend(url_vec)
            },
        }

        self.stats.urlreservoir_add_addedurls.fetch_add(url_vec_len, sync::atomic::Ordering::Relaxed);
    }

    fn try_get_url_and_uri(&self) -> Result<(url::Url, hyper::Uri), GetUrlError>{
        self.stats.urlreservoir_getuu_calls.fetch_add(1, sync::atomic::Ordering::Relaxed);

        let (popped_url, url_deque_len)={
            let mut url_deque=if let Ok(url_deque)=self.url_deque.try_lock(){
                url_deque
            } else{
                self.stats.urlreservoir_getuu_locked.fetch_add(1, sync::atomic::Ordering::Relaxed);
                return Err(GetUrlError::IsLocked);
            };

            (url_deque.pop_front(), url_deque.len())
        };
        self.stats.urlreservoir_size.store(url_deque_len, sync::atomic::Ordering::Relaxed);

        let url=if let Some(url)=popped_url{
            url
        } else{
            self.stats.urlreservoir_getuu_emptyqueue.fetch_add(1, sync::atomic::Ordering::Relaxed);
            return Err(GetUrlError::IsEmpty);
        };

        let uri=if let Ok(uri)=hyper::Uri::from_str(url.as_str()){
            uri
        } else{
            self.stats.urlreservoir_getuu_illformat.fetch_add(1, sync::atomic::Ordering::Relaxed);
            return Err(GetUrlError::IllFormattedUrl)
        };

        self.stats.urlreservoir_getuu_success.fetch_add(1, sync::atomic::Ordering::Relaxed);
        return Ok((url,uri));
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
            Err(_) => {
                futures::task::current().notify();
                Ok(futures::Async::NotReady)
            },
        }
    }
}
