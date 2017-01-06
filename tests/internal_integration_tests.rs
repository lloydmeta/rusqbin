#[macro_use]
extern crate hyper;
extern crate rusqbin;
extern crate rustc_serialize;

use self::rusqbin::models::BinSummary;

use hyper::header::{Headers, Header};
use hyper::method::Method;

use rusqbin::models::Request;
use rusqbin::storage::*;

#[cfg(test)]
mod common;
use common::*;

/// Internal tests: server is started and stopped and requests are made to
/// it, but internal state is checked from Rust using the bin server.

#[test]
fn test_list_empty() {
    run_with_server(|test_env| {
        let ref bins = test_env.server.storage.lock().unwrap().get_bin_summaries();
        assert!(bins.is_empty());
    })
}


#[test]
fn test_creating_bin() {
    run_with_server(|test_env| {
        let bin = test_env.create_bin().unwrap();
        let ref storage = test_env.server.storage.lock().unwrap();
        assert!(storage.get_bin_summary(&bin.id).is_some());
    })
}

#[test]
fn test_deleting_existing_bin() {
    run_with_server(|test_env| {
        let bin = test_env.create_bin().unwrap();
        let _deleted = test_env.delete_bin(&bin.id).unwrap();
        let ref storage = test_env.server.storage.lock().unwrap();
        assert!(storage.get_bin_summary(&bin.id).is_none());
    })
}

#[test]
fn test_requesting_bin_summary() {
    run_with_server(|test_env| {
        let new_bin = test_env.create_bin().unwrap();
        let bin_id = new_bin.id;

        let requests = vec![ServerRequest {
            method: Method::Get,
            headers: Headers::new(),
            path: "/",
            body: None,
        },
                            ServerRequest {
                                method: Method::Get,
                                headers: Headers::new(),
                                path: "/hello/world",
                                body: None,
                            },
                            ServerRequest {
                                method: Method::Post,
                                headers: Headers::new(),
                                path: "/boom/chicka/chicka",
                                body: Some("{ id: 3 }"),
                            }];
        test_env.parallel_requests(&bin_id, &requests, 2);

        let ref storage = test_env.server.storage.lock().unwrap();
        let bin_summary: BinSummary = storage.get_bin_summary(&bin_id).unwrap();
        assert_eq!(bin_summary.request_count, requests.len() * 2);
    })
}


header! { (XFlubble, "X-Flubble") => [String] }
header! { (XDoodle, "X-Doodle") => [String] }

#[test]
fn test_requesting_bin_requests() {
    run_with_server(|test_env| {
        let new_bin = test_env.create_bin().unwrap();
        let bin_id = new_bin.id;

        let mut headers = Headers::new();
        headers.set(XFlubble("yep".to_owned()));
        headers.set(XDoodle("nope".to_owned()));

        let requests = vec![ServerRequest {
            method: Method::Post,
            headers: headers,
            path: "/",
            body: Some("hey there."),
        }];
        test_env.parallel_requests(&bin_id, &requests, 1);

        let ref storage = test_env.server.storage.lock().unwrap();
        let requests: &Vec<Request> = storage.get_bin(&bin_id).unwrap();
        assert_eq!(requests.len(), 1);

        let req: &Request = &requests[0];
        println!("{:?}", req.headers);
        assert_eq!(req.headers.len(), 5); // includes content-length, host, XRusqbinId, and the 2 additional ones we sent.

        assert!(req.headers.get(<XFlubble as Header>::header_name()).is_some());
        assert!(req.headers.get(<XDoodle as Header>::header_name()).is_some());

        assert_eq!(req.body, Some("hey there.".to_owned()));
        assert_eq!(req.method, Method::Post.as_ref());

    })
}
