# Rusqbin [![Build Status](https://travis-ci.org/lloydmeta/rusqbin.svg?branch=master)](https://travis-ci.org/lloydmeta/rusqbin) [![Crates.io](https://img.shields.io/crates/v/rusqbin.svg)](https://crates.io/crates/rusqbin) [![](https://images.microbadger.com/badges/image/lloydmeta/rusqbin.svg)](https://microbadger.com/images/lloydmeta/rusqbin "rusqbin docker image details")

Rusqbin is a web server that stashes your requests for later retrieval. It is available as
both a binary and a library through [crates.io](https://crates.io/crates/rusqbin).

Rusdocs are published for:
* [Master branch](http://beachape.com/rusqbin)
* [Latest release](https://docs.rs/rusqbin)

## Usage

### Web API

The web server has the following API for dealing with request bins.

  - `POST`    /rusqbins                    To create a bin and get back bin_id
  - `GET`     /rusqbins                    To list bin summaries
  - `GET`     /rusqbins/${bin_id}          To get bin-specific summary information
  - `GET`     /rusqbins/${bin_id}/requests To get detailed request information for a bin (lists all requests in the bin)
  - `DELETE`  /rusqbins/${bin_id}          To delete a bin

In any other case, send requests with a X-Rusqbin-Id header with a
bin_id to have your requests logged to a bin for later retrieval.

### Docker

`$ docker run lloydmeta/rusqbin:latest`

### Binary

To use Rusqbin as a binary, simply install it using `cargo install rusqbin` and then run `rusqbin`,
and follow the simple usage instructions. The port the server runs on can be set by optionally adding a port argument.

![Binary usage demo](https://raw.githubusercontent.com/lloydmeta/rusqbin/master/rusqbin-demo.gif)

Logging is handled by [`env_logger`](https://github.com/rust-lang-nursery/log), so you can configure it at runtime using
a `RUST_LOG` environment variable.

### Library

To use it as a library, add it to your project as [a crate dependency](https://crates.io/crates/rusqbin), then from within Rust code:

```rust
use rusqbin::storage::*;
use rusqbin::server::*;
use rusqbin::models::*;
use hyper::{Method, Uri};
use hyper::client::Client;
use hyper::client::Request as HyperRequest;
use std::io::Read;
use std::thread;
use std::sync::{Arc, Mutex};
use std::str::FromStr;
use futures::future;

// Start a BinsServer on port 7000 in another thread, utilising
// a simple mutex for shutting it down. A channel could also work.
let server = Arc::new(BinsServer::new(7000, InMemoryBins::new()));
let arc_stay_alive = Arc::new(Mutex::new(true));
let bg_server = server.clone();
let bg_stay_alive = arc_stay_alive.clone();
thread::spawn(move || {
  bg_server.run_until(future::poll_fn(|| {
    if *bg_stay_alive.lock().unwrap() {
      Ok(futures::Async::NotReady)
    } else {
      Ok(futures::Async::Ready(()))
    }
  }))
});

let mut client_core = tokio_core::reactor::Core::new().unwrap();
let client = Client::new(&client_core.handle());

// Create a bin via programmatically, making sure to scope the
// storage unlocking with braces properly
let bin = {
  let mut server_storage = server.storage.lock().unwrap();
  server_storage.create_bin()
};
let bin_id = bin.id.value();

// Fire an HTTP request with the proper X-Rusqbin-Id header
let mut req = HyperRequest::new(Method::Post, Uri::from_str("http://localhost:7000/hello/world").unwrap());
req.headers_mut().set(XRusqBinId(bin_id.to_owned()));
let future_resp = client.request(req);

// Here we use core.run to block on response, but you should never
// do this in production code.
client_core.run(future_resp);
// Check to make sure our HTTP request was received and stashed
// in our rusqbin server
{
  let mut server_storage = server.storage.lock().unwrap();
  let bin_requests: &Bin = server_storage.get_bin(&bin.id).unwrap();
  let req = &bin_requests[0];
  assert_eq!(req.method, "POST".to_owned());
  assert_eq!(req.path, "/hello/world".to_owned());
}
// Cleanup by shutting down our server using the mutex
*arc_stay_alive.lock().unwrap() = false;
```

In the example above, we use the out-of-the-box `InMemoryBins` for storage, but you can pass any given implementation of
`rusqbin::storage::Bins` when creating a BinsServer.

## Credit

Rusqbin is a simple port of [Requestbin](https://requestb.in/) written in Rust. Inspired by [Requestinator](https://github.com/DonMcNamara/requestinator)
