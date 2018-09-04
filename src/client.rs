use url_reservoir;
use worker;
use stats;

use hyper;
use tokio_core;
use futures;
use hyper::rt::{Future, Stream};

use std::sync;
use std::time;
// use std::io;


#[derive(Debug)]
enum RunClientError {
    HyperError(hyper::error::Error),
    // Timeout(tokio_core::reactor::Timeout),
    Timeout,
    TimeoutError,
    TooLargeError,
    // IOError(io::Error),
    UnwantedFile,
}

impl From<hyper::error::Error> for RunClientError{
    fn from(error: hyper::error::Error) -> Self{
        RunClientError::HyperError(error)
    }
}

// impl From<tokio_core::reactor::Timeout> for RunClientError{
//     fn from(error: tokio_core::reactor::Timeout) -> Self{
//         RunClientError::Timeout(error)
//     }
// }

// impl From<io::Error> for RunClientError{
//     fn from(error: io::Error) -> Self{
//         RunClientError::IOError(error)
//     }
// }


const TIMEOUT_MILLISECONDS: u64 = 2*1000;
const MAX_BODY_LENGTH: usize = 20*1024*1024;
const BUFFER_SIZE: usize = 10;
pub fn run_client(urluri_stream: url_reservoir::UrlReservoirUrlUriStream, work_sender: sync::mpsc::Sender<worker::Work>, stats: sync::Arc<stats::Stats>){
    let mut core = tokio_core::reactor::Core::new().unwrap();
    let handle = core.handle();

    // let client=hyper::Client::new();
    let client=hyper::Client::builder().keep_alive(false).build_http::<hyper::Body>();
    let futs=urluri_stream
        .map(move |urluri|{
            let (url,uri)=urluri;
            client
            .get(uri)
            .from_err::<RunClientError>()
            .and_then(|response|{
                enum HtmlPng {
                    Html,
                    Png,
                }

                let content_type={
                    response
                        .headers()
                        .get("Content-Type")
                        .map(|ct| ct
                            .to_str()
                            .map(|ct| if ct.contains("text/html") {Some(HtmlPng::Html)} else if ct.contains("image/png") {Some(HtmlPng::Png)} else {None})
                            .unwrap_or(None)
                        )
                        .unwrap_or(None)
                };

                if content_type.is_some() {
                    futures::future::ok(())
                } else {
                    futures::future::err(RunClientError::UnwantedFile)
                }.and_then(|_|{
                    response
                        .into_body()
                        .from_err::<RunClientError>()
                        .fold(Vec::new(), |mut acc, chunk|{
                            if acc.len()+chunk.len()>MAX_BODY_LENGTH{
                                Err(RunClientError::TooLargeError)
                            } else{
                                acc.extend(&chunk[..]);
                                Ok(acc)
                            }
                        })
                })
                .map(move |body_vec|
                    match content_type {
                        Some(HtmlPng::Html) => worker::Work::Html(url, body_vec),
                        Some(HtmlPng::Png) => worker::Work::Png(body_vec),
                        None => unreachable!(),
                    }
                )
            })
            .select2(tokio_core::reactor::Timeout::new(time::Duration::from_millis(TIMEOUT_MILLISECONDS), &handle).unwrap())
            .then(|result_either|{
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
                            futures::future::Either::B((_,_)) => Ok(Err(RunClientError::TimeoutError)), //(timeout_error, _get)
                        }
                    },
                }
            })
        })
        .buffer_unordered(BUFFER_SIZE)
        .for_each(move |result_work|{
            match result_work {
                Ok(work) => {
                    stats.client_work_sent.fetch_add(1, sync::atomic::Ordering::Relaxed);
                    if let Err(e)=work_sender.send(work){
                        println!("Error: {:?}", e);
                    }
                },
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

    // hyper::rt::run(futs);
    let core_result=core.run(futs);
    println!("Tokio core result: {:?}", core_result);
    println!("Client rerminated.");
}