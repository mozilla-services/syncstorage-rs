use std::sync::{Arc, Mutex, RwLock};

use actix_web::test::TestServer;
use actix_web::HttpMessage;
use serde_json;

use super::*;
use handlers::{BsoBody, PostCollectionBody};

fn setup() -> TestServer {
    TestServer::build_with_state(move || ServerState).start(|app| {
        init_routes!(app);
    })
}

fn test_endpoint(method: http::Method, path: &str, expected_body: &str) {
    let mut server = setup();

    let request = server.client(method, path).finish().unwrap();

    let response = server.execute(request.send()).unwrap();
    assert!(response.status().is_success());

    let body = server.execute(response.body()).unwrap();
    assert_eq!(body, expected_body.as_bytes());
}

macro_rules! test_endpoint_with_body {
    ($method:ident, $path:expr, $body:expr, $expected_body:expr) => {
        let mut server = setup();

        let request = server
            .client(http::Method::$method, $path)
            .json($body)
            .unwrap();

        let response = server.execute(request.send()).unwrap();
        assert!(response.status().is_success());

        let body = server.execute(response.body()).unwrap();
        assert_eq!(body, $expected_body.as_bytes());
    };
}

#[test]
fn collections() {
    test_endpoint(http::Method::GET, "deadbeef/info/collections", "null");
}

#[test]
fn collection_counts() {
    test_endpoint(http::Method::GET, "deadbeef/info/collection_counts", "null");
}

#[test]
fn collection_usage() {
    test_endpoint(http::Method::GET, "deadbeef/info/collection_usage", "null");
}

#[test]
fn configuration() {
    test_endpoint(http::Method::GET, "deadbeef/info/configuration", "null");
}

#[test]
fn quota() {
    test_endpoint(http::Method::GET, "deadbeef/info/quota", "null");
}

#[test]
fn delete_collection() {
    test_endpoint(http::Method::DELETE, "deadbeef/storage/bookmarks", "null");
    test_endpoint(
        http::Method::DELETE,
        "deadbeef/storage/bookmarks?ids=1,",
        "null",
    );
    test_endpoint(
        http::Method::DELETE,
        "deadbeef/storage/bookmarks?ids=1,2,3",
        "null",
    );
}

#[test]
fn get_collection() {
    test_endpoint(http::Method::GET, "deadbeef/storage/bookmarks", "null");
}

#[test]
fn post_collection() {
    test_endpoint_with_body!(
        POST,
        "deadbeef/storage/bookmarks",
        vec![PostCollectionBody {
            id: "foo".to_string(),
            sortindex: Some(0),
            payload: Some("bar".to_string()),
            ttl: Some(31536000000),
        }],
        "null"
    );
}

#[test]
fn delete_bso() {
    test_endpoint(
        http::Method::DELETE,
        "deadbeef/storage/bookmarks/wibble",
        "null",
    );
}

#[test]
fn get_bso() {
    test_endpoint(
        http::Method::GET,
        "deadbeef/storage/bookmarks/wibble",
        "null",
    );
}

#[test]
fn put_bso() {
    test_endpoint_with_body!(
        PUT,
        "deadbeef/storage/bookmarks/wibble",
        BsoBody {
            sortindex: Some(0),
            payload: Some("wibble".to_string()),
            ttl: Some(31536000000),
        },
        "null"
    );
}
