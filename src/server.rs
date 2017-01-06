//! Defines a BinsServer, which can serve requests against bins.
//!
//! BinsServer holds a database and wraps Hyper's Http server.
use std::sync::{Mutex, Arc};
use std::io::Write;
use std::collections::HashMap;
use std::io::Read;

use hyper::server::{Server, Handler, Request, Response};
use hyper::uri::RequestUri::AbsolutePath;
use hyper::header::ContentLength;
use hyper::header::Headers;
use hyper::server::Listening;
use hyper::{Get, Post, Delete};
use hyper::status::StatusCode;

use storage::*;
use models;
use models::{Id, IdExtractor};

use errors::*;

use regex::Regex;

use rustc_serialize::json;
use rustc_serialize::Encodable;

use time;

use url::Url;

const BIN_SUMMARY_PATH_REGEXP: &'static str =
    r"/rusqbins/((?i)[A-F0-9]{8}\-[A-F0-9]{4}\-4[A-F0-9]{3}\-[89AB][A-F0-9]{3}\-[A-F0-9]{12})$";
const BIN_REQUESTS_PATH_REGEXP: &'static str =
    r"/rusqbins/((?i)[A-F0-9]{8}\-[A-F0-9]{4}\-4[A-F0-9]{3}\-[89AB][A-F0-9]{3}\-[A-F0-9]{12})/requests/?$";

/// Holds details about the current running server
pub struct BinsServer<T>
    where T: Bins + Send + 'static
{
    pub address: String,
    pub port: usize,
    pub storage: Arc<Mutex<T>>,
}

/// A Worker handles requests on the server and holds on to some
/// state, which enables it to do its work efficiently.
struct Worker<T>
    where T: Bins
{
    id_extractor: IdExtractor,
    bin_summary_path_regexp: Regex,
    bin_requests_path_regexp: Regex,
    bins: Arc<Mutex<T>>,
}

header! { (ContentType, "Content-Type") => [String] }
header! { (XRusqBinId, "X-Rusqbin-Id") => [String] }

impl<T> Handler for Worker<T>
    where T: Bins + Send
{
    // Req is a mutable reference because we need to mutate it for reading
    fn handle(&self, mut req: Request, res: Response) {
        // From https://github.com/hyperium/hyper/blob/0.9.x/examples/server.rs
        let handling_result: Result<(), Error> = {
            let req_uri = req.uri.clone();
            match req_uri {
                AbsolutePath(ref path) => {
                    match (&req.method, &path[..]) {
                        (&Get, _) if self.extract_id_from_bin_summary_path(&path)
                            .is_some() => self.find_bin_summary(&path, res),
                        (&Delete, _) if self.extract_id_from_bin_summary_path(&path)
                            .is_some() => self.delete_bin(&path, res),
                        (&Get, _) if self.extract_id_from_bin_requests_path(&path)
                            .is_some() => self.find_bin_requests(&path, res),
                        (&Get, "/rusqbins") |
                        (&Get, "/rusqbins/") => self.list_bins(res),
                        (&Post, "/rusqbins") |
                        (&Post, "/rusqbins/") => self.create_bin(res),
                        _ if self.extract_id_from_header(&req.headers).is_some() => {
                            self.insert_request(&mut req, res)
                        }
                        _ => bad_request(res),
                    }
                }
                _ => bad_request(res),
            }
        };
        match handling_result {
            Err(Error::PoisonedLock) => panic!("Yo. Mutex got poisoned. Now wut?"),
            Err(e) => println!("Something really messed up bad: {:?}", e),
            _ => (),
        }
    }
}

impl<T> Worker<T>
    where T: Bins
{
    // <-- Routing-related helper functions
    fn extract_id_from_bin_summary_path<'a>(&'a self, s: &'a str) -> Option<Id> {
        let caps = self.bin_summary_path_regexp.captures(&*s);
        caps.and_then(|c| c.get(1).and_then(|r| self.id_extractor.parse(r.as_str())))
    }

    fn extract_id_from_bin_requests_path<'a>(&'a self, s: &'a str) -> Option<Id> {
        let caps = self.bin_requests_path_regexp.captures(&*s);
        caps.and_then(|c| c.get(1).and_then(|r| self.id_extractor.parse(r.as_str())))
    }

    fn extract_id_from_header<'a>(&'a self, headers: &'a Headers) -> Option<Id> {
        headers.get::<XRusqBinId>()
            .and_then(|s| self.id_extractor.parse(s))
    }
    // Routing-related helper functions -->

    // <-- "Controller" methods.

    fn create_bin(&self, res: Response) -> Result<(), Error> {
        let mut cont = self.bins.lock()?;
        let new_bin = cont.create_bin();
        write_json(&new_bin, res)
    }

    fn delete_bin(&self, path: &String, res: Response) -> Result<(), Error> {
        if let Some(id) = self.extract_id_from_bin_summary_path(path) {
            let mut cont = self.bins.lock()?;
            match cont.delete_bin(&id) {
                DeleteBinStatus::Ok => ok(res),
                DeleteBinStatus::NoSuchBin => not_found(res),
            }
        } else {
            // this methods should not be invoked if extraction isn't successful
            Err(Error::UnforeseenError)
        }
    }

    fn list_bins(&self, res: Response) -> Result<(), Error> {
        let cont = self.bins.lock()?;
        let all = &cont.get_bin_summaries();
        write_json(all, res)
    }

    fn find_bin_summary(&self, path: &String, res: Response) -> Result<(), Error> {
        if let Some(id) = self.extract_id_from_bin_summary_path(path) {
            let cont = self.bins.lock()?;
            match cont.get_bin_summary(&id) {
                Some(bin) => write_json(&bin, res),
                None => not_found(res),
            }
        } else {
            // this methods should not be invoked if extraction isn't successful
            Err(Error::UnforeseenError)
        }
    }

    fn find_bin_requests(&self, path: &String, res: Response) -> Result<(), Error> {
        if let Some(id) = self.extract_id_from_bin_requests_path(path) {
            let cont = self.bins.lock()?;
            match cont.get_bin(&id) {
                Some(bin) => write_json(bin, res),
                None => not_found(res),
            }
        } else {
            // this methods should not be invoked if extraction isn't successful
            Err(Error::UnforeseenError)
        }
    }

    fn insert_request(&self, req: &mut Request, res: Response) -> Result<(), Error> {
        let now = time::get_time();
        let now_millis = (now.sec as i64 * 1000) + (now.nsec as i64 / 1000 / 1000);
        if let Some(id) = self.extract_id_from_header(&req.headers.clone()) {
            let mut cont = self.bins.lock()?;
            match cont.insert_request(&id, build_models_request(now_millis, req)?) {
                InsertRequestStatus::Ok => ok(res),
                _ => not_found(res),
            }
        } else {
            // this methods should not be invoked if extraction isn't successful
            Err(Error::UnforeseenError)
        }
    }
}

fn write_json<T: Encodable>(t: &T, mut res: Response) -> Result<(), Error> {
    let pretty = json::as_pretty_json(t);
    let encoded = format!("{}", pretty);
    res.headers_mut().set(ContentLength(encoded.len() as u64));
    res.headers_mut().set(ContentType("application/json".to_owned()));
    let mut res_start = res.start()?;
    Ok(res_start.write_all(encoded.as_bytes())?)
}

fn not_found(mut res: Response) -> Result<(), Error> {
    *res.status_mut() = StatusCode::NotFound;
    Ok(())
}

fn bad_request(mut res: Response) -> Result<(), Error> {
    *res.status_mut() = StatusCode::BadRequest;
    Ok(())
}

fn ok(mut res: Response) -> Result<(), Error> {
    *res.status_mut() = StatusCode::Ok;
    Ok(())
}

fn build_models_request(req_time: i64, req: &mut Request) -> Result<models::Request, Error> {
    let req_headers: Headers = req.headers.clone(); // to escape immutable req borrow..
    let content_length = req_headers.get::<ContentLength>().map(|l| l.0);
    let content_type = req_headers.get::<ContentType>().map(|t| t.0.clone());
    let method = req.method.as_ref().to_owned();
    let path = format!("{}", req.uri);

    let body = match content_length {
        Some(l) if l > 0 => {
            let mut buffer = String::new();
            let _ = req.read_to_string(&mut buffer)?;
            Some(buffer)
        }
        _ => None,
    };

    let mut headers: HashMap<String, Vec<String>> = HashMap::new();
    for header in req_headers.iter() {
        let k = header.name();
        let v = header.value_string();
        headers.entry(k.to_owned()).or_insert(vec![]).push(v);
    }

    // our req is at this point guaranteed to be an AbsolutePath by the time it comes here.
    let parsed_url: Url = Url::parse(&*format!("http://b.com{}", path))?;
    let mut query_map: HashMap<String, Vec<String>> = HashMap::new();
    for (k, v) in parsed_url.query_pairs() {
        query_map.entry(k.into_owned()).or_insert(vec![]).push(v.into_owned());
    }

    Ok(models::Request {
        content_length: content_length,
        content_type: content_type,
        time: req_time,
        method: method,
        path: path,
        body: body,
        headers: headers,
        query_string: query_map,
    })
}

impl<T> BinsServer<T>
    where T: Bins + Send + 'static
{
    pub fn new(port: usize, bins: T) -> BinsServer<T> {
        let address = format!("127.0.0.1:{}", port);
        BinsServer {
            address: address,
            port: port,
            storage: Arc::new(Mutex::new(bins)),
        }
    }

    /// Starts a BinsServer.
    pub fn start(&self) -> Result<Listening, Error> {
        let bin_summary_path_regexp = Regex::new(BIN_SUMMARY_PATH_REGEXP)?;
        let bin_requests_path_regexp = Regex::new(BIN_REQUESTS_PATH_REGEXP)?;
        let worker = Worker {
            id_extractor: IdExtractor::new(),
            bin_summary_path_regexp: bin_summary_path_regexp,
            bin_requests_path_regexp: bin_requests_path_regexp,
            bins: self.storage.clone(),
        };
        let listening: Listening = Server::http(&*self.address)?.handle(worker)?;
        Ok(listening)
    }
}
