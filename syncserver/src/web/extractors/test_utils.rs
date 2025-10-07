use std::sync::Arc;

use actix_http::h1;
use actix_web::{
    dev::ServiceResponse,
    http::Method,
    test::{self, TestRequest},
    web::Bytes,
    Error, FromRequest, HttpMessage,
};
use base64::{engine, Engine};
use futures::executor::block_on;
use glean::server_events::GleanEventsLogger;
use hawk::{Credentials, Key, RequestBuilder};
use hmac::{Hmac, Mac};
use lazy_static::lazy_static;
use rand::{thread_rng, Rng};
use sha2::Sha256;
use tokio::sync::RwLock;

use syncserver_common;
use syncserver_settings::{Secrets, Settings as GlobalSettings};
use syncstorage_db::mock::{MockDb, MockDbPool};
use syncstorage_settings::{Deadman, ServerLimits, Settings as SyncstorageSettings};

use super::CollectionPostRequest;
use crate::{server::ServerState, web::auth::HawkPayload};

lazy_static! {
    static ref SERVER_LIMITS: Arc<ServerLimits> = Arc::new(ServerLimits::default());
    pub static ref SECRETS: Arc<Secrets> = Arc::new(Secrets::new("Ted Koppel is a robot").unwrap());
    pub static ref USER_ID: u64 = thread_rng().gen_range(0..10000);
    pub static ref USER_ID_STR: String = USER_ID.to_string();
}

pub const TEST_HOST: &str = "localhost";
pub const TEST_PORT: u16 = 8080;
// String is too long for valid name
pub const INVALID_COLLECTION_NAME: &str = "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz";
pub const INVALID_BSO_NAME: &str =
    "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz";

pub fn make_db() -> MockDb {
    MockDb::new()
}

pub fn make_state() -> ServerState {
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

pub fn extract_body_as_str(sresponse: ServiceResponse) -> String {
    String::from_utf8(block_on(test::read_body(sresponse)).to_vec()).unwrap()
}

pub fn create_valid_hawk_header(
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

pub async fn post_collection(
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
