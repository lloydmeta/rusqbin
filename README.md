# Rusqbin 

Rusqbin is a web server that stashes your requests for later retrieval. It is available as
both a binary and a library.

To use it as a binary, simply install it using `cargo install rusqbins` and then `cargo rusqbins`,
and follow the simple usage instructions. The port the server runs on can be set using the `--port=port_num`.

To use it as a library from within Rust code:

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

fn main() {
    let s = BinsServer::new(7000, InMemoryBins::new());
    let mut l = s.start().unwrap();

    let client = Client::new();
    
    // Create a bin
    let mut resp = client.post("http://localhost:7000/rusqbins").send().unwrap();
    let mut string = String::new();
    let _ = resp.read_to_string(&mut string).unwrap();
    let bin: BinSummary = json::decode(&*string).unwrap();
    let bin_id = bin.id.value();
    
    // Fire a request
    let _ = client.get("http://localhost:7000/hello").header(XRusqBinId(bin_id.to_owned())).send().unwrap();
    
    let mut bin_requests_resp = client.get(&*format!("http://localhost:7000/rusqbins/{}/requests", bin_id)).send().unwrap();
    let mut requests_string = String::new();
    let _ = bin_requests_resp.read_to_string(&mut requests_string).unwrap();
    let bin_requests: Vec<Request> = json::decode(&*requests_string).unwrap();
    
    let ref req = bin_requests[0];
    
    assert_eq!(req.method, "GET".to_owned());
    assert_eq!(req.path, "/hello".to_owned());
    
    l.close().unwrap();
}
```

In the example above, we use the default `InMemoryBins` for storage, but you can pass any given implementation of
`rusqbin::storage::Bins` when creating a BinsServer.

[Requestbin](https://requestb.in/) written in Rust. Inspired by [Requestinator](https://github.com/DonMcNamara/requestinator)
