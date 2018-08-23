// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, you can obtain one at https://mozilla.org/MPL/2.0/.

use actix_web::test::TestServer;
use actix_web::HttpMessage;
use serde::de::DeserializeOwned;
use serde_json;

use super::*;
use db::results::{GetBso, GetCollection, PostCollection, PutBso};
use handlers::{BsoBody, PostCollectionBody};

fn setup() -> TestServer {
    TestServer::build_with_state(move || ServerState { db: MockDb::new() }).start(|app| {
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

fn test_endpoint_with_response<T>(method: http::Method, path: &str, assertions: &Fn(T) -> ())
where
    T: DeserializeOwned,
{
    let mut server = setup();

    let request = server.client(method, path).finish().unwrap();

    let response = server.execute(request.send()).unwrap();
    assert!(response.status().is_success());

    let body = server.execute(response.body()).unwrap();
    let result: T = serde_json::from_slice(&*body).unwrap();
    assertions(result);
}

macro_rules! test_endpoint_with_body {
    ($method:ident $path:expr, $body:expr, $result:ident: $result_type:ty {$($assertion:expr;)+}) => {
        let mut server = setup();

        let request = server
            .client(http::Method::$method, $path)
            .json($body)
            .unwrap();

        let response = server.execute(request.send()).unwrap();
        assert!(response.status().is_success());

        let body = server.execute(response.body()).unwrap();
        let $result: $result_type = serde_json::from_slice(&*body).unwrap();
        $($assertion;)+
    };
}

#[test]
fn collections() {
    test_endpoint(http::Method::GET, "deadbeef/info/collections", "{}");
}

#[test]
fn collection_counts() {
    test_endpoint(http::Method::GET, "deadbeef/info/collection_counts", "{}");
}

#[test]
fn collection_usage() {
    test_endpoint(http::Method::GET, "deadbeef/info/collection_usage", "{}");
}

#[test]
fn configuration() {
    test_endpoint(http::Method::GET, "deadbeef/info/configuration", "{}");
}

#[test]
fn quota() {
    test_endpoint(http::Method::GET, "deadbeef/info/quota", "[]");
}

#[test]
fn delete_all() {
    test_endpoint(http::Method::DELETE, "deadbeef", "null");
    test_endpoint(http::Method::DELETE, "deadbeef/storage", "null");
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
    test_endpoint_with_response(
        http::Method::GET,
        "deadbeef/storage/bookmarks",
        &move |collection: GetCollection| {
            assert_eq!(collection.len(), 0);
        },
    );
}

#[test]
fn post_collection() {
    test_endpoint_with_body! {
        POST "deadbeef/storage/bookmarks", vec![PostCollectionBody {
            id: "foo".to_string(),
            sortindex: Some(0),
            payload: Some("bar".to_string()),
            ttl: Some(31536000),
        }],
        result: PostCollection {
            assert_eq!(result.modified, 0);
            assert_eq!(result.success.len(), 0);
            assert_eq!(result.failed.len(), 0);
        }
    };
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
    test_endpoint_with_response(
        http::Method::GET,
        "deadbeef/storage/bookmarks/wibble",
        &move |bso: GetBso| {
            assert_eq!(bso.id, "");
            assert_eq!(bso.modified, 0);
            assert_eq!(bso.payload, "");
            assert!(bso.sortindex.is_none());
        },
    );
}

#[test]
fn put_bso() {
    test_endpoint_with_body! {
        PUT "deadbeef/storage/bookmarks/wibble", BsoBody {
            sortindex: Some(0),
            payload: Some("wibble".to_string()),
            ttl: Some(31536000),
        },
        result: PutBso {
            assert_eq!(result, 0);
        }
    };
}
