extern crate rusqbin;
extern crate hyper;
extern crate clap;
extern crate openssl_probe;

#[macro_use]
extern crate log;
extern crate env_logger;

use hyper::server::Listening;
use rusqbin::server::BinsServer;
use rusqbin::storage::InMemoryBins;
use clap::{Arg, App};

use std::error::Error;
use std::process::exit;

const DEFAULT_PORT: usize = 9999;
const DEFAULT_PORT_STR: &'static str = "9999";

const GREET: &'static str = r#"

**************************** Rusqbin ****************************

Send:
- POST    /rusqbins                    To create a bin and get back bin_id
- GET     /rusqbins                    To list bin summaries
- GET     /rusqbins/${bin_id}          To get bin-specific summary information
- GET     /rusqbins/${bin_id}/requests To get detailed request information for a bin
- DELETE  /rusqbins/${bin_id}          To delete a bin

In any other case, send requests with a X-Rusqbin-Id header with a
bin_id to have your requests logged to a bin for later retrieval.
"#;

fn main() {
    match inner_main() {
        Ok(_) => exit(0),
        Err(e) => {
            println!("Something went horribly wrong: {}", e);
            exit(1)
        }
    }
}

fn inner_main() -> Result<(), Box<Error>> {
    openssl_probe::init_ssl_cert_env_vars();
    env_logger::init()?;
    let matches = App::new("rusqbin")
        .version(&version()[..])
        .author("Lloyd (github.com/lloydmeta)")
        .about("requestb.in in Rust")
        .arg(
            Arg::with_name("port")
                .short("p")
                .default_value(DEFAULT_PORT_STR)
                .help("Sets the port for your sever")
                .required(false)
                .index(1),
        )
        .get_matches();

    match matches.value_of("port") {
        Some(port_str) => {
            let port: usize = port_str.parse().expect("Port must be number");
            Ok(start_on_port(port)?)
        }
        None => {
            info!("\nUsing default port {}", DEFAULT_PORT_STR);
            Ok(start_on_port(DEFAULT_PORT)?)
        }
    }
}

/// Starts a BinsServer on the given port with an InMemory database.
fn start_on_port(p: usize) -> Result<(), Box<Error>> {
    let s = BinsServer::new(p, InMemoryBins::new());
    let r = s.start();
    let _l: Listening = r?;
    println!("{}", GREET);
    Ok(println!("\nServer started on {}", s.address))
}

fn version() -> String {
    let (maj, min, pat) = (
        option_env!("CARGO_PKG_VERSION_MAJOR"),
        option_env!("CARGO_PKG_VERSION_MINOR"),
        option_env!("CARGO_PKG_VERSION_PATCH"),
    );
    match (maj, min, pat) {
        (Some(maj), Some(min), Some(pat)) => format!("{}.{}.{}", maj, min, pat),
        _ => "".to_owned(),
    }
}
