//! Request header/body/query extractors
//!
//! Handles ensuring the header's, body, and query parameters are correct, extraction to
//! relevant types, and failing correctly with the appropriate errors if issues arise.

use actix_web::{
    dev::Payload,
    web::{Data, Query},
    Error, FromRequest, HttpRequest,
};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use futures::future::LocalBoxFuture;
use hmac::{Hmac, Mac, NewMac};
use serde::Deserialize;
use sha2::Sha256;

use super::db::{self, models::Db, params, results};
use super::error::TokenserverError;
use super::support::TokenData;
use crate::server::ServerState;

const DEFAULT_TOKEN_DURATION: u64 = 5 * 60;

/// Information from the request needed to process a Tokenserver request.
#[derive(Debug, PartialEq)]
pub struct TokenserverRequest {
    pub user: results::GetUser,
    pub fxa_uid: String,
    pub generation: i64,
    pub keys_changed_at: i64,
    pub client_state: String,
    pub shared_secret: String,
    pub hashed_fxa_uid: String,
    pub hashed_device_id: String,
    pub service_id: i32,
    pub duration: u64,
}

impl FromRequest for TokenserverRequest {
    type Config = ();
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let req = req.clone();

        Box::pin(async move {
            let token_data = TokenData::extract(&req).await?;
            let state = get_server_state(&req)?;
            let tokenserver_state = state.tokenserver_state.as_ref().unwrap();
            let fxa_metrics_hash_secret = &tokenserver_state.fxa_metrics_hash_secret.as_bytes();
            let shared_secret =
                String::from_utf8(state.secrets.master_secret.clone()).map_err(|_| {
                    error!("⚠️ Failed to read master secret");

                    TokenserverError::internal_error()
                })?;
            let key_id = KeyId::extract(&req).await?;
            let fxa_uid = token_data.user;
            let hashed_fxa_uid = {
                let hashed_fxa_uid_full = fxa_metrics_hash(&fxa_uid, fxa_metrics_hash_secret);
                hashed_fxa_uid_full[0..32].to_owned()
            };
            let hashed_device_id = {
                let device_id = "none".to_string();
                hash_device_id(&hashed_fxa_uid, &device_id, fxa_metrics_hash_secret)
            };
            let service_id = {
                let path = req.match_info();

                // If we've reached this extractor, we know that the Tokenserver path was matched,
                // meaning "application" and "version" are both present in the URL. So, we can use
                // `unwrap()` here.
                let application = path.get("application").unwrap();
                let version = path.get("version").unwrap();

                if application == "sync" {
                    if version == "1.1" {
                        db::SYNC_1_1_SERVICE_ID
                    } else if version == "1.5" {
                        db::SYNC_1_5_SERVICE_ID
                    } else {
                        return Err(TokenserverError::unsupported(
                            "Unsupported application version",
                        )
                        .into());
                    }
                } else {
                    return Err(TokenserverError::unsupported("Unsupported application").into());
                }
            };
            let user = {
                let db = tokenserver_state.db_pool.get().map_err(|_| {
                    error!("⚠️ Could not acquire database connection");

                    TokenserverError::internal_error()
                })?;
                let email = format!("{}@{}", fxa_uid, tokenserver_state.fxa_email_domain);

                db.get_user(params::GetUser { email, service_id }).await?
            };
            let TokenDuration(duration) = TokenDuration::extract(&req).await?;

            let tokenserver_request = TokenserverRequest {
                user,
                fxa_uid,
                generation: token_data.generation.unwrap_or(0),
                keys_changed_at: key_id.keys_changed_at,
                client_state: key_id.client_state,
                shared_secret,
                hashed_fxa_uid,
                hashed_device_id,
                service_id,
                duration: duration.unwrap_or(DEFAULT_TOKEN_DURATION),
            };

            Ok(tokenserver_request)
        })
    }
}

#[derive(Deserialize)]
struct QueryParams {
    pub duration: Option<String>,
}

struct TokenDuration(Option<u64>);

impl FromRequest for TokenDuration {
    type Config = ();
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let req = req.clone();

        Box::pin(async move {
            let params = Query::<QueryParams>::extract(&req).await?;

            // An error in the "duration" query parameter should never cause a request to fail.
            // Instead, we should simply resort to using the default token duration.
            Ok(Self(params.duration.clone().and_then(|duration_string| {
                match duration_string.parse::<u64>() {
                    // The specified token duration should never be greater than the default
                    // token duration set on the server.
                    Ok(duration) if duration <= DEFAULT_TOKEN_DURATION => Some(duration),
                    _ => None,
                }
            })))
        })
    }
}

impl FromRequest for Box<dyn Db> {
    type Config = ();
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let req = req.clone();

        Box::pin(async move {
            let state = get_server_state(&req)?;
            let tokenserver_state = state.tokenserver_state.as_ref().unwrap();
            let db = tokenserver_state.db_pool.get().map_err(|_| {
                error!("⚠️ Could not acquire database connection");

                TokenserverError::internal_error()
            })?;

            Ok(db)
        })
    }
}

impl FromRequest for TokenData {
    type Config = ();
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let req = req.clone();

        Box::pin(async move {
            // The request must have a valid Authorization header
            let authorization_header = req
                .headers()
                .get("Authorization")
                .ok_or_else(|| TokenserverError::invalid_credentials("Unauthorized"))?
                .to_str()
                .map_err(|_| TokenserverError::invalid_credentials("Unauthorized"))?;

            // The request must use Bearer auth
            if let Some((auth_type, _)) = authorization_header.split_once(" ") {
                if auth_type.to_ascii_lowercase() != "bearer" {
                    return Err(TokenserverError::invalid_credentials("Unsupported").into());
                }
            }

            let auth = BearerAuth::extract(&req)
                .await
                .map_err(|_| TokenserverError::invalid_credentials("Unsupported"))?;
            let state = get_server_state(&req)?;

            // XXX: tokenserver_state will no longer be an Option once the Tokenserver
            // code is rolled out, so we will eventually be able to remove this unwrap().
            let tokenserver_state = state.tokenserver_state.as_ref().unwrap();
            tokenserver_state
                .oauth_verifier
                .verify_token(auth.token())
                .map_err(Into::into)
        })
    }
}

#[derive(Debug, PartialEq)]
struct KeyId {
    client_state: String,
    keys_changed_at: i64,
}

impl FromRequest for KeyId {
    type Config = ();
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let req = req.clone();

        Box::pin(async move {
            let headers = req.headers();
            let x_key_id = headers
                .get("X-KeyId")
                .ok_or_else(|| TokenserverError::invalid_key_id("Missing X-KeyID header"))?
                .to_str()
                .map_err(|_| TokenserverError::invalid_key_id("Invalid X-KeyID header"))?;

            let (keys_changed_at_string, encoded_client_state) = x_key_id
                .split_once("-")
                .ok_or_else(|| TokenserverError::invalid_key_id("Invalid X-KeyID header"))?;

            let client_state = {
                let client_state_bytes =
                    base64::decode_config(encoded_client_state, base64::URL_SAFE_NO_PAD)
                        .map_err(|_| TokenserverError::invalid_key_id("Invalid base64 encoding"))?;

                let client_state = hex::encode(client_state_bytes);

                // If there's a client state value in the X-Client-State header, verify that it matches
                // the value in X-KeyID.
                let maybe_x_client_state = headers
                    .get("X-Client-State")
                    .and_then(|header| header.to_str().ok());
                if let Some(x_client_state) = maybe_x_client_state {
                    if x_client_state != client_state {
                        return Err(TokenserverError::invalid_client_state("Unauthorized").into());
                    }
                }

                client_state
            };

            let keys_changed_at = keys_changed_at_string
                .parse::<i64>()
                .map_err(|_| TokenserverError::invalid_credentials("invalid keysChangedAt"))?;

            Ok(KeyId {
                client_state,
                keys_changed_at,
            })
        })
    }
}

fn get_server_state(req: &HttpRequest) -> Result<&Data<ServerState>, Error> {
    req.app_data::<Data<ServerState>>().ok_or_else(|| {
        error!("⚠️ Could not load the app state");

        TokenserverError::internal_error().into()
    })
}

fn fxa_metrics_hash(fxa_uid: &str, hmac_key: &[u8]) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(hmac_key).expect("HMAC has no key size limit");
    mac.update(fxa_uid.as_bytes());

    let result = mac.finalize().into_bytes();
    hex::encode(result)
}

fn hash_device_id(fxa_uid: &str, device: &str, hmac_key: &[u8]) -> String {
    let mut to_hash = String::from(fxa_uid);
    to_hash.push_str(device);
    let fxa_metrics_hash = fxa_metrics_hash(&to_hash, hmac_key);

    String::from(&fxa_metrics_hash[0..32])
}

#[cfg(test)]
mod tests {
    use super::*;

    use actix_web::{
        dev::ServiceResponse,
        http::{Method, StatusCode},
        test::{self, TestRequest},
        HttpResponse,
    };
    use futures::executor::block_on;
    use lazy_static::lazy_static;
    use serde_json;
    use tokio::sync::RwLock;

    use crate::db::mock::MockDbPool;
    use crate::server::{metrics, ServerState};
    use crate::settings::{Deadman, Secrets, ServerLimits, Settings};
    use crate::tokenserver::{
        self, db::mock::MockDbPool as MockTokenserverPool, MockOAuthVerifier,
    };

    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    lazy_static! {
        static ref SECRETS: Arc<Secrets> = Arc::new(Secrets::new("Ted Koppel is a robot").unwrap());
        static ref SERVER_LIMITS: Arc<ServerLimits> = Arc::new(ServerLimits::default());
    }

    #[actix_rt::test]
    async fn test_valid_tokenserver_request() {
        let fxa_uid = "test123";
        let verifier = {
            let token_data = TokenData {
                user: fxa_uid.to_owned(),
                client_id: "client id".to_owned(),
                scope: vec!["scope".to_owned()],
                generation: Some(1234),
                profile_changed_at: Some(1234),
            };
            let valid = true;

            MockOAuthVerifier { valid, token_data }
        };
        let state = make_state(verifier);

        let req = TestRequest::default()
            .data(state)
            .header("authorization", "Bearer fake_token")
            .header("accept", "application/json,text/plain:q=0.5")
            .header("x-keyid", "0000000001234-YWFh")
            .param("application", "sync")
            .param("version", "1.5")
            .uri("/1.0/sync/1.5?duration=100")
            .method(Method::GET)
            .to_http_request();

        let mut payload = Payload::None;
        let result = TokenserverRequest::from_request(&req, &mut payload)
            .await
            .unwrap();
        let expected_tokenserver_request = TokenserverRequest {
            user: results::GetUser::default(),
            fxa_uid: fxa_uid.to_owned(),
            generation: 1234,
            keys_changed_at: 1234,
            client_state: "616161".to_owned(),
            shared_secret: "Ted Koppel is a robot".to_owned(),
            hashed_fxa_uid: "4d00ecae64b98dd7dc7dea68d0dd615d".to_owned(),
            hashed_device_id: "3a41cccbdd666ebc4199f1f9d1249d44".to_owned(),
            service_id: db::SYNC_1_5_SERVICE_ID,
            duration: 100,
        };

        assert_eq!(result, expected_tokenserver_request);
    }

    #[actix_rt::test]
    async fn test_invalid_auth_token() {
        let fxa_uid = "test123";
        let verifier = {
            let start = SystemTime::now();
            let current_time = start.duration_since(UNIX_EPOCH).unwrap();
            let token_data = TokenData {
                user: fxa_uid.to_owned(),
                client_id: "client id".to_owned(),
                scope: vec!["scope".to_owned()],
                generation: Some(current_time.as_secs() as i64),
                profile_changed_at: Some(current_time.as_secs() as i64),
            };
            let valid = false;

            MockOAuthVerifier { valid, token_data }
        };
        let state = make_state(verifier);

        let request = TestRequest::default()
            .data(state)
            .header("authorization", "Bearer fake_token")
            .header("accept", "application/json,text/plain:q=0.5")
            .header("x-keyid", "0000000001234-YWFh")
            .param("application", "sync")
            .param("version", "1.5")
            .method(Method::GET)
            .to_http_request();

        let response: HttpResponse = TokenserverRequest::extract(&request)
            .await
            .unwrap_err()
            .into();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let expected_error = TokenserverError::invalid_credentials("Unauthorized");
        let body = extract_body_as_str(ServiceResponse::new(request, response));
        assert_eq!(body, serde_json::to_string(&expected_error).unwrap());
    }

    #[actix_rt::test]
    async fn test_application_and_version() {
        fn build_request() -> TestRequest {
            let fxa_uid = "test123";
            let verifier = {
                let start = SystemTime::now();
                let current_time = start.duration_since(UNIX_EPOCH).unwrap();
                let token_data = TokenData {
                    user: fxa_uid.to_owned(),
                    client_id: "client id".to_owned(),
                    scope: vec!["scope".to_owned()],
                    generation: Some(current_time.as_secs() as i64),
                    profile_changed_at: Some(current_time.as_secs() as i64),
                };
                let valid = true;

                MockOAuthVerifier { valid, token_data }
            };

            TestRequest::default()
                .data(make_state(verifier))
                .header("authorization", "Bearer fake_token")
                .header("accept", "application/json,text/plain:q=0.5")
                .header("x-keyid", "0000000001234-YWFh")
                .method(Method::GET)
        }

        // Valid application and invalid version
        {
            let request = build_request()
                .param("application", "sync")
                .param("version", "1.0")
                .to_http_request();

            let response: HttpResponse = TokenserverRequest::extract(&request)
                .await
                .unwrap_err()
                .into();

            assert_eq!(response.status(), StatusCode::NOT_FOUND);

            let expected_error = TokenserverError::unsupported("Unsupported application version");
            let body = extract_body_as_str(ServiceResponse::new(request, response));
            assert_eq!(body, serde_json::to_string(&expected_error).unwrap());
        }

        // Invalid application and valid version
        {
            let request = build_request()
                .param("application", "push")
                .param("version", "1.5")
                .to_http_request();

            let response: HttpResponse = TokenserverRequest::extract(&request)
                .await
                .unwrap_err()
                .into();

            assert_eq!(response.status(), StatusCode::NOT_FOUND);

            let expected_error = TokenserverError::unsupported("Unsupported application");
            let body = extract_body_as_str(ServiceResponse::new(request, response));
            assert_eq!(body, serde_json::to_string(&expected_error).unwrap());
        }

        // Invalid application and invalid version
        {
            let request = build_request()
                .param("application", "push")
                .param("version", "1.0")
                .to_http_request();

            let response: HttpResponse = TokenserverRequest::extract(&request)
                .await
                .unwrap_err()
                .into();

            assert_eq!(response.status(), StatusCode::NOT_FOUND);

            let expected_error = TokenserverError::unsupported("Unsupported application");
            let body = extract_body_as_str(ServiceResponse::new(request, response));
            assert_eq!(body, serde_json::to_string(&expected_error).unwrap());
        }

        // Valid application and valid version (1.5)
        {
            let request = build_request()
                .param("application", "sync")
                .param("version", "1.5")
                .to_http_request();

            let tokenserver_request = TokenserverRequest::extract(&request).await.unwrap();

            assert_eq!(tokenserver_request.service_id, db::SYNC_1_5_SERVICE_ID);
        }

        // Valid application and valid version (1.1)
        {
            let request = build_request()
                .param("application", "sync")
                .param("version", "1.1")
                .to_http_request();

            let tokenserver_request = TokenserverRequest::extract(&request).await.unwrap();

            assert_eq!(tokenserver_request.service_id, db::SYNC_1_1_SERVICE_ID);
        }
    }

    #[actix_rt::test]
    async fn test_key_id() {
        fn build_request() -> TestRequest {
            let fxa_uid = "test123";
            let verifier = {
                let start = SystemTime::now();
                let current_time = start.duration_since(UNIX_EPOCH).unwrap();
                let token_data = TokenData {
                    user: fxa_uid.to_owned(),
                    client_id: "client id".to_owned(),
                    scope: vec!["scope".to_owned()],
                    generation: Some(current_time.as_secs() as i64),
                    profile_changed_at: Some(current_time.as_secs() as i64),
                };
                let valid = true;

                MockOAuthVerifier { valid, token_data }
            };

            TestRequest::default()
                .data(make_state(verifier))
                .header("authorization", "Bearer fake_token")
                .header("accept", "application/json,text/plain:q=0.5")
                .param("application", "sync")
                .param("version", "1.5")
                .method(Method::GET)
        }

        // Request with no X-KeyID header
        {
            let request = build_request().to_http_request();
            let response: HttpResponse = KeyId::extract(&request).await.unwrap_err().into();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

            let expected_error = TokenserverError::invalid_key_id("Missing X-KeyID header");
            let body = extract_body_as_str(ServiceResponse::new(request, response));
            assert_eq!(body, serde_json::to_string(&expected_error).unwrap());
        }

        // X-KeyID header with non-visible ASCII characters (\u{200B} is the zero-width space character)
        {
            let request = build_request()
                .header("x-keyid", "\u{200B}")
                .to_http_request();
            let response: HttpResponse = KeyId::extract(&request).await.unwrap_err().into();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

            let expected_error = TokenserverError::invalid_key_id("Invalid X-KeyID header");
            let body = extract_body_as_str(ServiceResponse::new(request, response));
            assert_eq!(body, serde_json::to_string(&expected_error).unwrap());
        }

        // Improperly-formatted X-KeyID header
        {
            let request = build_request()
                .header("x-keyid", "00000000")
                .to_http_request();
            let response: HttpResponse = KeyId::extract(&request).await.unwrap_err().into();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

            let expected_error = TokenserverError::invalid_key_id("Invalid X-KeyID header");
            let body = extract_body_as_str(ServiceResponse::new(request, response));
            assert_eq!(body, serde_json::to_string(&expected_error).unwrap());
        }

        // X-KeyID header with improperly-base64-encoded client state bytes
        {
            let request = build_request()
                .header("x-keyid", "0000000001234-notbase64")
                .to_http_request();
            let response: HttpResponse = KeyId::extract(&request).await.unwrap_err().into();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

            let expected_error = TokenserverError::invalid_key_id("Invalid base64 encoding");
            let body = extract_body_as_str(ServiceResponse::new(request, response));
            assert_eq!(body, serde_json::to_string(&expected_error).unwrap());
        }

        // X-KeyID header with non-UTF-8 bytes
        {
            let request = build_request()
                .header("x-keyid", &[0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8][..])
                .to_http_request();
            let response: HttpResponse = KeyId::extract(&request).await.unwrap_err().into();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

            let expected_error = TokenserverError::invalid_key_id("Invalid X-KeyID header");
            let body = extract_body_as_str(ServiceResponse::new(request, response));
            assert_eq!(body, serde_json::to_string(&expected_error).unwrap());
        }

        // X-KeyID header with non-integral keys_changed_at
        {
            let request = build_request()
                .header("x-keyid", "notanumber-YWFh")
                .to_http_request();
            let response: HttpResponse = KeyId::extract(&request).await.unwrap_err().into();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

            let expected_error = TokenserverError::invalid_credentials("invalid keysChangedAt");
            let body = extract_body_as_str(ServiceResponse::new(request, response));
            assert_eq!(body, serde_json::to_string(&expected_error).unwrap());
        }

        // X-KeyID header with client state that does not match that in the X-Client-State header
        {
            let request = build_request()
                .header("x-keyid", "0000000001234-YWFh")
                .header("x-client-state", "626262")
                .to_http_request();
            let response: HttpResponse = KeyId::extract(&request).await.unwrap_err().into();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

            let expected_error = TokenserverError::invalid_client_state("Unauthorized");
            let body = extract_body_as_str(ServiceResponse::new(request, response));
            assert_eq!(body, serde_json::to_string(&expected_error).unwrap());
        }

        // Valid X-KeyID header with matching X-Client-State header
        {
            let request = build_request()
                .header("x-keyid", "0000000001234-YWFh")
                .header("x-client-state", "616161")
                .to_http_request();
            let key_id = KeyId::extract(&request).await.unwrap();
            let expected_key_id = KeyId {
                client_state: "616161".to_owned(),
                keys_changed_at: 1234,
            };

            assert_eq!(key_id, expected_key_id);
        }

        // Valid X-KeyID header with no X-Client-State header
        {
            let request = build_request()
                .header("x-keyid", "0000000001234-YWFh")
                .to_http_request();
            let key_id = KeyId::extract(&request).await.unwrap();
            let expected_key_id = KeyId {
                client_state: "616161".to_owned(),
                keys_changed_at: 1234,
            };

            assert_eq!(key_id, expected_key_id);
        }
    }

    fn extract_body_as_str(sresponse: ServiceResponse) -> String {
        String::from_utf8(block_on(test::read_body(sresponse)).to_vec()).unwrap()
    }

    fn make_state(verifier: MockOAuthVerifier) -> ServerState {
        let settings = Settings::default();
        let tokenserver_state = tokenserver::ServerState {
            fxa_email_domain: "test.com".to_owned(),
            fxa_metrics_hash_secret: "".to_owned(),
            oauth_verifier: Box::new(verifier),
            db_pool: Box::new(MockTokenserverPool::new()),
        };

        ServerState {
            db_pool: Box::new(MockDbPool::new()),
            limits: Arc::clone(&SERVER_LIMITS),
            limits_json: serde_json::to_string(&**SERVER_LIMITS).unwrap(),
            secrets: Arc::clone(&SECRETS),
            tokenserver_state: Some(tokenserver_state),
            port: 8000,
            metrics: Box::new(metrics::metrics_from_opts(&settings).unwrap()),
            quota_enabled: settings.enable_quota,
            deadman: Arc::new(RwLock::new(Deadman::default())),
        }
    }
}
