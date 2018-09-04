use url_reservoir;
use bloom_filter;
use stats;

use regex;
use url;
use sync;

const MAX_URLS_PER_HTML: usize = 100;
const MAX_URL_HOSTS_PER_HTML: usize = 4;
fn html_to_urls(re: &regex::Regex, url: url::Url, html_content: String) -> Vec<url::Url>{
    let mut url_vec:Vec<url::Url>=Vec::with_capacity(MAX_URLS_PER_HTML);

    for cap in re.captures_iter(html_content.as_str()).take(MAX_URLS_PER_HTML){
        let cap_str=match cap.get(1) {
            Some(cap) => cap.as_str(),
            None => continue,
        };

        let cap_url=match url.join(cap_str) {
            Ok(cap_url) => cap_url,
            Err(_) => continue,
        };

        let num_share_host=url_vec
            .iter()
            .filter(|url| url.host()==cap_url.host())
            .count();

        if num_share_host>=MAX_URL_HOSTS_PER_HTML{
            continue;
        }

        url_vec.push(cap_url);
    }

    url_vec
}


pub enum Work{
    Html(url::Url, Vec<u8>),
    Png(Vec<u8>)
}

// impl Work {
//     pub fn len(&self) -> usize{
//         match self {
//             Work::Html(_,v) => v.len(),
//             Work::Png(v) => v.len(),
//         }
//     }
// }


pub fn run_worker(work_receiver: sync::mpsc::Receiver<Work>, url_reservoir: sync::Arc<url_reservoir::UrlReservoir>, stats: sync::Arc<stats::Stats>){
    let re=regex::Regex::new("(?:href=|src=|url=)[\"']?([^\"' <>]*)").unwrap();
    let mut url_bloom_filter=bloom_filter::BloomFilter::new(vec![1238967,4567654]);
    for work in work_receiver.iter(){
        match work {
            Work::Html(url,bytes) => {
                stats.worker_got_html.fetch_add(1, sync::atomic::Ordering::Relaxed);
                if let Ok(html_content)=String::from_utf8(bytes){
                    let mut urls=html_to_urls(&re, url, html_content);
                    urls.retain(|u| !url_bloom_filter.contains_add(u.as_str().as_bytes()));
                    url_reservoir.add_urls(urls);
                }
            },
            Work::Png(_bytes) => {
                stats.worker_got_file.fetch_add(1, sync::atomic::Ordering::Relaxed);
                // println!("Got image of length {}.", bytes.len());
            },
        }
    }
    println!("Worker rerminated.");
}