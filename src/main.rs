mod url_reservoir;
mod client;
mod worker;
mod stats;
mod murmur;
mod bloom_filter;
mod reporter;
mod config;

extern crate hyper;
extern crate hyper_tls;
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
    // Load config
    let config=config::Config::get_or_panic("config.json");

    // Initialize stats
    let stats=stats::Stats::new();

    // Start reservoir
    let url_reservoir=url_reservoir::UrlReservoir::new(stats.clone(), config.max_urls_in_reservoir);
    url_reservoir.add_urls(config.initial_urls.iter().map(|url| url::Url::parse(url).unwrap()).collect());

    // Run Ctrl-C handler
    let maybe_url_reservoir2=sync::Mutex::new(Some(url_reservoir.clone()));
    ctrlc::set_handler(move||{
        println!("Ctrl-C caught.");
        if let Ok(mut maybe_url_reservoir2)=maybe_url_reservoir2.lock(){
            if let Some(url_reservoir2)=maybe_url_reservoir2.take(){
                url_reservoir2.close()
            }
        }
    }).unwrap();

    // Spawn clients
    let (work_sender, work_receiver) = sync::mpsc::channel::<worker::Work>();
    client::spawn_clients(url_reservoir.clone(), work_sender, stats.clone(), config.clone());

    // Run worker
    let done_flag=sync::Arc::new(sync::atomic::AtomicBool::new(false));
    let stats2=stats.clone();
    let config2=config.clone();
    let done_flag2=done_flag.clone();
    thread::spawn(move|| worker::run_worker(work_receiver, url_reservoir, stats2, config2, done_flag2));

    // Run reporter
    reporter::run_reporter(stats, config, done_flag);
}
