use std::collections::HashMap;
use std::str::FromStr;

use actix_web::{
    dev::Service,
    http::{
        self,
        header::{HeaderName, HeaderValue},
        StatusCode,
    },
    test,
    web::Bytes,
};
use base64::{engine, Engine};
use chrono::offset::Utc;
use hawk::{self, Credentials, Key, RequestBuilder};
use hmac::{Hmac, Mac};
use lazy_static::lazy_static;
use rand::{thread_rng, Rng};
use serde::de::DeserializeOwned;
use serde_json::json;
use sha2::Sha256;
use syncserver_common::{self, X_LAST_MODIFIED};
use syncserver_settings::{Secrets, Settings};
use syncstorage_db::{
    params,
    results::{DeleteBso, GetBso, PostBsos, PutBso},
    DbPoolImpl, SyncTimestamp,
};
use syncstorage_settings::ServerLimits;

use super::*;
use crate::build_app;
use crate::tokenserver;
use crate::web::{auth::HawkPayload, extractors::BsoBody};

lazy_static! {
    static ref SERVER_LIMITS: Arc<ServerLimits> = Arc::new(ServerLimits::default());
    static ref SECRETS: Arc<Secrets> =
        Arc::new(Secrets::new("foo").expect("Could not get Secrets in server/test.rs"));
    static ref RAND_UID: u32 = thread_rng().gen_range(0..10000);
}

const TEST_HOST: &str = "localhost";
const TEST_PORT: u16 = 8080;

/// NOTE: these tests run w/ test_settings() which enables
/// database_use_test_transactions (transactions don't commit), so data won't
/// persist to the db between requests. This can be overridden per test via
/// customizing the settings
fn get_test_settings() -> Settings {
    let mut settings = Settings::test_settings();
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
    if cfg!(feature = "mysql") && settings.syncstorage.uses_spanner() {
        panic!(
            "Spanner database_url specified for MySQL feature, please correct.\n\t{}",
            &settings.syncstorage.database_url
        )
    }
    if cfg!(feature = "spanner") && !&settings.syncstorage.uses_spanner() {
        panic!(
            "MySQL database_url specified for Spanner feature, please correct.\n\t{}",
            &settings.syncstorage.database_url
        )
    }
    settings.port = port;
    settings.host = host;
    settings.syncstorage.database_pool_max_size = pool_size + 1;
    settings
}

async fn get_test_state(settings: &Settings) -> ServerState {
    let metrics = Arc::new(Metrics::sink());
    let blocking_threadpool = Arc::new(BlockingThreadpool::default());

    ServerState {
        db_pool: Box::new(
            DbPoolImpl::new(
                &settings.syncstorage,
                &Metrics::from(&metrics),
                blocking_threadpool,
            )
            .expect("Could not get db_pool in get_test_state"),
        ),
        limits: Arc::clone(&SERVER_LIMITS),
        limits_json: serde_json::to_string(&**SERVER_LIMITS).unwrap(),
        metrics,
        port: settings.port,
        quota_enabled: settings.syncstorage.enable_quota,
        deadman: Arc::new(RwLock::new(Deadman::from(&settings.syncstorage))),
    }
}

/// Wrapper around an `actix_web::Service` used to run the tests.
///
/// It also stores the tracing subscriber guard to ensure that logging is setup while running the
/// test.
struct TestService<S> {
    service: S,
    _subscriber_guard: tracing::subscriber::DefaultGuard,
}

impl<S> TestService<S> {
    fn new(service: S) -> Self {
        Self {
            service,
            _subscriber_guard: crate::logging::init_test_logging(),
        }
    }
}

impl<S> std::ops::Deref for TestService<S> {
    type Target = S;

    fn deref(&self) -> &S {
        &self.service
    }
}

impl<S> std::ops::DerefMut for TestService<S> {
    fn deref_mut(&mut self) -> &mut S {
        &mut self.service
    }
}

macro_rules! init_app {
    () => {
        async {
            let settings = get_test_settings();
            init_app!(settings).await
        }
    };
    ($settings:expr) => {
        async {
            let limits = Arc::new($settings.syncstorage.limits.clone());
            let state = get_test_state(&$settings).await;
            TestService::new(
                test::init_service(build_app!(
                    state,
                    None::<tokenserver::ServerState>,
                    Arc::clone(&SECRETS),
                    limits,
                    build_cors(&$settings)
                ))
                .await,
            )
        }
    };
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
        .insert_header((
            "Authorization",
            create_hawk_header(method.as_str(), settings.port, path),
        ))
        .insert_header(("Accept", "application/json"))
        .insert_header((
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:72.0) Gecko/20100101 Firefox/72.0",
        ));
    if let Some(body) = payload {
        req = req.set_json(&body);
    };
    if let Some(h) = headers {
        for (k, v) in h {
            let ln = String::from(k).to_lowercase();
            let hn = HeaderName::from_lowercase(ln.as_bytes())
                .expect("Could not get hn in create_request");
            let hv = HeaderValue::from_str(v.as_str()).expect("Could not get hv in create_request");
            req = req.insert_header((hn, hv));
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
        fxa_uid: format!("xxx_test_uid_{}", *RAND_UID),
        fxa_kid: format!("xxx_test_kid_{}", *RAND_UID),
        device_id: "xxx_test".to_owned(),
        tokenserver_origin: Default::default(),
    };
    let payload =
        serde_json::to_string(&payload).expect("Could not get payload in create_hawk_header");
    let mut signature = Hmac::<Sha256>::new_from_slice(&SECRETS.signing_secret)
        .expect("Could not get signature in create_hawk_header");
    signature.update(payload.as_bytes());
    let signature = signature.finalize().into_bytes();
    let mut id: Vec<u8> = vec![];
    id.extend(payload.as_bytes());
    id.extend_from_slice(&signature);
    let id = engine::general_purpose::URL_SAFE.encode(&id);
    let token_secret = syncserver_common::hkdf_expand_32(
        format!("services.mozilla.com/tokenlib/v1/derive/{}", id).as_bytes(),
        Some(b"wibble"),
        &SECRETS.master_secret,
    )
    .expect("hkdf_expand_32 failed in create_hawk_header");
    let token_secret = engine::general_purpose::URL_SAFE.encode(token_secret);
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

async fn test_endpoint(
    method: http::Method,
    path: &str,
    status: Option<StatusCode>,
    expected_body: Option<&str>,
) {
    let app = init_app!().await;

    let req = create_request(method, path, None, None).to_request();
    let sresp = app
        .call(req)
        .await
        .expect("Could not get sresp in test_endpoint");
    match status {
        None => assert!(sresp.response().status().is_success()),
        Some(status) => assert!(sresp.response().status() == status),
    };
    if let Some(x_body) = expected_body {
        let body = test::read_body(sresp).await;
        assert_eq!(body, x_body.as_bytes());
    }
}

async fn test_endpoint_with_response<T>(method: http::Method, path: &str, assertions: &dyn Fn(T))
where
    T: DeserializeOwned,
{
    let settings = get_test_settings();
    let limits = Arc::new(settings.syncstorage.limits.clone());
    let state = get_test_state(&settings).await;
    let app = test::init_service(build_app!(
        state,
        None::<tokenserver::ServerState>,
        Arc::clone(&SECRETS),
        limits,
        build_cors(&settings)
    ))
    .await;

    let req = create_request(method, path, None, None).to_request();
    let sresponse = match app.call(req).await {
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
    let body = test::read_body(sresponse).await;
    let result: T = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            panic!("test_endpoint_with_response: serde_json failed: {:?}", e);
        }
    };
    assertions(result)
}

async fn test_endpoint_with_body(
    method: http::Method,
    path: &str,
    body: serde_json::Value,
) -> Bytes {
    let settings = get_test_settings();
    let limits = Arc::new(settings.syncstorage.limits.clone());
    let state = get_test_state(&settings).await;
    let app = test::init_service(build_app!(
        state,
        None::<tokenserver::ServerState>,
        Arc::clone(&SECRETS),
        limits,
        build_cors(&settings)
    ))
    .await;
    let req = create_request(method, path, None, Some(body)).to_request();
    let sresponse = app
        .call(req)
        .await
        .expect("Could not get sresponse in test_endpoint_with_body");
    assert!(sresponse.response().status().is_success());
    test::read_body(sresponse).await
}

#[actix_rt::test]
async fn collections() {
    test_endpoint(
        http::Method::GET,
        "/1.5/42/info/collections",
        None,
        Some("{}"),
    )
    .await;
}

#[actix_rt::test]
async fn collection_counts() {
    test_endpoint(
        http::Method::GET,
        "/1.5/42/info/collection_counts",
        None,
        Some("{}"),
    )
    .await;
}

#[actix_rt::test]
async fn collection_usage() {
    test_endpoint(
        http::Method::GET,
        "/1.5/42/info/collection_usage",
        None,
        Some("{}"),
    )
    .await;
}

#[actix_rt::test]
async fn configuration() {
    test_endpoint(
        http::Method::GET,
        "/1.5/42/info/configuration",
        None,
        Some(
            &serde_json::to_string(&ServerLimits::default())
                .expect("Could not serde_json::to_string in test_endpoint"),
        ),
    )
    .await;
}

#[actix_rt::test]
async fn quota() {
    test_endpoint(
        http::Method::GET,
        "/1.5/42/info/quota",
        None,
        Some("[0.0,null]"),
    )
    .await;
}

#[actix_rt::test]
async fn delete_all() {
    test_endpoint(http::Method::DELETE, "/1.5/42", None, Some("null")).await;
    test_endpoint(http::Method::DELETE, "/1.5/42/storage", None, Some("null")).await;
}

#[actix_rt::test]
async fn delete_collection() {
    let start = SyncTimestamp::default();
    test_endpoint_with_response(
        http::Method::DELETE,
        "/1.5/42/storage/bookmarks",
        &move |result: DeleteBso| {
            assert!(
                result == SyncTimestamp::from_seconds(0.00),
                "Bad Bookmarks {:?} != 0",
                result
            );
        },
    )
    .await;
    test_endpoint_with_response(
        http::Method::DELETE,
        "/1.5/42/storage/bookmarks?ids=1,",
        &move |result: DeleteBso| {
            assert!(
                result > start,
                "Bad Bookmarks ids {:?} < {:?}",
                result,
                start
            );
        },
    )
    .await;
    test_endpoint_with_response(
        http::Method::DELETE,
        "/1.5/42/storage/bookmarks?ids=1,2,3",
        &move |result: DeleteBso| {
            assert!(
                result > start,
                "Bad Bookmarks ids, m {:?} < {:?}",
                result,
                start
            );
        },
    )
    .await;
}

#[actix_rt::test]
async fn get_collection() {
    test_endpoint_with_response(
        http::Method::GET,
        "/1.5/42/storage/bookmarks",
        &move |collection: Vec<GetBso>| {
            assert_eq!(collection.len(), 0);
        },
    )
    .await;
    test_endpoint_with_response(
        http::Method::GET,
        "/1.5/42/storage/nonexistent",
        &move |collection: Vec<GetBso>| {
            assert_eq!(collection.len(), 0);
        },
    )
    .await;
}

#[actix_rt::test]
async fn post_collection() {
    let start = SyncTimestamp::default();
    let res_body = json!([params::PostCollectionBso {
        id: "foo".to_string(),
        sortindex: Some(0),
        payload: Some("bar".to_string()),
        ttl: Some(31_536_000),
    }]);
    let bytes =
        test_endpoint_with_body(http::Method::POST, "/1.5/42/storage/bookmarks", res_body).await;
    let result: PostBsos =
        serde_json::from_slice(&bytes).expect("Could not get result in post_collection");
    assert!(result.modified >= start);
    assert_eq!(result.success.len(), 1);
    assert_eq!(result.failed.len(), 0);
}

#[actix_rt::test]
async fn delete_bso() {
    test_endpoint(
        http::Method::DELETE,
        "/1.5/42/storage/bookmarks/wibble",
        Some(StatusCode::NOT_FOUND),
        None,
    )
    .await;
}

#[actix_rt::test]
async fn get_bso() {
    test_endpoint(
        http::Method::GET,
        "/1.5/42/storage/bookmarks/wibble",
        Some(StatusCode::NOT_FOUND),
        None,
    )
    .await;
}

#[actix_rt::test]
async fn put_bso() {
    let start = SyncTimestamp::default();
    let bytes = test_endpoint_with_body(
        http::Method::PUT,
        "/1.5/42/storage/bookmarks/wibble",
        json!(BsoBody::default()),
    )
    .await;
    let result: PutBso = serde_json::from_slice(&bytes).expect("Could not get result in put_bso");
    assert!(result >= start);
}

#[actix_rt::test]
async fn bsos_can_have_a_collection_field() {
    let start = SyncTimestamp::default();
    // test that "collection" is accepted, even if ignored
    let bso1 = json!({"id": "global", "collection": "meta", "payload": "SomePayload"});
    let bsos = json!(
        [bso1,
         {"id": "2", "collection": "foo", "payload": "SomePayload"},
    ]);
    let bytes = test_endpoint_with_body(http::Method::POST, "/1.5/42/storage/meta", bsos).await;
    let result: PostBsos = serde_json::from_slice(&bytes)
        .expect("Could not get result in bsos_can_have_a_collection_field");
    assert_eq!(result.success.len(), 2);
    assert_eq!(result.failed.len(), 0);

    let bytes =
        test_endpoint_with_body(http::Method::PUT, "/1.5/42/storage/meta/global", bso1).await;
    let result2: PutBso = serde_json::from_slice(&bytes)
        .expect("Could not get result2 in bsos_can_have_a_collection_field");
    assert!(result2 >= start);
}

#[actix_rt::test]
async fn invalid_content_type() {
    let path = "/1.5/42/storage/bookmarks/wibble";
    let app = init_app!().await;

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

    let response = app
        .call(req)
        .await
        .expect("Could not get response in invalid_content_type");

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

    let response2 = app
        .call(req)
        .await
        .expect("Could not get response2 in invalid_content_type");
    assert_eq!(response2.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
}

#[actix_rt::test]
async fn invalid_batch_post() {
    let app = init_app!().await;

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

    let response = app
        .call(req)
        .await
        .expect("Could not get response in invalid_batch_post");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = String::from_utf8(test::read_body(response).await.to_vec())
        .expect("Could not get body in invalid_batch_post");
    assert_eq!(body, "0");
}

#[actix_rt::test]
async fn accept_new_or_dev_ios() {
    let app = init_app!().await;
    let mut headers = HashMap::new();
    headers.insert(
        "User-Agent",
        "Firefox-iOS-Sync/23.0b17297 (iPhone; iPhone OS 12.4) (Firefox)".to_owned(),
    );

    let req = create_request(
        http::Method::GET,
        "/1.5/42/info/collections",
        Some(headers),
        None,
    )
    .to_request();
    let response = app.call(req).await.unwrap();
    assert!(response.status().is_success());

    let app = init_app!().await;
    let mut headers = HashMap::new();
    headers.insert(
        "User-Agent",
        "Firefox-iOS-Sync/0.0.1b1 (iPhone; iPhone OS 13.5) (Fennec (eoger))".to_owned(),
    );

    let req = create_request(
        http::Method::GET,
        "/1.5/42/info/collections",
        Some(headers),
        None,
    )
    .to_request();
    let response = app.call(req).await.unwrap();
    assert!(response.status().is_success());

    let app = init_app!().await;
    let mut headers = HashMap::new();
    headers.insert(
        "User-Agent",
        "Firefox-iOS-Sync/dev (iPhone; iPhone OS 13.5) (Fennec (eoger))".to_owned(),
    );

    let req = create_request(
        http::Method::GET,
        "/1.5/42/info/collections",
        Some(headers),
        None,
    )
    .to_request();
    let response = app.call(req).await.unwrap();
    assert!(response.status().is_success());
}

#[actix_rt::test]
async fn reject_old_ios() {
    let app = init_app!().await;
    let mut headers = HashMap::new();
    headers.insert(
        "User-Agent",
        "Firefox-iOS-Sync/18.0b1 (iPhone; iPhone OS 13.2.2) (Fennec (synctesting))".to_owned(),
    );

    let req = create_request(
        http::Method::GET,
        "/1.5/42/info/collections",
        Some(headers.clone()),
        None,
    )
    .to_request();
    let response = app.call(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

    let req = create_request(
        http::Method::POST,
        "/1.5/42/storage/tabs?batch=sammich",
        Some(headers),
        Some(json!([
            {"id": "123", "payload": "xxx", "sortindex": 23},
        ])),
    )
    .to_request();
    let response = app.call(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = String::from_utf8(test::read_body(response).await.to_vec()).unwrap();
    assert_eq!(body, "0");
}

#[actix_rt::test]
async fn info_configuration_xlm() {
    let app = init_app!().await;
    let req =
        create_request(http::Method::GET, "/1.5/42/info/configuration", None, None).to_request();
    let response = app.call(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let xlm = response.headers().get(X_LAST_MODIFIED);
    assert!(xlm.is_some());
    assert_eq!(
        xlm.unwrap()
            .to_str()
            .expect("Couldn't parse X-Last-Modified"),
        "0.00"
    );
}

#[actix_rt::test]
async fn overquota() {
    let mut settings = get_test_settings();
    settings.syncstorage.enable_quota = true;
    settings.syncstorage.enforce_quota = true;
    settings.syncstorage.limits.max_quota_limit = 5;
    // persist the db across requests
    settings.syncstorage.database_use_test_transactions = false;
    let app = init_app!(settings).await;

    // Clear out any data that's already in the store.
    let req = create_request(http::Method::DELETE, "/1.5/42/storage", None, None).to_request();
    let resp = app.call(req).await.unwrap();
    assert!(resp.response().status().is_success());

    // Quota is enforced before the write, allowing one write to go over
    let req = create_request(
        http::Method::PUT,
        "/1.5/42/storage/xxx_col2/12345",
        None,
        Some(json!(
            {"payload": "*".repeat(500)}
        )),
    )
    .to_request();
    let response = app.call(req).await.unwrap();
    let status = response.status();
    assert_eq!(status, StatusCode::OK);

    // avoid the request calls running so quickly that they trigger a 503
    actix_rt::time::sleep(Duration::from_millis(10)).await;

    let req = create_request(
        http::Method::PUT,
        "/1.5/42/storage/xxx_col2/12345",
        None,
        Some(json!(
            {"payload": "*".repeat(500)}
        )),
    )
    .to_request();
    let response = app.call(req).await.unwrap();
    let status = response.status();
    assert_eq!(status, StatusCode::FORBIDDEN);
    let body = String::from_utf8(test::read_body(response).await.to_vec()).unwrap();
    // WeaveError::OverQuota
    assert_eq!(body, "14");

    // TODO? Support and test the X-Weave-Quota-Remaining header?
    // match quota_header {
    //     None => {
    //         dbg!(response);
    //     }
    //     Some(x) => assert_eq!(x, "299"),
    // };

    // Delete any persisted data

    // XXX: this should run as cleanup regardless of test failure but it's
    // difficult. e.g. FutureExt::catch_unwind isn't compatible w/ actix-web
    let req = create_request(http::Method::DELETE, "/1.5/42/storage", None, None).to_request();
    let resp = app.call(req).await.unwrap();
    assert!(resp.response().status().is_success());
}

#[actix_rt::test]
async fn lbheartbeat_max_pool_size_check() {
    let mut settings = get_test_settings();
    settings.syncstorage.database_pool_max_size = 10;

    let app = init_app!(settings).await;

    // Test all is well.
    let lb_req = create_request(http::Method::GET, "/__lbheartbeat__", None, None).to_request();
    let sresp = app.call(lb_req).await.unwrap();
    let status = sresp.status();
    // dbg!(status, test::read_body(sresp).await);
    assert!(status.is_success());

    // Exhaust the connections.
    let mut headers: HashMap<&str, String> = HashMap::new();
    headers.insert("TEST_CONNECTIONS", "10".to_owned());
    headers.insert("TEST_IDLES", "0".to_owned());
    let req = create_request(
        http::Method::GET,
        "/__lbheartbeat__",
        Some(headers.clone()),
        None,
    )
    .to_request();
    let sresp = app.call(req).await.unwrap();
    let status = sresp.status();
    // dbg!(status, test::read_body(sresp).await);
    assert!(status == StatusCode::INTERNAL_SERVER_ERROR);

    // check duration for exhausted connections
    actix_rt::time::sleep(Duration::from_secs(1)).await;
    let req =
        create_request(http::Method::GET, "/__lbheartbeat__", Some(headers), None).to_request();
    let sresp = app.call(req).await.unwrap();
    let status = sresp.status();
    let body = test::read_body(sresp).await;
    let resp: HashMap<String, serde_json::value::Value> =
        serde_json::de::from_str(std::str::from_utf8(body.as_ref()).unwrap()).unwrap();
    // dbg!(status, body, &resp);
    assert!(status == StatusCode::INTERNAL_SERVER_ERROR);
    assert!(resp.get("duration_ms").unwrap().as_u64().unwrap() > 1000);

    // check recovery
    let mut headers: HashMap<&str, String> = HashMap::new();
    headers.insert("TEST_CONNECTIONS", "5".to_owned());
    headers.insert("TEST_IDLES", "5".to_owned());
    let req =
        create_request(http::Method::GET, "/__lbheartbeat__", Some(headers), None).to_request();
    let sresp = app.call(req).await.unwrap();
    let status = sresp.status();
    // dbg!(status, test::read_body(sresp).await);
    assert!(status == StatusCode::OK);
}

#[actix_rt::test]
async fn lbheartbeat_ttl_check() {
    let mut settings = get_test_settings();
    settings.syncstorage.lbheartbeat_ttl = Some(2);
    settings.syncstorage.lbheartbeat_ttl_jitter = 60;

    let app = init_app!(settings).await;

    let lb_req = create_request(http::Method::GET, "/__lbheartbeat__", None, None).to_request();
    let sresp = app.call(lb_req).await.unwrap();
    assert!(sresp.status().is_success());

    actix_rt::time::sleep(Duration::from_secs(3)).await;

    let lb_req = create_request(http::Method::GET, "/__lbheartbeat__", None, None).to_request();
    let sresp = app.call(lb_req).await.unwrap();
    assert_eq!(sresp.status(), StatusCode::INTERNAL_SERVER_ERROR);
}
