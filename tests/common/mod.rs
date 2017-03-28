extern crate rusqbin;
extern crate hyper;
extern crate rustc_serialize;

use self::rusqbin::server::XRusqBinId;
use self::rusqbin::server::BinsServer;
use self::rusqbin::models::{BinSummary, Request, Id};
use self::rusqbin::storage::InMemoryBins;

use hyper::client::RequestBuilder;
use hyper::client::Response;
use hyper::client::Client;
use hyper::method::Method;
use hyper::header::Headers;
use hyper::status::StatusCode;
use hyper::error::Error as HyperError;
use rustc_serialize::json;

use std::io::Read;
use std::panic;
use std::error::Error;
use std::thread;

use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};

static PORT_NUM: AtomicUsize = ATOMIC_USIZE_INIT;

pub struct TestEnv {
    pub server: BinsServer<InMemoryBins>,
}

#[allow(dead_code)]
impl TestEnv {
    pub fn client(&self) -> Client {
        Client::new()
    }

    pub fn base_uri(&self) -> String {
        format!("http://localhost:{}", self.server.port)
    }

    pub fn create_bin(&self) -> Result<BinSummary, Box<Error>> {
        let path = format!("{}/rusqbins", self.base_uri());
        let mut resp: Response = self.client()
            .post(&*path)
            .send()?;
        let mut string = String::new();
        let _ = resp.read_to_string(&mut string)?;
        Ok(json::decode(&*string)?)
    }

    pub fn get_bin_summary(&self, bin_id: &Id) -> Result<BinSummary, Box<Error>> {
        let mut summary_resp: Response = self.client()
            .get(&*format!("{}/rusqbins/{}", self.base_uri(), bin_id))
            .send()?;
        let mut summary_string = String::new();
        let _ = summary_resp.read_to_string(&mut summary_string)?;
        Ok(json::decode(&*summary_string)?)
    }

    pub fn delete_bin(&self, bin_id: &Id) -> Result<bool, Box<Error>> {
        let resp: Response = self.client()
            .delete(&*format!("{}/rusqbins/{}", self.base_uri(), bin_id))
            .send()?;
        Ok(resp.status == StatusCode::Ok)
    }

    pub fn get_bin_requests(&self, bin_id: &Id) -> Result<Vec<Request>, Box<Error>> {
        let mut summary_resp: Response = self.client()
            .get(&*format!("{}/rusqbins/{}/requests", self.base_uri(), bin_id))
            .send()?;
        let mut summary_string = String::new();
        let _ = summary_resp.read_to_string(&mut summary_string)?;
        Ok(json::decode(&*summary_string)?)
    }

    // Fires sets of 3 requests in parallel
    pub fn parallel_requests(&self,
                             bin_id: &Id,
                             requests: &Vec<ServerRequest>,
                             sets: usize)
                             -> Arc<Mutex<Vec<Result<Response, HyperError>>>> {
        let mut threads = vec![];


        let req_results: Arc<Mutex<Vec<Result<Response, HyperError>>>> = Arc::new(Mutex::new(vec![]));
        for _ in 0..sets {
            for r in requests.iter() {
                // Avoid closing over the request, because headers is itself not thread safe
                let results = req_results.clone();
                let req_body = r.body.clone();
                let req_method = r.method.clone();
                let mut req_headers = r.headers.clone();
                let bin_id_clone = bin_id.to_owned();
                let path = format!("{}{}", self.base_uri(), r.path);

                let t = thread::spawn(move || {
                    let client = Client::new();
                    let mut req_to_send: RequestBuilder = match req_method {
                        Method::Get => client.get(&*path),
                        Method::Post => client.post(&*path),
                        Method::Delete => client.delete(&*path),
                        Method::Put => client.put(&*path),
                        Method::Patch => client.patch(&*path),
                        Method::Head => client.head(&*path),
                        _ => client.get(&*path),

                    };

                    req_headers.set(XRusqBinId(bin_id_clone.value().to_owned()));
                    req_to_send = req_to_send.headers(req_headers);

                    if let Some(body) = req_body {
                        req_to_send = req_to_send.body(body);
                    }
                    let result = req_to_send.send();
                    let mut rs = results.lock().unwrap();
                    rs.push(result);
                });
                threads.push(t);
            }
        }

        for t in threads {
            // Wait for the thread to finish. Returns a result.
            let _ = t.join();
        }

        req_results
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
    where T: FnOnce(TestEnv) -> () + panic::UnwindSafe
{
    let mut p: usize = PORT_NUM.fetch_add(1, Ordering::SeqCst);
    while p < 5000 {
        p = PORT_NUM.fetch_add(1, Ordering::SeqCst);
    }
    // set up
    let s = BinsServer::new(p, InMemoryBins::new());

    let mut l = s.start().unwrap();
    let test_env = TestEnv { server: s };

    let result = panic::catch_unwind(|| test(test_env));

    // tear down. Doesn't work yet but whatevs.
    l.close().unwrap();

    assert!(result.is_ok())
}
