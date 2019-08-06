use std::collections::HashMap;

use actix_service::Service;
use actix_web::{http, http::StatusCode, test};
use base64;
use bytes::Bytes;
use chrono::offset::Utc;
use hawk::{self, Credentials, Key, RequestBuilder};
use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use lazy_static::lazy_static;
use serde::de::DeserializeOwned;
use serde_json;
use sha2::Sha256;
use std::str::FromStr;

use super::*;
use crate::build_app;
use crate::db::mysql::pool::MysqlDbPool;
use crate::db::params;
use crate::db::results::{DeleteBso, GetBso, PostBsos, PutBso};
use crate::db::util::SyncTimestamp;
use crate::settings::{Secrets, ServerLimits};
use crate::web::auth::HawkPayload;
use crate::web::extractors::BsoBody;

lazy_static! {
    static ref SERVER_LIMITS: Arc<ServerLimits> = Arc::new(ServerLimits::default());
    static ref SECRETS: Arc<Secrets> = Arc::new(Secrets::new("foo"));
}

const TEST_HOST: &str = "localhost";
const TEST_PORT: u16 = 8080;

fn get_test_settings() -> Settings {
    let settings = Settings::with_env_and_config_file(&None).unwrap();
    let treq = test::TestRequest::with_uri("/").to_http_request();
    let port = treq
        .uri()
        .port_part()
        .map(|p| p.as_u16())
        .unwrap_or(TEST_PORT);
    // Make sure that our poolsize is >= the
    let host = treq.uri().host().unwrap_or(TEST_HOST).to_owned();
    let pool_size = u32::from_str(
        std::env::var_os("RUST_TEST_THREADS")
            .unwrap_or_else(|| std::ffi::OsString::from("10"))
            .into_string()
            .unwrap()
            .as_str(),
    )
    .unwrap();
    Settings {
        debug: true,
        port,
        host,
        database_url: settings.database_url.clone(),
        database_pool_max_size: Some(pool_size + 1),
        database_use_test_transactions: true,
        limits: ServerLimits::default(),
        master_secret: Secrets::default(),
    }
}

fn get_test_state(settings: &Settings) -> ServerState {
    ServerState {
        db_pool: Box::new(MysqlDbPool::new(&settings).unwrap()),
        limits: Arc::clone(&SERVER_LIMITS),
        secrets: Arc::clone(&SECRETS),
        port: settings.port,
    }
}

fn create_request(
    method: http::Method,
    path: &str,
    headers: Option<HashMap<&'static str, String>>,
    payload: Option<serde_json::Value>,
) -> actix_http::Request {
    let settings = get_test_settings();
    let mut req = test::TestRequest::with_uri(path)
        .method(method.clone())
        .header(
            "Authorization",
            create_hawk_header(method.as_str(), settings.port, path),
        );
    if let Some(body) = payload {
        req = req.set_json(&body);
    };
    if let Some(h) = headers {
        for (k, v) in h {
            let ln = String::from(k).to_lowercase();
            let hn = actix_http::http::HeaderName::from_lowercase(ln.as_bytes()).unwrap();
            let hv = actix_http::http::HeaderValue::from_str(v.as_str()).unwrap();
            req = req.header(hn, hv);
        }
    }
    req.to_request()
}

fn create_hawk_header(method: &str, port: u16, path: &str) -> String {
    // TestServer hardcodes its hostname to localhost and binds to a random
    // port
    let host = TEST_HOST;
    let payload = HawkPayload {
        expires: (Utc::now().timestamp() + 5) as f64,
        node: format!("http://{}:{}", host, port).to_string(),
        salt: "wibble".to_string(),
        user_id: 42,
    };
    let payload = serde_json::to_string(&payload).unwrap();
    let mut signature: Hmac<Sha256> = Hmac::new_varkey(&SECRETS.signing_secret).unwrap();
    signature.input(payload.as_bytes());
    let signature = signature.result().code();
    let mut id: Vec<u8> = vec![];
    id.extend(payload.as_bytes());
    id.extend_from_slice(&signature);
    let id = base64::encode_config(&id, base64::URL_SAFE);
    let token_secret = hkdf_expand_32(
        format!("services.mozilla.com/tokenlib/v1/derive/{}", id).as_bytes(),
        Some(b"wibble"),
        &SECRETS.master_secret,
    );
    let token_secret = base64::encode_config(&token_secret, base64::URL_SAFE);
    let request = RequestBuilder::new(method, host, port, path).request();
    let credentials = Credentials {
        id,
        key: Key::new(token_secret.as_bytes(), hawk::DigestAlgorithm::Sha256).unwrap(),
    };
    let header = request.make_header(&credentials).unwrap();
    format!("Hawk {}", header)
}

fn hkdf_expand_32(info: &[u8], salt: Option<&[u8]>, key: &[u8]) -> [u8; 32] {
    let mut result = [0u8; 32];
    let hkdf = Hkdf::<Sha256>::new(salt, key);
    hkdf.expand(info, &mut result).unwrap();
    result
}

fn test_endpoint(
    method: http::Method,
    path: &str,
    status: Option<StatusCode>,
    expected_body: Option<&str>,
) {
    let settings = get_test_settings();
    let limits = Arc::new(settings.limits.clone());
    let mut app = test::init_service(build_app!(get_test_state(&settings), limits));

    let req = create_request(method, path, None, None);
    let sresp = test::block_on(app.call(req)).unwrap();
    match status {
        None => assert!(sresp.response().status().is_success()),
        Some(status) => assert!(sresp.response().status() == status),
    };
    if let Some(x_body) = expected_body {
        let body = test::read_body(sresp);
        assert_eq!(body, x_body.as_bytes());
    }
}

fn test_endpoint_with_response<T>(method: http::Method, path: &str, assertions: &Fn(T) -> ())
where
    T: DeserializeOwned,
{
    let settings = get_test_settings();
    let limits = Arc::new(settings.limits.clone());
    let mut app = test::init_service(build_app!(get_test_state(&settings), limits));

    let req = create_request(method, path, None, None);
    let sresponse = match test::block_on(app.call(req)) {
        Ok(v) => v,
        Err(e) => {
            panic!("test_endpoint_with_response: Block failed: {:?}", e);
        }
    };
    if !sresponse.response().status().is_success() {
        dbg!(
            "⚠️ Warning: Returned error",
            sresponse.response().status(),
            sresponse.response()
        );
    }
    let body = test::read_body(sresponse);
    let result: T = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            panic!("test_endpoint_with_response: serde_json failed: {:?}", e);
        }
    };
    assertions(result);
}

fn test_endpoint_with_body(method: http::Method, path: &str, body: serde_json::Value) -> Bytes {
    let settings = get_test_settings();
    let limits = Arc::new(settings.limits.clone());
    let mut app = test::init_service(build_app!(get_test_state(&settings), limits));
    let req = create_request(method, path, None, Some(body));
    let sresponse = test::block_on(app.call(req)).unwrap();
    assert!(sresponse.response().status().is_success());
    test::read_body(sresponse)
}

#[test]
fn collections() {
    test_endpoint(
        http::Method::GET,
        "/1.5/42/info/collections",
        None,
        Some("{}"),
    );
}

#[test]
fn collection_counts() {
    test_endpoint(
        http::Method::GET,
        "/1.5/42/info/collection_counts",
        None,
        Some("{}"),
    );
}

#[test]
fn collection_usage() {
    test_endpoint(
        http::Method::GET,
        "/1.5/42/info/collection_usage",
        None,
        Some("{}"),
    );
}

#[test]
fn configuration() {
    test_endpoint(
        http::Method::GET,
        "/1.5/42/info/configuration",
        None,
        Some(&serde_json::to_string(&ServerLimits::default()).unwrap()),
    );
}

#[test]
fn quota() {
    test_endpoint(
        http::Method::GET,
        "/1.5/42/info/quota",
        None,
        Some("[0.0,null]"),
    );
}

#[test]
fn delete_all() {
    test_endpoint(http::Method::DELETE, "/1.5/42", None, Some("null"));
    test_endpoint(http::Method::DELETE, "/1.5/42/storage", None, Some("null"));
}

#[test]
fn delete_collection() {
    let start = SyncTimestamp::default();
    test_endpoint(
        http::Method::DELETE,
        "/1.5/42/storage/bookmarks",
        None,
        Some("0.00"),
    );
    test_endpoint_with_response(
        http::Method::DELETE,
        "/1.5/42/storage/bookmarks?ids=1,",
        &move |result: DeleteBso| {
            assert!(result > start);
        },
    );
    test_endpoint_with_response(
        http::Method::DELETE,
        "/1.5/42/storage/bookmarks?ids=1,2,3",
        &move |result: DeleteBso| {
            assert!(result > start);
        },
    );
}

#[test]
fn get_collection() {
    test_endpoint_with_response(
        http::Method::GET,
        "/1.5/42/storage/bookmarks",
        &move |collection: Vec<GetBso>| {
            assert_eq!(collection.len(), 0);
        },
    );
    test_endpoint_with_response(
        http::Method::GET,
        "/1.5/42/storage/nonexistent",
        &move |collection: Vec<GetBso>| {
            assert_eq!(collection.len(), 0);
        },
    );
}

#[test]
fn post_collection() {
    let start = SyncTimestamp::default();
    let res_body = json!([params::PostCollectionBso {
        id: "foo".to_string(),
        sortindex: Some(0),
        payload: Some("bar".to_string()),
        ttl: Some(31_536_000),
    }]);
    let bytes = test_endpoint_with_body(http::Method::POST, "/1.5/42/storage/bookmarks", res_body);
    let result: PostBsos = serde_json::from_slice(&bytes.to_vec()).unwrap();
    assert!(result.modified >= start);
    assert_eq!(result.success.len(), 1);
    assert_eq!(result.failed.len(), 0);
}

#[test]
fn delete_bso() {
    test_endpoint(
        http::Method::DELETE,
        "/1.5/42/storage/bookmarks/wibble",
        Some(StatusCode::NOT_FOUND),
        None,
    )
}

#[test]
fn get_bso() {
    test_endpoint(
        http::Method::GET,
        "/1.5/42/storage/bookmarks/wibble",
        Some(StatusCode::NOT_FOUND),
        None,
    )
}

#[test]
fn put_bso() {
    let start = SyncTimestamp::default();
    let bytes = test_endpoint_with_body(
        http::Method::PUT,
        "/1.5/42/storage/bookmarks/wibble",
        json!(BsoBody::default()),
    );
    let result: PutBso = serde_json::from_slice(&bytes).unwrap();
    assert!(result >= start);
}

#[test]
fn invalid_content_type() {
    let path = "/1.5/42/storage/bookmarks/wibble";
    let settings = get_test_settings();
    let limits = Arc::new(settings.limits.clone());
    let mut app = test::init_service(build_app!(get_test_state(&settings), limits));

    let mut headers = HashMap::new();
    headers.insert("Content-Type", "application/javascript".to_owned());
    let req = create_request(
        http::Method::PUT,
        path,
        Some(headers.clone()),
        Some(json!(BsoBody {
            id: Some("wibble".to_string()),
            sortindex: Some(0),
            payload: Some("wibble".to_string()),
            ttl: Some(31_536_000),
            ..Default::default()
        })),
    );

    let response = test::block_on(app.call(req)).unwrap();

    assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);

    let path = "/1.5/42/storage/bookmarks";

    let req = create_request(
        http::Method::POST,
        path,
        Some(headers.clone()),
        Some(json!([BsoBody {
            id: Some("wibble".to_string()),
            sortindex: Some(0),
            payload: Some("wibble".to_string()),
            ttl: Some(31_536_000),
            ..Default::default()
        }])),
    );

    let response = test::block_on(app.call(req)).unwrap();
    assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
}
