use actix_web::{client::ClientRequest, http::StatusCode, test::TestServer, HttpMessage};
use base64;
use chrono::offset::Utc;
use hawk::{self, Credentials, Key, RequestBuilder};
use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use lazy_static::lazy_static;
use serde::de::DeserializeOwned;
use serde_json;
use sha2::Sha256;

use super::*;
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

fn setup() -> TestServer {
    TestServer::with_factory(|| {
        let settings = Settings::with_env_and_config_file(&None).unwrap();
        let settings = Settings {
            debug: true,
            port: 8000,
            host: settings.host,
            database_url: settings.database_url,
            database_pool_max_size: Some(1),
            database_use_test_transactions: true,
            limits: ServerLimits::default(),
            master_secret: Secrets::default(),
        };

        let state = ServerState {
            db_pool: Box::new(MysqlDbPool::new(&settings).unwrap()),
            limits: Arc::clone(&SERVER_LIMITS),
            secrets: Arc::clone(&SECRETS),
            port: 8000,
        };
        build_app(state)
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
        .set_header(
            "Authorization",
            create_hawk_header(method.as_str(), server.addr().port(), path),
        )
        .finish()
        .unwrap()
}

fn create_hawk_header(method: &str, port: u16, path: &str) -> String {
    // TestServer hardcodes its hostname to localhost and binds to a random
    // port
    let host = "localhost";
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
            .set_header(
                "Authorization",
                create_hawk_header(method.as_str(), server.addr().port(), $path),
            ).json($body)
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
    test_endpoint(http::Method::GET, "/1.5/42/info/collections", "{}");
}

#[test]
fn collection_counts() {
    test_endpoint(http::Method::GET, "/1.5/42/info/collection_counts", "{}");
}

#[test]
fn collection_usage() {
    test_endpoint(http::Method::GET, "/1.5/42/info/collection_usage", "{}");
}

#[test]
fn configuration() {
    test_endpoint(
        http::Method::GET,
        "/1.5/42/info/configuration",
        &serde_json::to_string(&ServerLimits::default()).unwrap(),
    );
}

#[test]
fn quota() {
    test_endpoint(http::Method::GET, "/1.5/42/info/quota", "[0.0,null]");
}

#[test]
fn delete_all() {
    test_endpoint(http::Method::DELETE, "/1.5/42", "null");
    test_endpoint(http::Method::DELETE, "/1.5/42/storage", "null");
}

#[test]
fn delete_collection() {
    let start = SyncTimestamp::default();
    test_endpoint(http::Method::DELETE, "/1.5/42/storage/bookmarks", "0.0");
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
    test_endpoint_with_body! {
        POST "/1.5/42/storage/bookmarks", vec![params::PostCollectionBso {
            id: "foo".to_string(),
            sortindex: Some(0),
            payload: Some("bar".to_string()),
            ttl: Some(31536000),
        }],
        result: PostBsos {
            assert!(result.modified >= start);
            assert_eq!(result.success.len(), 1);
            assert_eq!(result.failed.len(), 0);
        }
    };
}

#[test]
fn delete_bso() {
    let mut server = setup();

    let request = create_request(
        &server,
        http::Method::DELETE,
        "/1.5/42/storage/bookmarks/wibble",
    );
    let response = server.execute(request.send()).unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[test]
fn get_bso() {
    let mut server = setup();

    let request = create_request(
        &server,
        http::Method::GET,
        "/1.5/42/storage/bookmarks/wibble",
    );
    let response = server.execute(request.send()).unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[test]
fn put_bso() {
    let start = SyncTimestamp::default();
    test_endpoint_with_body! {
        PUT "/1.5/42/storage/bookmarks/wibble", BsoBody {
            id: Some("wibble".to_string()),
            sortindex: Some(0),
            payload: Some("wibble".to_string()),
            ttl: Some(31536000),
        },
        result: PutBso {
            assert!(result >= start);
        }
    };
}

#[test]
fn invalid_content_type() {
    let mut server = setup();
    let request = server
        .client(http::Method::PUT, "/1.5/42/storage/bookmarks/wibble")
        .set_header(
            "Authorization",
            create_hawk_header(
                "PUT",
                server.addr().port(),
                "/1.5/42/storage/bookmarks/wibble",
            ),
        )
        .set_header("Content-Type", "application/javascript")
        .json(BsoBody {
            id: Some("wibble".to_string()),
            sortindex: Some(0),
            payload: Some("wibble".to_string()),
            ttl: Some(31536000),
        })
        .unwrap();

    let response = server.execute(request.send()).unwrap();
    assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);

    let request = server
        .client(http::Method::POST, "/1.5/42/storage/bookmarks")
        .set_header(
            "Authorization",
            create_hawk_header("POST", server.addr().port(), "/1.5/42/storage/bookmarks"),
        )
        .set_header("Content-Type", "application/javascript")
        .json(json!([BsoBody {
            id: Some("wibble".to_string()),
            sortindex: Some(0),
            payload: Some("wibble".to_string()),
            ttl: Some(31536000),
        }]))
        .unwrap();

    let response = server.execute(request.send()).unwrap();
    assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
}
