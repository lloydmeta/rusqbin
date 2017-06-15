#![doc(html_playground_url = "https://play.rust-lang.org/")]

//! Rusqbin is a web server that stashes your requests for later retrieval. It is available as
//! both a binary and a library.
//!
//! Rusqbin's web API is the following :
//!
//! - POST    /rusqbins                     To create a bin and get back bin_id
//! - GET     /rusqbins                     To list bin summaries
//! - GET     /rusqbins/${bin_id}/requests  To get detailed request information for a bin
//! - GET     /rusqbins/${bin_id}           To get bin-specific information (lists all requests in the bin)
//! - DELETE  /rusqbins/${bin_id}           To delete a bin
//!
//! In any other case, send requests with a X-Rusqbin-Id header with a bin_id to have your requests
//! logged to a bin for later retrieval.
//!
//! To use it as a binary, simply install it using `cargo install rusqbin` and then `rusqbin`,
//! and follow the simple usage instructions.
//!
//! To use it as a library from within Rust code:
//!
//! ```
//! # extern crate rusqbin;
//! # extern crate hyper;
//! # extern crate serde_json;
//! # extern crate futures;
//! # extern crate tokio_core;
//! use rusqbin::storage::*;
//! use rusqbin::server::*;
//! use rusqbin::models::*;
//! use hyper::{Method, Uri};
//! use hyper::client::Client;
//! use hyper::client::Request as HyperRequest;
//! use std::io::Read;
//! use std::thread;
//! use std::time::Duration;
//! use std::sync::{Arc, Mutex};
//! use std::str::FromStr;
//! use futures::future;
//! # fn main() {
//! // Start a BinsServer on port 7000 in another thread, utilising
//! // a simple mutex for shutting it down
//! let server = Arc::new(BinsServer::new(7000, InMemoryBins::new()));
//! let arc_stay_alive = Arc::new(Mutex::new(true));
//! let bg_server = server.clone();
//! let bg_stay_alive = arc_stay_alive.clone();
//! thread::spawn(move || {
//!   bg_server.run_until(future::poll_fn(|| {
//!     if *bg_stay_alive.lock().unwrap() {
//!       Ok(futures::Async::NotReady)
//!     } else {
//!       Ok(futures::Async::Ready(()))
//!     }
//!   }))
//! });
//! thread::sleep(Duration::from_millis(500)); // wait a bit for the server to come up
//!
//! let mut client_core = tokio_core::reactor::Core::new().unwrap();
//! let client = Client::new(&client_core.handle());
//!
//! // Create a bin via programmatically, making sure to scope the
//! // storage unlocking with braces properly
//! let bin = {
//!   let mut server_storage = server.storage.lock().unwrap();
//!   server_storage.create_bin()
//! };
//! let bin_id = bin.id.value();
//!
//! // Fire an HTTP request with the proper X-Rusqbin-Id header
//! let mut req = HyperRequest::new(Method::Post, Uri::from_str("http://localhost:7000/hello/world").unwrap());
//! req.headers_mut().set(XRusqBinId(bin_id.to_owned()));
//! let future_resp = client.request(req);
//!
//! // Here we use core.run to block on response, but you should never
//! // do this in production code.
//! client_core.run(future_resp);
//!
//! // Check to make sure our HTTP request was received and stashed
//! // in our rusqbin server
//! {
//!   let mut server_storage = server.storage.lock().unwrap();
//!   let bin_requests: &Bin = server_storage.get_bin(&bin.id).unwrap();
//!   let req = &bin_requests[0];
//!   assert_eq!(req.method, "POST".to_owned());
//!   assert_eq!(req.path, "/hello/world".to_owned());
//! }
//!
//! // Cleanup by shutting down our server
//! *arc_stay_alive.lock().unwrap() = false;
//! # }
//! ```
//!
//! In the example above, we use the default `InMemoryBins` for storage, but you can pass any given implementation of
//! `rusqbin::storage::Bins` when creating a BinsServer.
//!
//! [Requestbin](https://requestb.in/) written in Rust. Inspired by [Requestinator](https://github.com/DonMcNamara/requestinator)
#[macro_use]
extern crate hyper;
extern crate futures;
extern crate uuid;
extern crate regex;
extern crate time;
extern crate url;
#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate log;

pub mod models;
pub mod storage;
pub mod server;
pub mod errors;
