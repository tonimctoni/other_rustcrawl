use url_reservoir;
use worker;
use stats;
use config;

use hyper;
use tokio_core;
use futures;
use hyper::rt::{Future, Stream};

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


pub fn run_client(urluri_stream: url_reservoir::UrlReservoirUrlUriStream, work_sender: sync::mpsc::Sender<worker::Work>, stats: sync::Arc<stats::Stats>, config: sync::Arc<config::Config>){
    let timeout_milliseconds=config.timeout;
    let max_body_len=config.max_body_len;
    let buffer_size=config.buffer_size;
    let desired_mimetypes: Vec<String>=config.files_to_gather.iter().map(|file_config| file_config.mimetype.clone()).collect();
    drop(config);

    let mut core = tokio_core::reactor::Core::new().unwrap();
    let handle = core.handle();

    // let client=hyper::Client::new();
    let client=hyper::Client::builder().keep_alive(false).build_http::<hyper::Body>();
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