//! Defines a BinsServer, which can serve requests against bins.
//!
//! BinsServer holds a database and wraps Hyper's Http server.
use std::sync::{Mutex, Arc};
use std::collections::HashMap;

use hyper;
use hyper::server::{Http, Request, Response, Service};
use hyper::header::ContentLength;
use hyper::header::Headers;
use hyper::{Get, Post, Delete};
use hyper::StatusCode;

use storage::*;
use models;
use models::{Id, IdExtractor};

use errors;
use errors::*;

use regex::Regex;

use serde::*;
use serde_json;

use time;

use url::Url;

use futures::{future, Future, Stream};

lazy_static! {
    static ref BIN_SUMMARY_PATH_REGEXP: Regex = {
        Regex::new(r"/rusqbins/((?i)[A-F0-9]{8}\-[A-F0-9]{4}\-4[A-F0-9]{3}\-[89AB][A-F0-9]{3}\-[A-F0-9]{12})$").unwrap()
    };
    static ref BIN_REQUESTS_PATH_REGEXP: Regex = {
        Regex::new(r"/rusqbins/((?i)[A-F0-9]{8}\-[A-F0-9]{4}\-4[A-F0-9]{3}\-[89AB][A-F0-9]{3}\-[A-F0-9]{12})/requests/?$").unwrap()
    };
}

/// Holds details about the current running server
pub struct BinsServer<T>
where
    T: Bins + Send,
{
    pub address: String,
    pub port: usize,
    pub storage: Arc<Mutex<T>>,
}

/// A Worker handles requests on the server and holds on to some
/// state, which enables it to do its work efficiently.
struct Worker<T>
where
    T: Bins,
{
    id_extractor: IdExtractor,
    bin_summary_path_regexp: Regex,
    bin_requests_path_regexp: Regex,
    bins: Arc<Mutex<T>>,
}

header! { (ContentType, "Content-Type") => [String] }
header! { (XRusqBinId, "X-Rusqbin-Id") => [String] }

impl<T> Service for Worker<T>
where
    T: Bins + Send + 'static,
{
    // boilerplate hooking up hyper's server types
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    // The future representing the eventual Response your call will
    // resolve to. This can change to whatever Future you need.
    type Future = Box<future::Future<Item = Self::Response, Error = Self::Error>>;

    // Req is a mutable reference because we need to mutate it for reading
    fn call(&self, req: Request) -> Self::Future {
        let result_future: Box<Future<Item = Response, Error = errors::Error>> = {
            let path = req.path().to_string();
            match (req.method(), &path[..]) {
                (&Get, path) if self.extract_id_from_bin_summary_path(path).is_some() => {
                    future_result(self.find_bin_summary(path))
                }
                (&Delete, path) if self.extract_id_from_bin_summary_path(path).is_some() => {
                    future_result(self.delete_bin(path))
                }
                (&Get, path) if self.extract_id_from_bin_requests_path(path).is_some() => {
                    future_result(self.find_bin_requests(path))
                }
                (&Get, "/rusqbins") |
                (&Get, "/rusqbins/") => future_result(self.list_bins()),
                (&Post, "/rusqbins") |
                (&Post, "/rusqbins/") => future_result(self.create_bin()),
                _ if self.extract_id_from_header(req.headers()).is_some() => {
                    Box::new((self.insert_request(req)))
                }
                _ => future_result(bad_request(Response::new())),
            }
        };
        Box::new(result_future.then(
            |handling_result| match handling_result {
                Err(Error::PoisonedLock) => panic!("Yo. Mutex got poisoned. Now wut?"),
                Err(e) => {
                    error!("Something really messed up bad: {:?}", e);
                    let mut res = Response::new();
                    res.set_status(StatusCode::InternalServerError);
                    future::ok(res)
                }
                Ok(rsp) => future::ok(rsp),
            },
        ))
    }
}

fn future_result(
    r: Result<Response, errors::Error>,
) -> Box<Future<Item = Response, Error = errors::Error>> {
    Box::new(future::result(r))
}

impl<T> Worker<T>
where
    T: Bins + 'static,
{
    // <-- Routing-related helper functions
    fn extract_id_from_bin_summary_path<'a>(&'a self, s: &'a str) -> Option<Id> {
        let caps = self.bin_summary_path_regexp.captures(&*s);
        caps.and_then(|c| {
            c.get(1).and_then(|r| self.id_extractor.parse(r.as_str()))
        })
    }

    fn extract_id_from_bin_requests_path<'a>(&'a self, s: &'a str) -> Option<Id> {
        let caps = self.bin_requests_path_regexp.captures(&*s);
        caps.and_then(|c| {
            c.get(1).and_then(|r| self.id_extractor.parse(r.as_str()))
        })
    }

    fn extract_id_from_header<'a>(&'a self, headers: &'a Headers) -> Option<Id> {
        headers.get::<XRusqBinId>().and_then(
            |s| self.id_extractor.parse(s),
        )
    }
    // Routing-related helper functions -->

    // <-- "Controller" methods.

    fn create_bin(&self) -> Result<Response, Error> {
        let res = Response::new();
        let mut cont = self.bins.lock()?;
        let new_bin = cont.create_bin();
        info!("Created a new bin {:?}", new_bin);
        write_json(&new_bin, res)
    }

    fn delete_bin(&self, path: &str) -> Result<Response, Error> {
        let res = Response::new();
        if let Some(id) = self.extract_id_from_bin_summary_path(path) {
            debug!("Trying to delete a bin with id: {}", id);
            let mut cont = self.bins.lock()?;
            match cont.delete_bin(&id) {
                DeleteBinStatus::Ok => {
                    info!("Deleted bin with id: {}", id);
                    ok(res)
                }
                DeleteBinStatus::NoSuchBin => {
                    info!("No bin with id: {}", id);
                    not_found(res)
                }
            }
        } else {
            // this methods should not be invoked if extraction isn't successful
            Err(Error::UnforeseenError)
        }
    }

    fn list_bins(&self) -> Result<Response, Error> {
        let res = Response::new();
        let cont = self.bins.lock()?;
        let all = &cont.get_bin_summaries();
        info!("Retrieved all bins: {:?}", all);
        write_json(all, res)
    }

    fn find_bin_summary(&self, path: &str) -> Result<Response, Error> {
        let res = Response::new();
        if let Some(id) = self.extract_id_from_bin_summary_path(path) {
            debug!("Trying to find a bin with id: {}", id);
            let cont = self.bins.lock()?;
            match cont.get_bin_summary(&id) {
                Some(ref bin) => {
                    info!("Retrieved bin summary: {:?}", bin);
                    write_json(bin, res)
                }
                None => {
                    info!("No bin with that id: {}", id);
                    not_found(res)
                }
            }
        } else {
            // this methods should not be invoked if extraction isn't successful
            Err(Error::UnforeseenError)
        }
    }

    fn find_bin_requests(&self, path: &str) -> Result<Response, Error> {
        let res = Response::new();
        if let Some(id) = self.extract_id_from_bin_requests_path(path) {
            debug!("Trying to find a bin with id: {} ", id);
            let cont = self.bins.lock()?;
            match cont.get_bin(&id) {
                Some(ref bin) => {
                    info!("Retrieved bin: {:?}", bin);
                    write_json(bin, res)
                }
                None => {
                    info!("No bin with that id: {}", id);
                    not_found(res)
                }
            }
        } else {
            // this methods should not be invoked if extraction isn't successful
            Err(Error::UnforeseenError)
        }
    }

    fn insert_request(&self, req: Request) -> Box<future::Future<Item = Response, Error = Error>> {
        if let Some(id) = self.extract_id_from_header(req.headers()) {
            let now = time::get_time();
            debug!("Insert time: {:?}", now);
            let now_millis = (now.sec as i64 * 1000) + (now.nsec as i64 / 1000 / 1000);
            debug!("Insert time in Epoch millis: {:?}", now_millis);
            let bins = self.bins.clone();
            let f = build_models_request(now_millis, req).and_then(move |req_model| {
                let inner_bins = bins.clone();
                let mut cont = inner_bins.lock()?;
                let res = Response::new();
                match cont.insert_request(&id, req_model) {
                    InsertRequestStatus::Ok => {
                        info!("Successfully inserted a request into bin with id: {}", id);
                        ok(res)
                    }
                    _ => {
                        info!("No bin with that id: {}", id);
                        not_found(res)
                    }
                }
            });
            Box::new(f)
        } else {
            // this methods should not be invoked if extraction isn't successful
            Box::new(future::err(Error::UnforeseenError))
        }
    }
}

fn write_json<T: Serialize>(t: &T, mut res: Response) -> Result<Response, Error> {
    let encoded: String = serde_json::ser::to_string_pretty(t)?;
    res.headers_mut().set(ContentLength(encoded.len() as u64));
    res.headers_mut().set(
        ContentType("application/json".to_owned()),
    );
    res.set_body(encoded);
    Ok(res)
}

fn not_found(mut res: Response) -> Result<Response, Error> {
    res.set_status(StatusCode::NotFound);
    Ok(res)
}

fn bad_request(mut res: Response) -> Result<Response, Error> {
    res.set_status(StatusCode::BadRequest);
    Ok(res)
}

fn ok(mut res: Response) -> Result<Response, Error> {
    res.set_status(StatusCode::Ok);
    Ok(res)
}

fn build_models_request(
    req_time: i64,
    req: Request,
) -> Box<Future<Item = models::Request, Error = errors::Error>> {
    let req_headers: Headers = req.headers().clone(); // to escape immutable req borrow..
    let content_length = req_headers.get::<ContentLength>().map(|l| l.0);
    let content_type = req_headers.get::<ContentType>().map(|t| t.0.clone());
    let method = req.method().to_string();
    let path = format!("{}", req.uri());

    let mut headers: HashMap<String, Vec<String>> = HashMap::new();
    for header in req_headers.iter() {
        let k = header.name();
        let v = header.value_string();
        headers.entry(k.to_owned()).or_insert(vec![]).push(v);
    }

    // our req is at this point guaranteed to be an AbsolutePath by the time it comes here.
    let parsed_url: Url = match Url::parse(&*format!("http://b.com{}", path)) {
        Ok(url) => url,
        Err(e) => return Box::new(future::err(errors::Error::from(e))),

    };
    let mut query_map: HashMap<String, Vec<String>> = HashMap::new();
    for (k, v) in parsed_url.query_pairs() {
        query_map.entry(k.into_owned()).or_insert(vec![]).push(
            v.into_owned(),
        );
    }

    let future_body: Box<Future<Item = Option<String>, Error = errors::Error>> =
        read_to_string(req);

    Box::new(future_body.map(move |body| {
        models::Request {
            content_length: content_length,
            content_type: content_type,
            time: req_time,
            method: method,
            path: path,
            body: body,
            headers: headers,
            query_string: query_map,
        }
    }))
}

impl<T> BinsServer<T>
where
    T: Bins + Send + 'static,
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
    pub fn run(&self) -> Result<(), errors::Error> {
        self.run_until(future::empty())
    }

    /// Starts a BinServer and stops when the given shutdown signal returns.
    pub fn run_until<F>(&self, shutdown_signal: F) -> Result<(), errors::Error>
    where
        F: future::Future<Item = (), Error = ()>,
    {
        let addr = self.address.parse()?;
        let storage = self.storage.clone();
        let server = Http::new().bind(&addr, move || {
            Ok(Worker {
                id_extractor: IdExtractor::new(),
                bin_summary_path_regexp: BIN_SUMMARY_PATH_REGEXP.clone(),
                bin_requests_path_regexp: BIN_REQUESTS_PATH_REGEXP.clone(),
                bins: storage.clone(),
            })
        })?;
        Ok(server.run_until(shutdown_signal)?)
    }
}

/// Consumes the body and reads it into a String.
fn read_to_string(req: Request) -> Box<Future<Item = Option<String>, Error = Error>> {
    Box::new(read_to_bytes(req).and_then(|b| {
        let s = String::from_utf8(b).map_err(|_| Error::FromUtf8Error);
        match s {
            Ok(ref s) if s.len() == 0 => Ok(None),
            Ok(s) => Ok(Some(s)),
            Err(e) => Err(e),
        }
    }))
}

/// Consumes a request, returning the body as a vector of bytes
fn read_to_bytes(req: Request) -> Box<Future<Item = Vec<u8>, Error = Error>> {
    let vec = if let Some(len) = req.headers().get::<ContentLength>() {
        Vec::with_capacity(**len as usize)
    } else {
        vec![]
    };
    Box::new(req.body()
            // Body is a Stream (-.- " )which effectively uses Hyper::Error as well
            .map_err(|_| Error::HyperError)
            .fold(vec, |mut acc, chunk| {
                acc.extend_from_slice(chunk.as_ref());
                Ok::<_, Error>(acc)
            }))
}
