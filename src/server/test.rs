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
use db::mysql::pool::MysqlDbPool;
use db::params::PostCollectionBso;
use db::results::{GetBso, PostBsos, PutBso};
use db::util::ms_since_epoch;
use settings::{Secrets, ServerLimits};
use web::auth::HawkPayload;
use web::extractors::BsoBody;

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
        ).finish()
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
    test_endpoint(http::Method::GET, "/1.5/42/info/quota", "[0,null]");
}

#[test]
fn delete_all() {
    test_endpoint(http::Method::DELETE, "/1.5/42", "null");
    test_endpoint(http::Method::DELETE, "/1.5/42/storage", "null");
}

#[test]
fn delete_collection() {
    test_endpoint(http::Method::DELETE, "/1.5/42/storage/bookmarks", "0");
    test_endpoint(
        http::Method::DELETE,
        "/1.5/42/storage/bookmarks?ids=1,",
        "0",
    );
    test_endpoint(
        http::Method::DELETE,
        "/1.5/42/storage/bookmarks?ids=1,2,3",
        "0",
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
}

#[test]
fn post_collection() {
    let start = ms_since_epoch() as u64;
    test_endpoint_with_body! {
        POST "/1.5/42/storage/bookmarks", vec![PostCollectionBso {
            id: "foo".to_string(),
            sortindex: Some(0),
            payload: Some("bar".to_string()),
            ttl: Some(31536000),
        }],
        result: PostBsos {
            assert!(result.modified > start);
            assert_eq!(result.success.len(), 1);
            assert_eq!(result.failed.len(), 0);
        }
    };
}

#[test]
fn delete_bso() {
    #[derive(Debug, Default, Deserialize)]
    pub struct DeleteBso {
        modified: u64,
    }
    let start = ms_since_epoch() as u64;
    test_endpoint_with_response(
        http::Method::DELETE,
        "/1.5/42/storage/bookmarks/wibble",
        &move |dbso: DeleteBso| {
            assert!(dbso.modified > start);
        },
    );
}

#[test]
fn get_bso() {
    test_endpoint_with_response(
        http::Method::GET,
        "/1.5/42/storage/bookmarks/wibble",
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
    let start = ms_since_epoch() as u64;
    test_endpoint_with_body! {
        PUT "/1.5/42/storage/bookmarks/wibble", BsoBody {
            id: Some("wibble".to_string()),
            sortindex: Some(0),
            payload: Some("wibble".to_string()),
            ttl: Some(31536000),
        },
        result: PutBso {
            assert!(result > start);
        }
    };
}
