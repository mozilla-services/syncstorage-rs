// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, you can obtain one at https://mozilla.org/MPL/2.0/.

use actix_web::{client::ClientRequest, test::TestServer, HttpMessage};
use base64;
use chrono::offset::Utc;
use hawk::{Credentials, Key, RequestBuilder};
use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use ring;
use serde::de::DeserializeOwned;
use serde_json;
use sha2::Sha256;

use super::*;
use auth::HawkPayload;
use db::results::{GetBso, GetCollection, PostCollection, PutBso};
use handlers::{BsoBody, PostCollectionBody};

fn setup() -> TestServer {
    TestServer::build_with_state(move || ServerState {
        db: Box::new(MockDb::new()),
    }).start(|app| {
        init_routes!(app);
    })
}

fn test_endpoint(method: http::Method, path: &str, expected_body: &str) {
    let mut server = setup();

    let request = create_request(&server, method, path);

    let response = server.execute(request.send()).unwrap();
    assert!(response.status().is_success());

    let body = server.execute(response.body()).unwrap();
    assert_eq!(body, expected_body.as_bytes());
}

fn create_request(server: &TestServer, method: http::Method, path: &str) -> ClientRequest {
    server
        .client(method.clone(), path)
        .set_header("Authorization", create_hawk_header(method.as_str(), path))
        .finish()
        .unwrap()
}

fn create_hawk_header(method: &str, path: &str) -> String {
    let payload = HawkPayload {
        expires: (Utc::now().timestamp() + 5) as f64,
        node: "http://127.0.0.1:8000".to_string(),
        salt: "wibble".to_string(),
        uid: 42,
    };
    let payload = serde_json::to_string(&payload).unwrap();
    let signing_secret = hkdf_expand_32(
        b"services.mozilla.com/tokenlib/v1/signing",
        None,
        &[0u8; 32],
    );
    let mut signature: Hmac<Sha256> = Hmac::new_varkey(&signing_secret).unwrap();
    signature.input(payload.as_bytes());
    let signature = signature.result().code();
    let mut id: Vec<u8> = vec![];
    id.extend(payload.as_bytes());
    id.extend_from_slice(&signature);
    let id = base64::encode_config(&id, base64::URL_SAFE);
    let token_secret = hkdf_expand_32(
        format!("services.mozilla.com/tokenlib/v1/derive/{}", id).as_bytes(),
        Some(b"wibble"),
        &[0u8; 32],
    );
    let token_secret = base64::encode_config(&token_secret, base64::URL_SAFE);
    let request = RequestBuilder::new(method, "127.0.0.1", 8000, path).request();
    let credentials = Credentials {
        id,
        key: Key::new(token_secret.as_bytes(), &ring::digest::SHA256),
    };
    let header = request.make_header(&credentials).unwrap();
    format!("Hawk {}", header)
}

fn hkdf_expand_32(info: &[u8], salt: Option<&[u8]>, key: &[u8]) -> [u8; 32] {
    let mut result = [0u8; 32];
    let hkdf: Hkdf<Sha256> = Hkdf::extract(salt, key);
    hkdf.expand(info, &mut result).unwrap();
    result
}

fn test_endpoint_with_response<T>(method: http::Method, path: &str, assertions: &Fn(T) -> ())
where
    T: DeserializeOwned,
{
    let mut server = setup();

    let request = create_request(&server, method, path);

    let response = server.execute(request.send()).unwrap();
    assert!(response.status().is_success());

    let body = server.execute(response.body()).unwrap();
    let result: T = serde_json::from_slice(&*body).unwrap();
    assertions(result);
}

macro_rules! test_endpoint_with_body {
    ($method:ident $path:expr, $body:expr, $result:ident: $result_type:ty {$($assertion:expr;)+}) => {
        let mut server = setup();

        let method = http::Method::$method;
        let request = server
            .client(method.clone(), $path)
            .set_header("Authorization", create_hawk_header(method.as_str(), $path))
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
    test_endpoint(http::Method::GET, "/42/info/collections", "{}");
}

#[test]
fn collection_counts() {
    test_endpoint(http::Method::GET, "/42/info/collection_counts", "{}");
}

#[test]
fn collection_usage() {
    test_endpoint(http::Method::GET, "/42/info/collection_usage", "{}");
}

#[test]
fn configuration() {
    test_endpoint(http::Method::GET, "/42/info/configuration", "{}");
}

#[test]
fn quota() {
    test_endpoint(http::Method::GET, "/42/info/quota", "[]");
}

#[test]
fn delete_all() {
    test_endpoint(http::Method::DELETE, "/42", "null");
    test_endpoint(http::Method::DELETE, "/42/storage", "null");
}

#[test]
fn delete_collection() {
    test_endpoint(http::Method::DELETE, "/42/storage/bookmarks", "null");
    test_endpoint(http::Method::DELETE, "/42/storage/bookmarks?ids=1,", "null");
    test_endpoint(
        http::Method::DELETE,
        "/42/storage/bookmarks?ids=1,2,3",
        "null",
    );
}

#[test]
fn get_collection() {
    test_endpoint_with_response(
        http::Method::GET,
        "/42/storage/bookmarks",
        &move |collection: GetCollection| {
            assert_eq!(collection.len(), 0);
        },
    );
}

#[test]
fn post_collection() {
    test_endpoint_with_body! {
        POST "/42/storage/bookmarks", vec![PostCollectionBody {
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
    test_endpoint(http::Method::DELETE, "/42/storage/bookmarks/wibble", "null");
}

#[test]
fn get_bso() {
    test_endpoint_with_response(
        http::Method::GET,
        "/42/storage/bookmarks/wibble",
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
        PUT "/42/storage/bookmarks/wibble", BsoBody {
            sortindex: Some(0),
            payload: Some("wibble".to_string()),
            ttl: Some(31536000),
        },
        result: PutBso {
            assert_eq!(result, 0);
        }
    };
}
