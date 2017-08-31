//! Holds logic that lets us test our server synchronously
//! **WARNING** This is code to help test stuff..thar be dragons ahead

extern crate rusqbin;
extern crate hyper;
extern crate serde_json;
extern crate tokio_core;
extern crate futures;

use self::rusqbin::server::XRusqBinId;
use self::rusqbin::server::BinsServer;
use self::rusqbin::models::{BinSummary, Request, Id};
use self::rusqbin::storage::InMemoryBins;

use hyper::{Body, Method, StatusCode, Uri};
use hyper::client::{Response, FutureResponse};
use hyper::client::Request as HyperRequest;
use hyper::client::{Client, HttpConnector};
use hyper::header::Headers;
use hyper::header::ContentLength;

use std::error::Error;
use std::thread;
use std::time::Duration;
use std::str::FromStr;
use self::futures::{Future, future, Stream};
use std;

use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};

static PORT_NUM: AtomicUsize = ATOMIC_USIZE_INIT;

pub struct TestEnv {
    pub server: Arc<BinsServer<InMemoryBins>>,
    core: tokio_core::reactor::Core,
}

#[allow(dead_code)]
impl TestEnv {
    pub fn new(server: Arc<BinsServer<InMemoryBins>>) -> TestEnv {
        let core = tokio_core::reactor::Core::new().unwrap();
        TestEnv { server, core }
    }

    pub fn to_uri(&self, s: String) -> Uri {
        Uri::from_str(s.as_str()).unwrap()
    }

    pub fn with_client<F, Fut, I, E>(&mut self, f: F) -> I
    where
        F: FnOnce(&Client<HttpConnector, Body>) -> Fut,
        E: std::error::Error,
        Fut: Future<Item = I, Error = E>,
    {
        let fut = f(&self.client());
        self.core.run(fut).unwrap()
    }

    pub fn client(&self) -> Client<HttpConnector, Body> {
        Client::new(&self.core.handle())
    }

    pub fn base_uri(&self) -> String {
        format!("http://localhost:{}", self.server.port)
    }

    pub fn get_body(&mut self, res: Response) -> String {
        let f = read_to_string(res);
        self.core.run(f).unwrap()
    }

    pub fn create_bin(&mut self) -> Result<BinSummary, Box<Error>> {
        let path = format!("{}/rusqbins", self.base_uri());
        let uri = Uri::from_str(path.as_str())?;
        let req = HyperRequest::new(Method::Post, uri);
        let resp = self.with_client(|client| client.request(req));
        let string = self.get_body(resp);
        Ok(serde_json::from_str(&*string)?)
    }

    pub fn get_bin_summary(&mut self, bin_id: &Id) -> Result<BinSummary, Box<Error>> {
        let uri = Uri::from_str(&*format!("{}/rusqbins/{}", self.base_uri(), bin_id))?;
        let req = HyperRequest::new(Method::Get, uri);
        let resp = self.with_client(|client| client.request(req));
        let summary_string = self.get_body(resp);
        Ok(serde_json::from_str(&*summary_string)?)
    }

    pub fn delete_bin(&mut self, bin_id: &Id) -> Result<bool, Box<Error>> {
        let req = HyperRequest::new(
            Method::Delete,
            Uri::from_str(&*format!("{}/rusqbins/{}", self.base_uri(), bin_id))?,
        );
        let resp: Response = self.with_client(|c| c.request(req));
        Ok(resp.status() == StatusCode::Ok)
    }

    pub fn get_bin_requests(&mut self, bin_id: &Id) -> Result<Vec<Request>, Box<Error>> {
        let req = HyperRequest::new(
            Method::Get,
            Uri::from_str(&*format!(
                "{}/rusqbins/{}/requests",
                self.base_uri(),
                bin_id
            ))?,
        );
        let summary_resp: Response = self.with_client(|c| c.request(req));
        let summary_string = self.get_body(summary_resp);
        Ok(serde_json::from_str(&*summary_string)?)
    }

    // Fires sets of 3 requests in parallel
    pub fn parallel_requests(
        &mut self,
        bin_id: &Id,
        requests: &Vec<ServerRequest>,
        sets: usize,
    ) -> Vec<Response> {
        let futures: Vec<FutureResponse> = (0..sets)
            .flat_map(|_| {
                let f_reqs: Vec<FutureResponse> = requests
                    .iter()
                    .map(|ref r| {
                        let req_body = r.body.clone();
                        let req_method = r.method.clone();
                        let mut req_headers = r.headers.clone();
                        let bin_id_clone = bin_id.to_owned();
                        let path = format!("{}{}", self.base_uri(), r.path);

                        let mut request =
                            HyperRequest::new(req_method, Uri::from_str(path.as_str()).unwrap());
                        if let Some(body) = req_body {
                            request.set_body(body);
                        }

                        // Add the bin id to the list of headers
                        req_headers.set(XRusqBinId(bin_id_clone.value().to_owned()));
                        *request.headers_mut() = req_headers;
                        self.client().request(request)

                    })
                    .collect();
                f_reqs
            })
            .collect();

        let joined = future::join_all(futures);
        self.core.run(joined).unwrap()
    }
}

#[allow(dead_code)]
pub struct ServerRequest<'a> {
    pub method: Method,
    pub path: &'a str,
    pub body: Option<&'static str>,
    pub headers: Headers,
}

/// Integration tests: server is started and stopped and requests are made to
/// and from it to get end-to-end testing.
///
pub fn run_with_server<T>(test: T) -> ()
where
    T: FnOnce(TestEnv) -> (),
{
    let mut p: usize = PORT_NUM.fetch_add(1, Ordering::SeqCst);
    while p < 5000 {
        p = PORT_NUM.fetch_add(1, Ordering::SeqCst);
    }
    // set up
    let s = Arc::new(BinsServer::new(p, InMemoryBins::new()));
    let still_running = Arc::new(Mutex::new(true));
    let s_spawn = s.clone();
    let running_spawn = still_running.clone();
    thread::spawn(move || {
        s_spawn.run_until(future::poll_fn(|| if *running_spawn.lock().unwrap() {
            Ok(futures::Async::NotReady)
        } else {
            Ok(futures::Async::Ready(()))
        }))
    });
    thread::sleep(Duration::from_millis(500)); // wait a bit for the server to come up
    let test_env = TestEnv::new(s.clone());
    test(test_env);
    let mut running = still_running.lock().unwrap();
    *running = false;
}


#[derive(Debug)]
pub enum TestingError {
    FromUtf8Error,
    HyperError(hyper::Error),
}

/// Consumes the body and reads it into a String.
pub fn read_to_string(resp: Response) -> Box<Future<Item = String, Error = TestingError>> {
    Box::new(read_to_bytes(resp).and_then(|b| {
        String::from_utf8(b).map_err(|_| TestingError::FromUtf8Error)
    }))
}

/// Consumes a response, returning the body as a vector of bytes
pub fn read_to_bytes(resp: Response) -> Box<Future<Item = Vec<u8>, Error = TestingError>> {
    let vec = if let Some(len) = resp.headers().get::<ContentLength>() {
        Vec::with_capacity(**len as usize)
    } else {
        vec![]
    };
    Box::new(resp.body()
            // Body is a Stream (-.- " )which effectively uses Hyper::Error as well
            .map_err(|e| TestingError::HyperError(e))
            .fold(vec, |mut acc, chunk| {
                acc.extend_from_slice(chunk.as_ref());
                Ok::<_, TestingError>(acc)
            }))
}
