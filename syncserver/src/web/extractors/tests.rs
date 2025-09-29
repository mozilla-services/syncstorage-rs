use actix_http::h1;
use actix_web::{
    dev::{Payload, ServiceResponse},
    http::{
        header::{HeaderValue, ACCEPT},
        Method,
    },
    test::{self, TestRequest},
    web::Bytes,
    Error, FromRequest, HttpMessage, HttpResponse,
};
use base64::{engine, Engine};
use futures::executor::block_on;
use glean::server_events::GleanEventsLogger;
use hawk::{Credentials, Key, RequestBuilder};
use hmac::{Hmac, Mac};
use lazy_static::lazy_static;
use rand::{thread_rng, Rng};
use serde_json::{self, json};
use sha2::Sha256;
use std::{str::FromStr, sync::Arc};
use syncserver_common;
use syncserver_settings::{Secrets, Settings as GlobalSettings};
use syncstorage_db::{
    mock::{MockDb, MockDbPool},
    params, Sorting, SyncTimestamp,
};
use syncstorage_settings::{Deadman, ServerLimits, Settings as SyncstorageSettings};
use tokio::sync::RwLock;

use crate::{
    server::ServerState,
    web::{
        auth::HawkPayload,
        extractors::{
            get_accepted, BsoPutRequest, BsoQueryParams, BsoRequest, CollectionPostRequest,
            CollectionRequest, HawkIdentifier, Offset, PreConditionHeader, PreConditionHeaderOpt,
            ACCEPTED_CONTENT_TYPES,
        },
    },
};

lazy_static! {
    static ref SERVER_LIMITS: Arc<ServerLimits> = Arc::new(ServerLimits::default());
    static ref SECRETS: Arc<Secrets> = Arc::new(Secrets::new("Ted Koppel is a robot").unwrap());
    static ref USER_ID: u64 = thread_rng().gen_range(0..10000);
    static ref USER_ID_STR: String = USER_ID.to_string();
}

const TEST_HOST: &str = "localhost";
const TEST_PORT: u16 = 8080;
// String is too long for valid name
const INVALID_COLLECTION_NAME: &str = "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz";
const INVALID_BSO_NAME: &str =
    "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz";

fn make_db() -> MockDb {
    MockDb::new()
}

fn make_state() -> ServerState {
    let syncserver_settings = GlobalSettings::default();
    let syncstorage_settings = SyncstorageSettings::default();
    let glean_logger = Arc::new(GleanEventsLogger {
        // app_id corresponds to probe-scraper entry.
        // https://github.com/mozilla/probe-scraper/blob/main/repositories.yaml
        app_id: "syncstorage".to_owned(),
        app_display_version: env!("CARGO_PKG_VERSION").to_owned(),
        app_channel: "prod".to_owned(),
    });
    ServerState {
        db_pool: Box::new(MockDbPool::new()),
        limits: Arc::clone(&SERVER_LIMITS),
        limits_json: serde_json::to_string(&**SERVER_LIMITS).unwrap(),
        port: 8000,
        metrics: syncserver_common::metrics_from_opts(
            &syncstorage_settings.statsd_label,
            syncserver_settings.statsd_host.as_deref(),
            syncserver_settings.statsd_port,
        )
        .unwrap(),
        quota_enabled: syncstorage_settings.enable_quota,
        deadman: Arc::new(RwLock::new(Deadman::default())),
        glean_logger,
        glean_enabled: syncstorage_settings.glean_enabled,
    }
}

fn extract_body_as_str(sresponse: ServiceResponse) -> String {
    String::from_utf8(block_on(test::read_body(sresponse)).to_vec()).unwrap()
}

fn create_valid_hawk_header(
    payload: &HawkPayload,
    secrets: &Secrets,
    method: &str,
    path: &str,
    host: &str,
    port: u16,
) -> String {
    let salt = payload.salt.clone();
    let payload = serde_json::to_string(payload).unwrap();
    let mut hmac = Hmac::<Sha256>::new_from_slice(&secrets.signing_secret).unwrap();
    hmac.update(payload.as_bytes());
    let payload_hash = hmac.finalize().into_bytes();
    let mut id = payload.as_bytes().to_vec();
    id.extend(payload_hash.to_vec());
    let id = engine::general_purpose::URL_SAFE.encode(&id);
    let token_secret = syncserver_common::hkdf_expand_32(
        format!("services.mozilla.com/tokenlib/v1/derive/{}", id).as_bytes(),
        Some(salt.as_bytes()),
        &SECRETS.master_secret,
    )
    .unwrap();
    let token_secret = engine::general_purpose::URL_SAFE.encode(token_secret);
    let credentials = Credentials {
        id,
        key: Key::new(token_secret.as_bytes(), hawk::DigestAlgorithm::Sha256).unwrap(),
    };
    let request = RequestBuilder::new(method, host, port, path)
        .hash(&payload_hash[..])
        .request();
    format!("Hawk {}", request.make_header(&credentials).unwrap())
}

async fn post_collection(
    qs: &str,
    body: &serde_json::Value,
) -> Result<CollectionPostRequest, Error> {
    let payload = HawkPayload::test_default(*USER_ID);
    let state = make_state();
    let secrets = Arc::clone(&SECRETS);
    let path = format!(
        "/1.5/{}/storage/tabs{}{}",
        *USER_ID,
        if !qs.is_empty() { "?" } else { "" },
        qs
    );
    let bod_str = body.to_string();
    let header = create_valid_hawk_header(&payload, &secrets, "POST", &path, TEST_HOST, TEST_PORT);
    let req = TestRequest::with_uri(&format!("http://{}:{}{}", TEST_HOST, TEST_PORT, path))
        .data(state)
        .data(secrets)
        .method(Method::POST)
        .insert_header(("authorization", header))
        .insert_header(("content-type", "application/json; charset=UTF-8"))
        .insert_header(("accept", "application/json;q=0.9,/;q=0.2"))
        .set_payload(bod_str.to_owned())
        .param("uid", USER_ID_STR.as_str())
        .param("collection", "tabs")
        .to_http_request();
    req.extensions_mut().insert(make_db());

    // Not sure why but sending req through *::extract loses the body.
    // Compose a payload here and call the *::from_request
    let (_sender, mut payload) = h1::Payload::create(true);
    payload.unread_data(Bytes::from(bod_str.to_owned()));
    CollectionPostRequest::from_request(&req, &mut payload.into()).await
}

#[test]
fn test_invalid_query_args() {
    let state = make_state();
    let req = TestRequest::with_uri("/?lower=-1.23&sort=whatever")
        .data(state)
        .to_http_request();
    let result = block_on(BsoQueryParams::extract(&req));
    assert!(result.is_err());
    let response: HttpResponse = result.err().unwrap().into();
    assert_eq!(response.status(), 400);
    let body = extract_body_as_str(ServiceResponse::new(req, response));
    assert_eq!(body, "0");

    /* New tests for when we can use descriptive errors
    let err: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(err["status"], 400);
    assert_eq!(err["reason"], "Bad Request");

    let (_lower_error, sort_error) = if err["errors"][0]["name"] == "lower" {
        (&err["errors"][0], &err["errors"][1])
    } else {
        (&err["errors"][1], &err["errors"][0])
    };

    assert_eq!(sort_error["location"], "querystring");
    */
}

#[test]
fn test_weighted_header() {
    // test non-priority, full weight selection
    let req = TestRequest::default().insert_header((
        ACCEPT,
        HeaderValue::from_static("application/json;q=0.9,text/plain"),
    ));
    let selected = get_accepted(
        &req.to_http_request(),
        &ACCEPTED_CONTENT_TYPES,
        "application/json",
    );
    assert_eq!(selected, "text/plain".to_owned());

    // test default for */*
    let req = TestRequest::default()
        .insert_header((ACCEPT, HeaderValue::from_static("*/*;q=0.2,foo/bar")));
    let selected = get_accepted(
        &req.to_http_request(),
        &ACCEPTED_CONTENT_TYPES,
        "application/json",
    );
    assert_eq!(selected, "application/json".to_owned());

    // test default for selected weighted.
    let req = TestRequest::default().insert_header((
        ACCEPT,
        HeaderValue::from_static("foo/bar;q=0.1,application/json;q=0.5,text/plain;q=0.9"),
    ));
    let selected = get_accepted(
        &req.to_http_request(),
        &ACCEPTED_CONTENT_TYPES,
        "application/json",
    );
    assert_eq!(selected, "text/plain".to_owned());
}

#[test]
fn test_valid_query_args() {
    let req = TestRequest::with_uri("/?ids=1,2&full=&sort=index&older=2.43")
        .data(make_state())
        .to_http_request();
    let result = block_on(BsoQueryParams::extract(&req)).unwrap();
    assert_eq!(result.ids, vec!["1", "2"]);
    assert_eq!(result.sort, Sorting::Index);
    assert_eq!(result.older.unwrap(), SyncTimestamp::from_seconds(2.43));
    assert!(result.full);
}

#[test]
fn test_valid_bso_request() {
    let payload = HawkPayload::test_default(*USER_ID);
    let state = make_state();
    let secrets = Arc::clone(&SECRETS);
    let uri = format!("/1.5/{}/storage/tabs/asdf", *USER_ID);
    let header = create_valid_hawk_header(&payload, &secrets, "GET", &uri, TEST_HOST, TEST_PORT);
    let req = TestRequest::with_uri(&uri)
        .data(state)
        .data(secrets)
        .insert_header(("authorization", header))
        .method(Method::GET)
        .param("uid", USER_ID_STR.as_str())
        .param("collection", "tabs")
        .param("bso", "asdf")
        .to_http_request();
    req.extensions_mut().insert(make_db());
    let result = block_on(BsoRequest::extract(&req))
        .expect("Could not get result in test_valid_bso_request");
    assert_eq!(result.user_id.legacy_id, *USER_ID);
    assert_eq!(&result.collection, "tabs");
    assert_eq!(&result.bso, "asdf");
}

#[test]
fn test_invalid_bso_request() {
    let payload = HawkPayload::test_default(*USER_ID);
    let state = make_state();
    let secrets = Arc::clone(&SECRETS);
    let uri = format!("/1.5/{}/storage/tabs/{}", *USER_ID, INVALID_BSO_NAME);
    let header = create_valid_hawk_header(&payload, &secrets, "GET", &uri, TEST_HOST, TEST_PORT);
    let req = TestRequest::with_uri(&uri)
        .data(state)
        .data(secrets)
        .insert_header(("authorization", header))
        .method(Method::GET)
        // `param` sets the value that would be extracted from the tokenized URI, as if the router did it.
        .param("uid", USER_ID_STR.as_str())
        .param("collection", "tabs")
        .param("bso", INVALID_BSO_NAME)
        .to_http_request();
    req.extensions_mut().insert(make_db());
    let result = block_on(BsoRequest::extract(&req));
    assert!(result.is_err());
    let response: HttpResponse = result.err().unwrap().into();
    assert_eq!(response.status(), 400);
    let body = extract_body_as_str(ServiceResponse::new(req, response));
    assert_eq!(body, "0");

    /* New tests for when we can use descriptive errors
    let err: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(err["status"], 400);

    assert_eq!(err["errors"][0]["description"], "regex");
    assert_eq!(err["errors"][0]["location"], "path");
    assert_eq!(err["errors"][0]["name"], "bso");
    assert_eq!(err["errors"][0]["value"], INVALID_BSO_NAME);
    */
}

#[test]
fn test_valid_bso_post_body() {
    let payload = HawkPayload::test_default(*USER_ID);
    let state = make_state();
    let secrets = Arc::clone(&SECRETS);
    let uri = format!("/1.5/{}/storage/tabs/asdf", *USER_ID);
    let header = create_valid_hawk_header(&payload, &secrets, "POST", &uri, TEST_HOST, TEST_PORT);
    let bso_body = json!({
        "id": "128", "payload": "x"
    });
    let req = TestRequest::with_uri(&uri)
        .data(state)
        .data(secrets)
        .insert_header(("authorization", header))
        .insert_header(("content-type", "application/json"))
        .method(Method::POST)
        .set_payload(bso_body.to_string())
        .param("uid", USER_ID_STR.as_str())
        .param("collection", "tabs")
        .param("bso", "asdf")
        .to_http_request();
    req.extensions_mut().insert(make_db());
    let (_sender, mut payload) = h1::Payload::create(true);
    payload.unread_data(Bytes::from(bso_body.to_string()));
    let result = block_on(BsoPutRequest::from_request(&req, &mut payload.into()))
        .expect("Could not get result in test_valid_bso_post_body");
    assert_eq!(result.user_id.legacy_id, *USER_ID);
    assert_eq!(&result.collection, "tabs");
    assert_eq!(&result.bso, "asdf");
    assert_eq!(result.body.payload, Some("x".to_string()));
}

#[test]
fn test_invalid_bso_post_body() {
    let payload = HawkPayload::test_default(*USER_ID);
    let state = make_state();
    let secrets = Arc::clone(&SECRETS);
    let uri = format!("/1.5/{}/storage/tabs/asdf", *USER_ID);
    let header = create_valid_hawk_header(&payload, &secrets, "POST", &uri, TEST_HOST, TEST_PORT);
    let bso_body = json!({
        "payload": "xxx", "sortindex": -9_999_999_999_i64,
    });
    let req = TestRequest::with_uri(&uri)
        .data(state)
        .data(secrets)
        .insert_header(("authorization", header))
        .insert_header(("content-type", "application/json"))
        .method(Method::POST)
        .set_payload(bso_body.to_string())
        .param("uid", USER_ID_STR.as_str())
        .param("collection", "tabs")
        .param("bso", "asdf")
        .to_http_request();
    req.extensions_mut().insert(make_db());
    let result = block_on(BsoPutRequest::extract(&req));
    let response: HttpResponse = result
        .err()
        .expect("Could not get response in test_invalid_bso_post_body")
        .into();
    assert_eq!(response.status(), 400);
    let body = extract_body_as_str(ServiceResponse::new(req, response));
    assert_eq!(body, "8")

    /* New tests for when we can use descriptive errors
    let err: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(err["status"], 400);

    assert_eq!(err["errors"][0]["location"], "body");
    assert_eq!(&err["errors"][0]["name"], "bso");
    */
}

#[test]
fn test_valid_collection_request() {
    let payload = HawkPayload::test_default(*USER_ID);
    let state = make_state();
    let secrets = Arc::clone(&SECRETS);
    let uri = format!("/1.5/{}/storage/tabs", *USER_ID);
    let header = create_valid_hawk_header(&payload, &secrets, "GET", &uri, TEST_HOST, TEST_PORT);
    let req = TestRequest::with_uri(&uri)
        .data(state)
        .data(secrets)
        .insert_header(("authorization", header))
        .insert_header(("accept", "application/json,text/plain:q=0.5"))
        .method(Method::GET)
        .param("uid", USER_ID_STR.as_str())
        .param("collection", "tabs")
        .to_http_request();
    req.extensions_mut().insert(make_db());
    let result = block_on(CollectionRequest::extract(&req))
        .expect("Could not get result in test_valid_collection_request");
    assert_eq!(result.user_id.legacy_id, *USER_ID);
    assert_eq!(&result.collection, "tabs");
}

#[test]
fn test_quoted_bso() {
    let payload = HawkPayload::test_default(*USER_ID);
    let altered_bso = format!("\"{{{}}}\"", *USER_ID);
    let state = make_state();
    let secrets = Arc::clone(&SECRETS);
    let uri = format!(
        "/1.5/{}/storage/tabs/{}",
        *USER_ID,
        urlencoding::encode(&altered_bso)
    );
    let header = create_valid_hawk_header(&payload, &secrets, "GET", &uri, TEST_HOST, TEST_PORT);
    let req = TestRequest::with_uri(&uri)
        .data(state)
        .data(secrets)
        .insert_header(("authorization", header))
        .insert_header(("accept", "application/json,text/plain:q=0.5"))
        .method(Method::GET)
        .to_http_request();
    req.extensions_mut().insert(make_db());
    let result = block_on(BsoRequest::extract(&req))
        .expect("Could not get result in test_valid_collection_request");
    // make sure the altered bsoid matches the unaltered one, without the quotes and cury braces.
    assert_eq!(result.user_id.legacy_id, *USER_ID);
    assert_eq!(altered_bso.as_str(), result.bso);
}

#[test]
fn test_invalid_collection_request() {
    let hawk_payload = HawkPayload::test_default(*USER_ID);
    let state = make_state();
    let secrets = Arc::clone(&SECRETS);
    let uri = format!("/1.5/{}/storage/{}", *USER_ID, INVALID_COLLECTION_NAME);
    let header =
        create_valid_hawk_header(&hawk_payload, &secrets, "GET", &uri, TEST_HOST, TEST_PORT);
    let req = TestRequest::with_uri(&uri)
        .insert_header(("authorization", header))
        .method(Method::GET)
        .data(state)
        .data(secrets)
        .param("uid", USER_ID_STR.as_str())
        .param("collection", INVALID_COLLECTION_NAME)
        .to_http_request();
    req.extensions_mut().insert(make_db());

    let result = block_on(CollectionRequest::extract(&req));
    assert!(result.is_err());
    let response: HttpResponse = result.err().unwrap().into();
    assert_eq!(response.status(), 400);
    let body = extract_body_as_str(ServiceResponse::new(req, response));
    assert_eq!(body, "0");

    /* New tests for when we can use descriptive errors

    let err: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(err["status"], 400);

    assert_eq!(err["errors"][0]["description"], "regex");
    assert_eq!(err["errors"][0]["location"], "path");
    assert_eq!(err["errors"][0]["name"], "collection");
    assert_eq!(err["errors"][0]["value"], INVALID_COLLECTION_NAME);
    */
}

#[actix_rt::test]
async fn test_valid_collection_post_request() {
    // Batch requests require id's on each BSO
    let bso_body = json!([
        {"id": "123", "payload": "xxx", "sortindex": 23},
        {"id": "456", "payload": "xxxasdf", "sortindex": 23}
    ]);
    let result = post_collection("", &bso_body)
        .await
        .expect("Could not get result in test_valid_collection_post_request");
    assert_eq!(result.user_id.legacy_id, *USER_ID);
    assert_eq!(&result.collection, "tabs");
    assert_eq!(result.bsos.valid.len(), 2);
    assert!(result.batch.is_none());
}

#[actix_rt::test]
async fn test_invalid_collection_post_request() {
    // Add extra fields, these will be invalid
    let bso_body = json!([
        {"id": "1", "sortindex": 23, "jump": 1},
        {"id": "2", "sortindex": -99, "hop": "low"}
    ]);
    let result = post_collection("", &bso_body)
        .await
        .expect("Could not get result in test_invalid_collection_post_request");
    assert_eq!(result.user_id.legacy_id, *USER_ID);
    assert_eq!(&result.collection, "tabs");
    assert_eq!(result.bsos.invalid.len(), 2);
}

#[actix_rt::test]
async fn test_valid_collection_batch_post_request() {
    // If the "batch" parameter is has no value or has a value of "true"
    // then a new batch will be created.
    let bso_body = json!([
        {"id": "123", "payload": "xxx", "sortindex": 23},
        {"id": "456", "payload": "xxxasdf", "sortindex": 23}
    ]);
    let result = post_collection("batch=True", &bso_body)
        .await
        .expect("Could not get result in test_valid_collection_batch_post_request");
    assert_eq!(result.user_id.legacy_id, *USER_ID);
    assert_eq!(&result.collection, "tabs");
    assert_eq!(result.bsos.valid.len(), 2);
    let batch = result
        .batch
        .expect("Could not get batch in test_valid_collection_batch_post_request");
    assert!(batch.id.is_none());
    assert!(!batch.commit);

    let result2 = post_collection("batch", &bso_body)
        .await
        .expect("Could not get result2 in test_valid_collection_batch_post_request");
    let batch2 = result2
        .batch
        .expect("Could not get batch2 in test_valid_collection_batch_post_request");
    assert!(batch2.id.is_none());
    assert!(!batch2.commit);

    let result3 = post_collection("batch=MTI%3D&commit=true", &bso_body)
        .await
        .expect("Could not get result3 in test_valid_collection_batch_post_request");
    let batch3 = result3
        .batch
        .expect("Could not get batch3 in test_valid_collection_batch_post_request");
    assert!(batch3.id.is_some());
    assert!(batch3.commit);
}

#[actix_rt::test]
async fn test_invalid_collection_batch_post_request() {
    let bso_body = json!([
        {"id": "123", "payload": "xxx", "sortindex": 23},
        {"id": "456", "payload": "xxxasdf", "sortindex": 23}
    ]);
    let req = TestRequest::with_uri("/")
        .method(Method::POST)
        .data(make_state())
        .to_http_request();
    let result = post_collection("commit=true", &bso_body).await;
    assert!(result.is_err());
    let response: HttpResponse = result.err().unwrap().into();
    assert_eq!(response.status(), 400);
    let body = extract_body_as_str(ServiceResponse::new(req, response));
    assert_eq!(body, "0");
}

#[test]
fn test_invalid_precondition_headers() {
    fn assert_invalid_header(
        req: actix_web::HttpRequest,
        _error_header: &str,
        _error_message: &str,
    ) {
        let result = PreConditionHeaderOpt::extrude(req.headers());
        assert!(result.is_err());
        let response: HttpResponse = result.err().unwrap().into();
        assert_eq!(response.status(), 400);
        let body = extract_body_as_str(ServiceResponse::new(req, response));

        assert_eq!(body, "0");

        /* New tests for when we can use descriptive errors
        let err: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(err["status"], 400);

        assert_eq!(err["errors"][0]["description"], error_message);
        assert_eq!(err["errors"][0]["location"], "header");
        assert_eq!(err["errors"][0]["name"], error_header);
        */
    }
    let req = TestRequest::with_uri("/")
        .data(make_state())
        .insert_header(("X-If-Modified-Since", "32124.32"))
        .insert_header(("X-If-Unmodified-Since", "4212.12"))
        .to_http_request();
    assert_invalid_header(
        req,
        "X-If-Unmodified-Since",
        "conflicts with X-If-Modified-Since",
    );
    let req = TestRequest::with_uri("/")
        .data(make_state())
        .insert_header(("X-If-Modified-Since", "-32.1"))
        .to_http_request();
    assert_invalid_header(req, "X-If-Modified-Since", "Invalid value");
}

#[test]
fn test_valid_precondition_headers() {
    let req = TestRequest::with_uri("/")
        .data(make_state())
        .insert_header(("X-If-Modified-Since", "32.1"))
        .to_http_request();
    let result = PreConditionHeaderOpt::extrude(req.headers())
        .unwrap()
        .opt
        .unwrap();
    assert_eq!(
        result,
        PreConditionHeader::IfModifiedSince(SyncTimestamp::from_seconds(32.1))
    );
    let req = TestRequest::with_uri("/")
        .data(make_state())
        .insert_header(("X-If-Unmodified-Since", "32.14"))
        .to_http_request();
    let result = PreConditionHeaderOpt::extrude(req.headers())
        .unwrap()
        .opt
        .unwrap();
    assert_eq!(
        result,
        PreConditionHeader::IfUnmodifiedSince(SyncTimestamp::from_seconds(32.14))
    );
}

#[test]
fn valid_header_with_valid_path() {
    let hawk_payload = HawkPayload::test_default(*USER_ID);
    let state = make_state();
    let secrets = Arc::clone(&SECRETS);
    let uri = format!("/1.5/{}/storage/col2", *USER_ID);
    let header =
        create_valid_hawk_header(&hawk_payload, &secrets, "GET", &uri, TEST_HOST, TEST_PORT);
    let req = TestRequest::with_uri(&uri)
        .insert_header(("authorization", header))
        .method(Method::GET)
        .data(state)
        .data(secrets)
        .param("uid", USER_ID_STR.as_str())
        .to_http_request();
    let mut payload = Payload::None;
    let result = block_on(HawkIdentifier::from_request(&req, &mut payload))
        .expect("Could not get result in valid_header_with_valid_path");
    assert_eq!(result.legacy_id, *USER_ID);
}

#[test]
fn valid_header_with_invalid_uid_in_path() {
    // the uid in the hawk payload should match the UID in the path.
    let hawk_payload = HawkPayload::test_default(*USER_ID);
    let mismatch_uid = "5";
    let state = make_state();
    let secrets = Arc::clone(&SECRETS);
    let uri = format!("/1.5/{}/storage/col2", mismatch_uid);
    let header =
        create_valid_hawk_header(&hawk_payload, &secrets, "GET", &uri, TEST_HOST, TEST_PORT);
    let req = TestRequest::with_uri(&uri)
        .data(state)
        .data(secrets)
        .insert_header(("authorization", header))
        .method(Method::GET)
        .param("uid", mismatch_uid)
        .to_http_request();
    let result = block_on(HawkIdentifier::extract(&req));
    assert!(result.is_err());
    let response: HttpResponse = result.err().unwrap().into();
    assert_eq!(response.status(), 400);
    let body = extract_body_as_str(ServiceResponse::new(req, response));
    assert_eq!(body, "0");

    /* New tests for when we can use descriptive errors

    let err: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(err["status"], 400);

    assert_eq!(err["errors"][0]["description"], "conflicts with payload");
    assert_eq!(err["errors"][0]["location"], "path");
    assert_eq!(err["errors"][0]["name"], "uid");
    */
}

#[actix_rt::test]
async fn test_max_ttl() {
    let bso_body = json!([
        {"id": "123", "payload": "xxx", "sortindex": 23, "ttl": 94_608_000},
        {"id": "456", "payload": "xxxasdf", "sortindex": 23, "ttl": 999_999_999},
        {"id": "789", "payload": "xxxfoo", "sortindex": 23, "ttl": 1_000_000_000}
    ]);
    let result = post_collection("", &bso_body)
        .await
        .expect("Could not get result in test_valid_collection_post_request");
    assert_eq!(result.user_id.legacy_id, *USER_ID);
    assert_eq!(&result.collection, "tabs");
    assert_eq!(result.bsos.valid.len(), 2);
    assert_eq!(result.bsos.invalid.len(), 1);
    assert!(result.bsos.invalid.contains_key("789"));
}

#[actix_rt::test]
async fn test_offset() {
    let sample_offset = params::Offset {
        timestamp: Some(SyncTimestamp::default()),
        offset: 1234,
    };

    let test_offset = Offset {
        timestamp: None,
        offset: sample_offset.offset,
    };

    let offset_str = sample_offset.to_string();
    assert!(test_offset == Offset::from_str(&offset_str).unwrap())
}
