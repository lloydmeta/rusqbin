#[macro_use]
extern crate hyper;
extern crate rusqbin;
extern crate serde_json;

use self::rusqbin::models::BinSummary;

use hyper::client::Request as HyperRequest;
use hyper::StatusCode;
use hyper::header::{Headers, Header};
use hyper::Method;

use std::collections::HashMap;

use rusqbin::models::{Request, Id};

mod common;
use common::*;

/// Integration tests: server is started and stopped and requests are made to
/// and from it to get end-to-end testing. Request logging is checked using
/// the HTTP interface.
#[test]
fn test_start_and_stop() {
    run_with_server(|_| {});
}

#[test]
fn test_list_empty() {
    run_with_server(|mut test_env| {
        let req = HyperRequest::new(Method::Get,
                                    test_env.to_uri(format!("{}/rusqbins", test_env.base_uri())));
        let resp = test_env.with_client(|c| c.request(req));

        let string = test_env.get_body(resp);
        let decoded: HashMap<String, BinSummary> = serde_json::from_str(&*string).unwrap();

        assert_eq!(decoded.is_empty(), true);
    })
}

#[test]
fn test_getting_non_existent_bin() {
    run_with_server(|mut test_env| {
                        let uri = test_env.to_uri(format!("{}/rusqbins/5579fcd5-8353-4072-bb80-2d63a49c7ced",
                                                          test_env.base_uri()));
                        let req = HyperRequest::new(Method::Get, uri);
                        let resp = test_env.with_client(|c| c.request(req));

                        assert_eq!(resp.status(), StatusCode::NotFound);
                    })
}

#[test]
fn test_creating_bin() {
    run_with_server(|mut test_env| {
                        let bin = test_env.create_bin();
                        assert_eq!(bin.is_ok(), true);
                    })
}

#[test]
fn test_deleting_non_existing_bin() {
    run_with_server(|mut test_env| {
                        let deleted = test_env.delete_bin(&Id::random()).unwrap();
                        assert!(!deleted);
                    })
}

#[test]
fn test_deleting_existing_bin() {
    run_with_server(|mut test_env| {
                        let bin = test_env.create_bin().unwrap();
                        let deleted = test_env.delete_bin(&bin.id).unwrap();
                        assert!(deleted);
                    })
}

#[test]
fn test_requesting_bin_summary() {
    run_with_server(|mut test_env| {
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

        let bin_summary: BinSummary = test_env.get_bin_summary(&bin_id).unwrap();
        assert_eq!(bin_summary.request_count, requests.len() * 2);
    })
}


header! { (XFlubble, "X-Flubble") => [String] }
header! { (XDoodle, "X-Doodle") => [String] }

#[test]
fn test_requesting_bin_requests() {
    run_with_server(|mut test_env| {
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

        let requests: Vec<Request> = test_env.get_bin_requests(&bin_id).unwrap();
        assert_eq!(requests.len(), 1);

        let req: &Request = &requests[0];
        println!("{:?}", req.headers);
        assert_eq!(req.headers.len(), 5); // includes content-length, host, XRusqbinId, and the 2 additional ones we sent.

        assert!(req.headers
                    .get(<XFlubble as Header>::header_name())
                    .is_some());
        assert!(req.headers
                    .get(<XDoodle as Header>::header_name())
                    .is_some());

        assert_eq!(req.body, Some("hey there.".to_owned()));
        assert_eq!(req.method, Method::Post.as_ref());

    })
}
