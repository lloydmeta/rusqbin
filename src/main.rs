extern crate rusqbin;
extern crate hyper;
extern crate rustc_serialize;
extern crate docopt;

use hyper::server::Listening;
use rusqbin::server::BinsServer;
use rusqbin::storage::InMemoryBins;

use docopt::Docopt;

const GREET: &'static str = r#"

**************************** Rusqbin ****************************

Send:
- POST    /rusqbins           To create a bin and get back bin_id
- GET     /rusqbins           To list bin summaries
- GET     /rusqbins/${bin_id} To get bin-specific information
- DELETE  /rusqbins/${bin_id} To delete a bin

In any other case, send requests with a X-Rusqbin-Id header with a
bin_id to have your requests logged to a bin for later retrieval.
"#;

const USAGE: &'static str = r#"

**************************** Rusqbin ****************************

A web server that stashes your requests for later retrieval.

Usage:
    cargo rusqbin [--port=<pn>]
    cargo rusqbin -h | --help
    cargo rusqbin --version

Options:
    -h, --help           Show this help message and exit.
    --version            Show the version.
    --port=<pn>          Run on a specific port [default: 9999]
"#;


#[derive(Debug, RustcDecodable)]
struct Args {
    flag_port: Option<usize>,
}

fn main() {
    let try_args: Result<Args, docopt::Error> = Docopt::new(USAGE).and_then(|d| d.version(Some(version())).decode());
    match try_args {
        Ok(Args { flag_port: Some(port) }) => start_on_port(port),
        Ok(Args { flag_port: None }) => {
            println!("\nUsing default port 9999");
            start_on_port(9999)
        },
        Err(e) => e.exit()
    }
}

/// Starts a BinsServer on the given port with an InMemory database.
fn start_on_port(p: usize) {
    let s = BinsServer::new(p, InMemoryBins::new());
    let r = s.start();
    let _l: Listening = r.unwrap();
    println!("{}", GREET);
    println!("\nServer started on {}", s.address)
}

fn version() -> String {
    let (maj, min, pat) = (option_env!("CARGO_PKG_VERSION_MAJOR"),
                           option_env!("CARGO_PKG_VERSION_MINOR"),
                           option_env!("CARGO_PKG_VERSION_PATCH"));
    match (maj, min, pat) {
        (Some(maj), Some(min), Some(pat)) => format!("{}.{}.{}", maj, min, pat),
        _ => "".to_owned(),
    }
}