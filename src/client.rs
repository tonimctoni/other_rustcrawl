use url_reservoir;
use worker;
use stats;
use config;

use hyper;
use tokio_core;
use futures;
use hyper::rt::{Future, Stream};
use hyper_tls;

use std::thread;
use std::sync;
use std::time;

#[derive(Debug)]
enum RunClientError {
    HyperError(hyper::error::Error),
    Timeout,
    TimeoutError,
    TooLargeError,
    UnwantedFile,
}

impl From<hyper::error::Error> for RunClientError{
    fn from(error: hyper::error::Error) -> Self{
        RunClientError::HyperError(error)
    }
}


type ArcUrlReservoir = sync::Arc<url_reservoir::UrlReservoir>;
type WorkSender = sync::mpsc::Sender<worker::Work>;
type ArcStats = sync::Arc<stats::Stats>;
type ArcConfig = sync::Arc<config::Config>;

fn build_http_client(dns_threads: usize) -> sync::Arc<hyper::Client<hyper::client::HttpConnector>>{
    let connector=hyper::client::HttpConnector::new(dns_threads);
    let client=hyper::Client::builder()
        .keep_alive(false)
        .build::<_,hyper::Body>(connector);

    sync::Arc::new(client)
}

fn build_https_client(dns_threads: usize) -> sync::Arc<hyper::Client<hyper_tls::HttpsConnector<hyper::client::HttpConnector>>>{
    let connector=hyper_tls::HttpsConnector::new(dns_threads).unwrap();
    let client=hyper::Client::builder()
        .keep_alive(false)
        .build::<_,hyper::Body>(connector);

    sync::Arc::new(client)
}


pub fn spawn_clients(url_reservoir: ArcUrlReservoir, work_sender: WorkSender, stats: ArcStats, config: ArcConfig){
    if config.enable_https{
        let client=build_https_client(config.dns_threads);

        for _ in 0..(config.client_threads-1){
            let url_reservoir_stream=url_reservoir::UrlReservoirUrlUriStream::new(url_reservoir.clone());
            let work_sender2=work_sender.clone();
            let client2=client.clone();
            let stats2=stats.clone();
            let config2=config.clone();
            thread::spawn(move|| run_client(client2, url_reservoir_stream, work_sender2, stats2, config2));
        }

        let url_reservoir_stream=url_reservoir::UrlReservoirUrlUriStream::new(url_reservoir);
        thread::spawn(move|| run_client(client, url_reservoir_stream, work_sender, stats, config));
    } else{
        let client=build_http_client(config.dns_threads);

        for _ in 0..(config.client_threads-1){
            let url_reservoir_stream=url_reservoir::UrlReservoirUrlUriStream::new(url_reservoir.clone());
            let work_sender2=work_sender.clone();
            let client2=client.clone();
            let stats2=stats.clone();
            let config2=config.clone();
            thread::spawn(move|| run_client(client2, url_reservoir_stream, work_sender2, stats2, config2));
        }

        let url_reservoir_stream=url_reservoir::UrlReservoirUrlUriStream::new(url_reservoir);
        thread::spawn(move|| run_client(client, url_reservoir_stream, work_sender, stats, config));
    }

}


pub fn run_client<C>(client: sync::Arc<hyper::Client<C, hyper::Body>> ,urluri_stream: url_reservoir::UrlReservoirUrlUriStream, work_sender: WorkSender, stats: ArcStats, config: ArcConfig)
where C: hyper::client::connect::Connect + Sync + 'static,
      C::Transport: 'static,
      C::Future: 'static,

{
    let timeout_milliseconds=config.request_timeout;
    let max_body_len=config.max_body_len;
    let buffer_size=config.client_buffer_size;
    let desired_mimetypes: Vec<String>=config.files_to_gather.iter().map(|file_config| file_config.mimetype.clone()).collect();
    let desired_mimetypes=sync::Arc::new(desired_mimetypes);
    drop(config);

    let mut core = tokio_core::reactor::Core::new().unwrap();
    let handle = core.handle();

    let futs=urluri_stream
        .map(move |urluri|{
            let desired_mimetypes=desired_mimetypes.clone();
            let (url,uri)=urluri;
            client
            .get(uri)
            .from_err::<RunClientError>()
            .and_then(move |response|{
                enum HtmlFile {
                    Html,
                    File(usize),
                }

                // Get content type if it is a desired one, None otherwise
                let content_type={
                    response
                        .headers()
                        .get("Content-Type")
                        .map(move |ct| ct
                            .to_str()
                            .map(move |ct|
                                if ct.contains("text/html"){
                                    Some(HtmlFile::Html)
                                } else{
                                    desired_mimetypes
                                        .iter()
                                        .enumerate()
                                        .find(|(_iref, mimetyperef)| ct.contains(mimetyperef.as_str()))
                                        .map(|(i, _mimetype)| HtmlFile::File(i))
                                }
                            )
                            .unwrap_or(None)
                        )
                        .unwrap_or(None)
                };

                if content_type.is_some() {
                    futures::future::ok(())
                } else {
                    futures::future::err(RunClientError::UnwantedFile)
                }.and_then(move |_|{

                    // Concatenate all chunks of the file that is being received
                    response
                        .into_body()
                        .from_err::<RunClientError>()
                        .fold(Vec::new(), move |mut acc, chunk|{
                            if acc.len()+chunk.len()>max_body_len{
                                Err(RunClientError::TooLargeError)
                            } else{
                                acc.extend(&chunk[..]);
                                Ok(acc)
                            }
                        })
                })
                .map(move |body_vec|
                    match content_type {
                        Some(HtmlFile::Html) => worker::Work::Html(url, body_vec),
                        Some(HtmlFile::File(i)) => worker::Work::File(i, body_vec),
                        None => unreachable!(),
                    }
                )
            })
            .select2(tokio_core::reactor::Timeout::new(time::Duration::from_millis(timeout_milliseconds), &handle).unwrap())
            .then(|result_either|{

                // Prepare work if response to request is useful, otherwise prepare error; wrap in Ok in any case so it can get processed in the for_each
                match result_either {
                    Ok(either) => {
                        match either {
                            futures::future::Either::A((work,_timeout)) => Ok(Ok(work)),
                            futures::future::Either::B((_, _)) => Ok(Err(RunClientError::Timeout)),
                        }
                    },
                    Err(either) => {
                        match either {
                            futures::future::Either::A((e,_timeout)) => Ok(Err(RunClientError::from(e))),
                            futures::future::Either::B((_,_)) => Ok(Err(RunClientError::TimeoutError)),
                        }
                    },
                }
            })
        })
        .buffer_unordered(buffer_size)
        .for_each(move |result_work|{
            match result_work {

                // If the response yielded a useful file, send it to worker
                Ok(work) => {
                    stats.client_work_sent.fetch_add(1, sync::atomic::Ordering::Relaxed);
                    if let Err(e)=work_sender.send(work){
                        println!("Error: {:?}", e);
                    }
                },

                // On error inform stats for periodic report
                Err(e) => {
                    match e {
                        RunClientError::HyperError(_) => stats.client_hyper_error.fetch_add(1, sync::atomic::Ordering::Relaxed),
                        RunClientError::Timeout => stats.client_timeout.fetch_add(1, sync::atomic::Ordering::Relaxed),
                        RunClientError::TimeoutError => stats.client_timeout_error.fetch_add(1, sync::atomic::Ordering::Relaxed),
                        RunClientError::TooLargeError => stats.client_too_large_error.fetch_add(1, sync::atomic::Ordering::Relaxed),
                        RunClientError::UnwantedFile => stats.client_unwanted_file.fetch_add(1, sync::atomic::Ordering::Relaxed),
                    };
                },
            }
            Ok(())
        });

    let core_result=core.run(futs);
    println!("Tokio core result: {:?}", core_result);
    println!("Client rerminated.");
}