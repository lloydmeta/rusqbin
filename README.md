# Rusqbin [![Build Status](https://travis-ci.org/lloydmeta/rusqbin.svg?branch=master)](https://travis-ci.org/lloydmeta/rusqbin) [![Crates.io](https://img.shields.io/crates/v/rusqbin.svg)](https://crates.io/crates/rusqbin)

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
  - `GET`     /rusqbins/${bin_id}          To get bin-specific information (lists all requests in the bin)
  - `GET`     /rusqbins/${bin_id}/requests To get detailed request information for a bin
  - `DELETE`  /rusqbins/${bin_id}          To delete a bin

In any other case, send requests with a X-Rusqbin-Id header with a
bin_id to have your requests logged to a bin for later retrieval.

### Binary

To use Rusqbin as a binary, simply install it using `cargo install rusqbin` and then run `rusqbin-server`,
and follow the simple usage instructions. The port the server runs on can be set by optionally adding a port argument.

![Binary usage demo](https://raw.githubusercontent.com/lloydmeta/rusqbin/master/demo.gif)

Logging is handled by [`env_logger`](https://github.com/rust-lang-nursery/log), so you can configure it at runtime using
a `RUST_LOG` environment variable.

### Library

To use it as a library, add it to your project as [a crate dependency](https://crates.io/crates/rusqbin), then from within Rust code:

```rust
extern crate rusqbin;
extern crate hyper;
extern crate rustc_serialize;

use rusqbin::storage::*;
use rusqbin::server::*;
use rusqbin::models::*;
use rustc_serialize::json;
use hyper::client::Client;
use std::io::Read;

// Start a BinsServer on port 9000
let s = BinsServer::new(7000, InMemoryBins::new());
let mut l = s.start().unwrap();

let client = Client::new();

// Create a bin via HTTP
let mut resp = client.post("http://localhost:7000/rusqbins").send().unwrap();
let mut string = String::new();
let _ = resp.read_to_string(&mut string).unwrap();
let bin: BinSummary = json::decode(&*string).unwrap();
let bin_id = bin.id.value();

// Fire an HTTP request with the proper X-Rusqbin-Id header
let _ = client.get("http://localhost:7000/hello").header(XRusqBinId(bin_id.to_owned())).send().unwrap();

// Access bin storage from within Rust code.
let ref storage = s.storage.lock().unwrap();
let bin_requests: &Bin = storage.get_bin(&bin.id).unwrap();
let ref req = bin_requests[0];

assert_eq!(req.method, "GET".to_owned());
assert_eq!(req.path, "/hello".to_owned());

l.close().unwrap();
```

In the example above, we use the out-of-the-box `InMemoryBins` for storage, but you can pass any given implementation of
`rusqbin::storage::Bins` when creating a BinsServer.

## Credit

Rusqbin is a simple port of [Requestbin](https://requestb.in/) written in Rust. Inspired by [Requestinator](https://github.com/DonMcNamara/requestinator)
