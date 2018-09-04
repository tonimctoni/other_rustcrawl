mod url_reservoir;
mod client;
mod worker;
mod stats;
mod murmur;
mod bloom_filter;

extern crate hyper;
extern crate tokio_core;
extern crate futures;
extern crate regex;
extern crate url;
use std::thread;
use std::time;
use std::sync;







const REPORT_PERIOD: time::Duration = time::Duration::from_secs(5);

fn main() {
    let stats=stats::Stats::new();

    let url_reservoir=url_reservoir::UrlReservoir::new(stats.clone());
    // url_reservoir.add_urls((0..20000).map(|_| url::Url::parse("http://c024:8080/").unwrap()).collect());
    url_reservoir.add_urls(vec![url::Url::parse("http://cssdb.co").unwrap()]);
    let url_reservoir_url_uri_stream=url_reservoir::UrlReservoirUrlUriStream::new(url_reservoir.clone());

    let (work_sender, work_receiver) = sync::mpsc::channel::<worker::Work>();

    let stats2=stats.clone();
    thread::spawn(move|| client::run_client(url_reservoir_url_uri_stream, work_sender, stats2));
    let url_reservoir2=url_reservoir.clone();
    let stats2=stats.clone();
    thread::spawn(move|| worker::run_worker(work_receiver, url_reservoir2, stats2));

    for i in 0..{
        thread::sleep(REPORT_PERIOD);
        println!("Report number {}:\n{}\n\n", i, stats);
    }
}

// add both bloom filters
// save urls to file?
// increase bloom filters size
// test with delays
// decide via json what file type to gather, and everything else currently decided via constants
// interrupts should lead to clean exiting


// extern crate hyper;
// extern crate futures;
// use hyper::rt::{Future, Stream};
// use std::str::FromStr;

// struct MyStream{}

// impl Stream for MyStream {
//     type Item=hyper::Uri;
//     type Error=();

//     fn poll(&mut self) -> Result<futures::Async<Option<Self::Item>>, Self::Error>{
//         Ok(futures::Async::Ready(Some(hyper::Uri::from_str("http://www.google.com/").unwrap())))
//     }
// }

// fn main() {
//     let client=hyper::Client::new();
//     let futs=MyStream{}
//         .map(move |uri|{
//             client
//             .get(uri)
//             .map_err(|_| ())
//             .and_then(|res|{
//                 let do_i_want_to_get_the_full_page=res
//                     .headers()
//                     .get("Content-Type")
//                     .map(|content_type| 
//                         content_type
//                             .to_str()
//                             .map(|ct| ct.contains("text/plain"))
//                             .unwrap_or(false)
//                     )
//                     .unwrap_or(false);

//                     // if do_i_want_to_get_the_full_page {
//                     //     futures::future::Either::A(res.into_body().concat2().map(|body| body.to_vec()).map_err(|_| ()))
//                     // } else {
//                     //     futures::future::Either::B(futures::future::err(()).map(|_:()| vec![]).map_err(|_| ()))
//                     // }
//                     let f = if do_i_want_to_get_the_full_page {
//                         futures::future::ok(())
//                     } else {
//                         futures::future::err(())
//                     };
//                     f.and_then (|_| res.into_body().concat2().map_err(|_| ()))
//             })
//             .map(|body|{
//                 println!("len is {}.", body.len());
//             })
//             .map_err(|e|{
//                 println!("Error: {:?}", e);
//             })
//         })
//         .buffer_unordered(2)
//         .for_each(|_|{
//             Ok(())
//         });

//     hyper::rt::run(futs);
// }




// let f = if do_i_want_to_get_the_full_page {
//     futures::future::ok(())
// } else {
//     futures::future::err(())
// };
// f.and_then (|_| res.into_body().concat2().map_err(|_| ()))


// error[E0271]: type mismatch resolving `<futures::AndThen<futures::FutureResult<(), ()>, futures::MapErr<futures::stream::Concat2<hyper::Body>, [closure@src/main.rs:93:71: 93:77]>, [closure@src/main.rs:93:33: 93:78 res:_]> as futures::IntoFuture>::Error == hyper::Error`
//   --> src/main.rs:76:14
//    |
// 76 |             .and_then(|res|{
//    |              ^^^^^^^^ expected (), found struct `hyper::Error`
//    |
//    = note: expected type `()`
//               found type `hyper::Error`

// error[E0599]: no method named `map` found for type `futures::AndThen<hyper::client::ResponseFuture, futures::AndThen<futures::FutureResult<(), ()>, futures::MapErr<futures::stream::Concat2<hyper::Body>, [closure@src/main.rs:93:71: 93:77]>, [closure@src/main.rs:93:33: 93:78 res:_]>, [closure@src/main.rs:76:23: 94:14]>` in the current scope
//   --> src/main.rs:95:14
//    |
// 95 |             .map(|body|{
//    |              ^^^
//    |
//    = note: the method `map` exists but the following trait bounds were not satisfied:
// ...


// if do_i_want_to_get_the_full_page {
//     Box::<Future<_, _>>::new (res.into_body().concat2().map_err(|_| ()))
// } else {
//     Box::<Future<_, _>>::new (futures::future::err(()))
// }


// error[E0107]: wrong number of type arguments: expected 0, found 2
//   --> src/main.rs:89:31
//    |
// 89 |                         Box::<Future<_, _>>::new (res.into_body().concat2().map_err(|_| ()))
//    |                               ^^^^^^^^^^^^ 2 unexpected type arguments

// error[E0191]: the value of the associated type `Error` (from the trait `futures::Future`) must be specified
//   --> src/main.rs:89:31
//    |
// 89 |                         Box::<Future<_, _>>::new (res.into_body().concat2().map_err(|_| ()))
//    |                               ^^^^^^^^^^^^ missing associated type `Error` value

// error[E0191]: the value of the associated type `Item` (from the trait `futures::Future`) must be specified
//   --> src/main.rs:89:31
//    |
// 89 |                         Box::<Future<_, _>>::new (res.into_body().concat2().map_err(|_| ()))
//    |                               ^^^^^^^^^^^^ missing associated type `Item` value
// ...


// if do_i_want_to_get_the_full_page {
//     res.into_body().concat2().map_err(|_| ())
// } else {
//     futures::future::err(()).map_err(|_| ())
// }

// error[E0308]: if and else have incompatible types
//   --> src/main.rs:88:21
//    |
// 88 | /                     if do_i_want_to_get_the_full_page {
// 89 | |                         res.into_body().concat2().map_err(|_| ())
// 90 | |                     } else {
// 91 | |                         futures::future::err(()).map_err(|_| ())
// 92 | |                     }
//    | |_____________________^ expected struct `futures::stream::Concat2`, found struct `futures::FutureResult`
//    |
//    = note: expected type `futures::MapErr<futures::stream::Concat2<hyper::Body>, [closure@src/main.rs:89:59: 89:65]>`
//               found type `futures::MapErr<futures::FutureResult<_, ()>, [closure@src/main.rs:91:58: 91:64]>`
