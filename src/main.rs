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
extern crate ctrlc;
use std::thread;
use std::sync;




fn main() {
    let config=config::Config::get_or_panic("config.json");
    let stats=stats::Stats::new();

    let url_reservoir=url_reservoir::UrlReservoir::new(stats.clone(), config.max_urls_in_reservoir);
    url_reservoir.add_urls(config.initial_urls.iter().map(|url| url::Url::parse(url).unwrap()).collect());

    let maybe_url_reservoir2=sync::Mutex::new(Some(url_reservoir.clone()));
    ctrlc::set_handler(move||{
        println!("Ctrl-C caught.");
        if let Ok(mut maybe_url_reservoir2)=maybe_url_reservoir2.lock(){
            if let Some(url_reservoir2)=maybe_url_reservoir2.take(){
                url_reservoir2.close()
            }
        }
    }).unwrap();

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


    let done_flag=sync::Arc::new(sync::atomic::AtomicBool::new(false));
    let stats2=stats.clone();
    let config2=config.clone();
    let done_flag2=done_flag.clone();
    thread::spawn(move|| worker::run_worker(work_receiver, url_reservoir, stats2, config2, done_flag2));


    reporter::run_reporter(stats, config, done_flag);
}
