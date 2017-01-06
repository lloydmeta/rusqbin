extern crate rusqbin;
extern crate hyper;

use hyper::server::Listening;
use rusqbin::server::BinsServer;
use rusqbin::storage::InMemoryBins;

use std::env::args;
use std::num::ParseIntError;

const GREET: &'static str = "\n\
                              **************************** Rusqbin ****************************\n\n\
                              Send:\n\n\
                              - POST    /rusqbins           To create a bin and get back bin_id\n\
                              - GET     /rusqbins           To list bin summaries\n\
                              - GET     /rusqbins/${bin_id} To get bin-specific information\n\
                              - DELETE  /rusqbins/${bin_id} To delete a bin\n\n\
                              \
                              In any other case, send requests with a X-Rusqbin-Id header with a bin id\n\
                              to have your requests logged to a bin for later retrieval. ";
const USAGE: &'static str = "\nUsage: cargo rusqbin [port: optional, defaults to 9999]\n";

fn main() {
    // First argument is usually the path of the executable, so we use
    // the second one instead
    match args().skip(1).next() {
        Some(s) => {
            match parse_port(s) {
                Ok(p) => start_on_port(p),
                _ => println!("{}", USAGE),
            }
        }
        None => {
            println!("\nUsing default port 9999");
            start_on_port(9999)
        }
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

fn parse_port(s: String) -> Result<usize, CliError> {
    Ok(s.trim().parse::<usize>()?)
}

#[derive(Debug)]
enum CliError {
    Parse(ParseIntError),
}

impl From<ParseIntError> for CliError {
    fn from(e: ParseIntError) -> CliError {
        CliError::Parse(e)
    }
}
