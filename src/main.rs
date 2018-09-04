mod url_reservoir;
mod client;
mod worker;
mod stats;
mod murmur;
mod bloom_filter;
mod reporter;
mod config;

extern crate hyper;
extern crate tokio_core;
extern crate futures;
extern crate regex;
extern crate url;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
use std::thread;
use std::sync;









fn main() {
    let config=config::Config::get_or_panic("config.json");

    let stats=stats::Stats::new();

    let url_reservoir=url_reservoir::UrlReservoir::new(stats.clone());
    // url_reservoir.add_urls((0..20000).map(|_| url::Url::parse("http://c024:8080/").unwrap()).collect());
    url_reservoir.add_urls(config.initial_urls.iter().map(|url| url::Url::parse(url).unwrap()).collect());
    let (work_sender, work_receiver) = sync::mpsc::channel::<worker::Work>();

    for _ in 0..(config.client_threads-1){
        let url_reservoir_url_uri_stream=url_reservoir::UrlReservoirUrlUriStream::new(url_reservoir.clone());
        let work_sender2=work_sender.clone();
        let stats2=stats.clone();
        let config2=config.clone();
        thread::spawn(move|| client::run_client(url_reservoir_url_uri_stream, work_sender2, stats2, config2));
    }

    let url_reservoir_url_uri_stream=url_reservoir::UrlReservoirUrlUriStream::new(url_reservoir.clone());
    let stats2=stats.clone();
    let config2=config.clone();
    thread::spawn(move|| client::run_client(url_reservoir_url_uri_stream, work_sender, stats2, config2));

    let url_reservoir2=url_reservoir.clone();
    let stats2=stats.clone();
    let config2=config.clone();
    thread::spawn(move|| worker::run_worker(work_receiver, url_reservoir2, stats2, config2));

    reporter::run_reporter(stats, config);
}

// add both bloom filters
// save urls to file?
// increase bloom filters size
// test with delays
// decide via json what file type to gather, and everything else currently decided via constants
// interrupts should lead to clean exiting
