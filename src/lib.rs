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
//! # extern crate rustc_serialize;
//!
//! use rusqbin::storage::*;
//! use rusqbin::server::*;
//! use rusqbin::models::*;
//! use rustc_serialize::json;
//! use hyper::client::Client;
//! use std::io::Read;
//!
//! # fn main() {
//!
//! // Start a BinsServer on port 9000
//! let s = BinsServer::new(7000, InMemoryBins::new());
//! let mut l = s.start().unwrap();
//!
//! let client = Client::new();
//!
//! // Create a bin via HTTP
//! let mut resp = client.post("http://localhost:7000/rusqbins").send().unwrap();
//! let mut string = String::new();
//! let _ = resp.read_to_string(&mut string).unwrap();
//! let bin: BinSummary = json::decode(&*string).unwrap();
//! let bin_id = bin.id.value();
//!
//! // Fire an HTTP request with the proper X-Rusqbin-Id header
//! let _ = client.get("http://localhost:7000/hello").header(XRusqBinId(bin_id.to_owned())).send().unwrap();
//!
//! // Access bin storage from within Rust code.
//! let ref storage = s.storage.lock().unwrap();
//! let bin_requests: &Bin = storage.get_bin(&bin.id).unwrap();
//! let ref req = bin_requests[0];
//!
//! assert_eq!(req.method, "GET".to_owned());
//! assert_eq!(req.path, "/hello".to_owned());
//!
//! l.close().unwrap();
//! # }
//! ```
//!
//! In the example above, we use the default `InMemoryBins` for storage, but you can pass any given implementation of
//! `rusqbin::storage::Bins` when creating a BinsServer.
//!
//! [Requestbin](https://requestb.in/) written in Rust. Inspired by [Requestinator](https://github.com/DonMcNamara/requestinator)
#[macro_use]
extern crate hyper;
extern crate rustc_serialize;
extern crate uuid;
extern crate regex;
extern crate time;
extern crate url;

#[macro_use]
extern crate log;

pub mod models;
pub mod storage;
pub mod server;
pub mod errors;
