use std::collections::HashMap;

use actix_web::{
    dev::Service,
    http::{self, HeaderName, HeaderValue, StatusCode},
    test,
};
use base64;
use bytes::Bytes;
use chrono::offset::Utc;
use futures::executor::block_on;
use hawk::{self, Credentials, Key, RequestBuilder};
use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use lazy_static::lazy_static;
use serde::de::DeserializeOwned;
use serde_json;
use serde_json::json;
use sha2::Sha256;
use std::str::FromStr;

use super::*;
use crate::build_app;
use crate::db::params;
use crate::db::pool_from_settings;
use crate::db::results::{DeleteBso, GetBso, PostBsos, PutBso};
use crate::db::util::SyncTimestamp;
use crate::settings::{Secrets, ServerLimits};
use crate::web::auth::HawkPayload;
use crate::web::extractors::BsoBody;

lazy_static! {
    static ref SERVER_LIMITS: Arc<ServerLimits> = Arc::new(ServerLimits::default());
    static ref SECRETS: Arc<Secrets> =
        Arc::new(Secrets::new("foo").expect("Could not get Secrets in server/test.rs"));
}

const TEST_HOST: &str = "localhost";
const TEST_PORT: u16 = 8080;

fn get_test_settings() -> Settings {
    let settings = Settings::with_env_and_config_file(&None)
        .expect("Could not get Settings in get_test_settings");
    let treq = test::TestRequest::with_uri("/").to_http_request();
    let port = treq.uri().port_u16().unwrap_or(TEST_PORT);
    // Make sure that our poolsize is >= the
    let host = treq.uri().host().unwrap_or(TEST_HOST).to_owned();
    let pool_size = u32::from_str(
        std::env::var_os("RUST_TEST_THREADS")
            .unwrap_or_else(|| std::ffi::OsString::from("10"))
            .into_string()
            .expect("Could not get RUST_TEST_THREADS in get_test_settings")
            .as_str(),
    )
    .expect("Could not get pool_size in get_test_settings");
    Settings {
        debug: true,
        port,
        host,
        database_url: settings.database_url,
        database_pool_max_size: Some(pool_size + 1),
        database_use_test_transactions: true,
        limits: ServerLimits::default(),
        master_secret: Secrets::default(),
        ..Default::default()
    }
}

fn get_test_state(settings: &Settings) -> ServerState {
    let metrics = Metrics::sink();
    ServerState {
        db_pool: pool_from_settings(&settings, &Metrics::from(&metrics))
            .expect("Could not get db_pool in get_test_state"),
        limits: Arc::clone(&SERVER_LIMITS),
        secrets: Arc::clone(&SECRETS),
        metrics: Box::new(metrics),
        port: settings.port,
    }
}

fn create_request(
    method: http::Method,
    path: &str,
    headers: Option<HashMap<&'static str, String>>,
    payload: Option<serde_json::Value>,
) -> test::TestRequest {
    let settings = get_test_settings();
    let mut req = test::TestRequest::with_uri(path)
        .method(method.clone())
        .header(
            "Authorization",
            create_hawk_header(method.as_str(), settings.port, path),
        )
        .header("Accept", "application/json")
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:72.0) Gecko/20100101 Firefox/72.0",
        );
    if let Some(body) = payload {
        req = req.set_json(&body);
    };
    if let Some(h) = headers {
        for (k, v) in h {
            let ln = String::from(k).to_lowercase();
            let hn = HeaderName::from_lowercase(ln.as_bytes())
                .expect("Could not get hn in create_request");
            let hv = HeaderValue::from_str(v.as_str()).expect("Could not get hv in create_request");
            req = req.header(hn, hv);
        }
    }
    req
}

fn create_hawk_header(method: &str, port: u16, path: &str) -> String {
    // TestServer hardcodes its hostname to localhost and binds to a random
    // port
    let host = TEST_HOST;
    let payload = HawkPayload {
        expires: (Utc::now().timestamp() + 5) as f64,
        node: format!("http://{}:{}", host, port),
        salt: "wibble".to_string(),
        user_id: 42,
        fxa_uid: "xxx_test".to_owned(),
        fxa_kid: "xxx_test".to_owned(),
        device_id: "xxx_test".to_owned(),
    };
    let payload =
        serde_json::to_string(&payload).expect("Could not get payload in create_hawk_header");
    let mut signature: Hmac<Sha256> = Hmac::new_varkey(&SECRETS.signing_secret)
        .expect("Could not get signature in create_hawk_header");
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
        key: Key::new(token_secret.as_bytes(), hawk::DigestAlgorithm::Sha256)
            .expect("Could not get key in create_hawk_header"),
    };
    let header = request
        .make_header(&credentials)
        .expect("Could not get header in create_hawk_header");
    format!("Hawk {}", header)
}

fn hkdf_expand_32(info: &[u8], salt: Option<&[u8]>, key: &[u8]) -> [u8; 32] {
    let mut result = [0u8; 32];
    let hkdf = Hkdf::<Sha256>::new(salt, key);
    hkdf.expand(info, &mut result)
        .expect("Could not hkdf.expand in hkdf_expand_32");
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
    let app = test::init_service(build_app!(get_test_state(&settings), limits));

    let req = create_request(method, path, None, None).to_request();
    let mut app = block_on(app);
    let sresp = block_on(app.call(req)).expect("Could not get sresp in test_endpoint");
    match status {
        None => assert!(sresp.response().status().is_success()),
        Some(status) => assert!(sresp.response().status() == status),
    };
    if let Some(x_body) = expected_body {
        let body = block_on(test::read_body(sresp));
        assert_eq!(body, x_body.as_bytes());
    }
}

fn test_endpoint_with_response<T>(method: http::Method, path: &str, assertions: &dyn Fn(T) -> ())
where
    T: DeserializeOwned,
{
    let settings = get_test_settings();
    let limits = Arc::new(settings.limits.clone());
    let app = test::init_service(build_app!(get_test_state(&settings), limits));

    let req = create_request(method, path, None, None).to_request();
    let mut app = block_on(app);
    let sresponse = match block_on(app.call(req)) {
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
    let body = block_on(test::read_body(sresponse));
    let result: T = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            panic!("test_endpoint_with_response: serde_json failed: {:?}", e);
        }
    };
    assertions(result)
}

fn test_endpoint_with_body(method: http::Method, path: &str, body: serde_json::Value) -> Bytes {
    let settings = get_test_settings();
    let limits = Arc::new(settings.limits.clone());
    let app = test::init_service(build_app!(get_test_state(&settings), limits));
    let req = create_request(method, path, None, Some(body)).to_request();
    let mut app = block_on(app);
    let sresponse =
        block_on(app.call(req)).expect("Could not get sresponse in test_endpoint_with_body");
    assert!(sresponse.response().status().is_success());
    block_on(test::read_body(sresponse))
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
        Some(
            &serde_json::to_string(&ServerLimits::default())
                .expect("Could not serde_json::to_string in test_endpoint"),
        ),
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
    test_endpoint_with_response(
        http::Method::DELETE,
        "/1.5/42/storage/bookmarks",
        &move |result: DeleteBso| {
            assert!(
                result == SyncTimestamp::from_seconds(0.00),
                format!("Bad Bookmarks {:?} != 0", result)
            );
        },
    );
    test_endpoint_with_response(
        http::Method::DELETE,
        "/1.5/42/storage/bookmarks?ids=1,",
        &move |result: DeleteBso| {
            assert!(
                result > start,
                format!("Bad Bookmarks ids {:?} < {:?}", result, start)
            );
        },
    );
    test_endpoint_with_response(
        http::Method::DELETE,
        "/1.5/42/storage/bookmarks?ids=1,2,3",
        &move |result: DeleteBso| {
            assert!(
                result > start,
                format!("Bad Bookmarks ids, m {:?} < {:?}", result, start)
            );
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
    let result: PostBsos =
        serde_json::from_slice(&bytes.to_vec()).expect("Could not get result in post_collection");
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
    let result: PutBso = serde_json::from_slice(&bytes).expect("Could not get result in put_bso");
    assert!(result >= start);
}

#[test]
fn bsos_can_have_a_collection_field() {
    let start = SyncTimestamp::default();
    // test that "collection" is accepted, even if ignored
    let bso1 = json!({"id": "global", "collection": "meta", "payload": "SomePayload"});
    let bsos = json!(
        [bso1,
         {"id": "2", "collection": "foo", "payload": "SomePayload"},
    ]);
    let bytes = test_endpoint_with_body(http::Method::POST, "/1.5/42/storage/meta", bsos);
    let result: PostBsos = serde_json::from_slice(&bytes.to_vec())
        .expect("Could not get result in bsos_can_have_a_collection_field");
    assert_eq!(result.success.len(), 2);
    assert_eq!(result.failed.len(), 0);

    let bytes = test_endpoint_with_body(http::Method::PUT, "/1.5/42/storage/meta/global", bso1);
    let result2: PutBso = serde_json::from_slice(&bytes)
        .expect("Could not get result2 in bsos_can_have_a_collection_field");
    assert!(result2 >= start);
}

#[test]
fn invalid_content_type() {
    let path = "/1.5/42/storage/bookmarks/wibble";
    let settings = get_test_settings();
    let limits = Arc::new(settings.limits.clone());
    let mut app = block_on(test::init_service(build_app!(
        get_test_state(&settings),
        limits
    )));

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
    )
    .to_request();

    let response = block_on(app.call(req)).expect("Could not get response in invalid_content_type");

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
    )
    .to_request();

    let response2 =
        block_on(app.call(req)).expect("Could not get response2 in invalid_content_type");
    assert_eq!(response2.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
}

#[test]
fn invalid_batch_post() {
    let settings = get_test_settings();
    let limits = Arc::new(settings.limits.clone());
    let app = test::init_service(build_app!(get_test_state(&settings), limits));

    let mut headers = HashMap::new();
    headers.insert("accept", "application/json".to_owned());
    let req = create_request(
        http::Method::POST,
        "/1.5/42/storage/tabs?batch=sammich",
        Some(headers),
        Some(json!([
            {"id": "123", "payload": "xxx", "sortindex": 23},
            {"id": "456", "payload": "xxxasdf", "sortindex": 23}
        ])),
    )
    .to_request();

    let mut app = block_on(app);
    let response = block_on(app.call(req)).expect("Could not get response in invalid_batch_post");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = String::from_utf8(block_on(test::read_body(response)).to_vec())
        .expect("Could not get body in invalid_batch_post");
    assert_eq!(body, "0");
}
