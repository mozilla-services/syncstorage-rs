//! Request header/body/query extractors
//!
//! Handles ensuring the header's, body, and query parameters are correct, extraction to
//! relevant types, and failing correctly with the appropriate errors if issues arise.

use std::sync::Arc;

use actix_web::{
    dev::Payload,
    http::StatusCode,
    web::{self, Data, Query},
    Error, FromRequest, HttpRequest,
};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use futures::future::LocalBoxFuture;
use hmac::{Hmac, Mac, NewMac};
use lazy_static::lazy_static;
use regex::Regex;
use serde::Deserialize;
use sha2::Sha256;

use super::db::{self, models::Db, params, pool::DbPool, results};
use super::error::{ErrorLocation, TokenserverError};
use super::support::TokenData;
use super::NodeType;
use super::ServerState;
use crate::settings::Secrets;

lazy_static! {
    static ref CLIENT_STATE_REGEX: Regex = Regex::new("^[a-zA-Z0-9._-]{1,32}$").unwrap();
}

const DEFAULT_TOKEN_DURATION: u64 = 5 * 60;

/// Information from the request needed to process a Tokenserver request.
#[derive(Debug, Default, PartialEq)]
pub struct TokenserverRequest {
    pub user: results::GetOrCreateUser,
    pub fxa_uid: String,
    pub email: String,
    pub generation: Option<i64>,
    pub keys_changed_at: i64,
    pub client_state: String,
    pub shared_secret: String,
    pub hashed_fxa_uid: String,
    pub hashed_device_id: String,
    pub service_id: i32,
    pub duration: u64,
    pub node_type: NodeType,
}

impl TokenserverRequest {
    /// Performs an elaborate set of consistency checks on the
    /// provided claims, which we expect to behave as follows:
    ///
    ///   * `generation` is a monotonic timestamp, and increases every time
    ///     there is an authentication-related change on the user's account.
    ///
    ///   * `keys_changed_at` is a monotonic timestamp, and increases every time
    ///     the user's keys change. This is a type of auth-related change, so
    ///     `keys_changed_at` <= `generation` at all times.
    ///
    ///   * `client_state` is a key fingerprint and should never change back
    ///      to a previously-seen value.
    ///
    /// Callers who provide identity claims that violate any of these rules
    /// either have stale credetials (in which case they should re-authenticate)
    /// or are buggy (in which case we deny them access to the user's data).
    ///
    /// The logic here is slightly complicated by the fact that older versions
    /// of the FxA server may not have been sending all the expected fields, and
    /// that some clients do not report the `generation` timestamp.
    fn validate(&self) -> Result<(), TokenserverError> {
        // If the caller reports a generation number, then a change
        // in keys should correspond to a change in generation number.
        // Unfortunately a previous version of the server that didn't
        // have `keys_changed_at` support may have already seen and
        // written the new value of `generation`. The best we can do
        // here is enforce that `keys_changed_at` <= `generation`.
        if let (Some(generation), Some(user_keys_changed_at)) =
            (self.generation, self.user.keys_changed_at)
        {
            if self.keys_changed_at > user_keys_changed_at && generation < self.keys_changed_at {
                return Err(TokenserverError::invalid_keys_changed_at());
            }
        }

        // The client state on the request must not have been used in the past.
        if self.user.old_client_states.contains(&self.client_state) {
            let error_message = "Unacceptable client-state value stale value";
            return Err(TokenserverError::invalid_client_state(error_message));
        }

        // If the client state on the request differs from the most recently-used client state, it must
        // be accompanied by a valid change in generation (if the client reports a generation).
        if let Some(generation) = self.generation {
            if self.client_state != self.user.client_state && generation <= self.user.generation {
                let error_message =
                    "Unacceptable client-state value new value with no generation change";
                return Err(TokenserverError::invalid_client_state(error_message));
            }
        }

        // If the client state on the request differs from the most recently-used client state, it must
        // be accompanied by a valid change in keys_changed_at
        if let Some(user_keys_changed_at) = self.user.keys_changed_at {
            if self.client_state != self.user.client_state
                && self.keys_changed_at <= user_keys_changed_at
            {
                let error_message =
                    "Unacceptable client-state value new value with no keys_changed_at change";
                return Err(TokenserverError::invalid_client_state(error_message));
            }
        }

        // The generation on the request cannot be earlier than the generation stored on the user
        // record.
        if let Some(generation) = self.generation {
            if self.user.generation > generation {
                return Err(TokenserverError::invalid_generation());
            }
        }

        // The keys_changed_at on the request cannot be earlier than the keys_changed_at stored on
        // the user record.
        if let Some(user_keys_changed_at) = self.user.keys_changed_at {
            if user_keys_changed_at > self.keys_changed_at {
                return Err(TokenserverError::invalid_keys_changed_at());
            }
        }

        Ok(())
    }
}

impl FromRequest for TokenserverRequest {
    type Config = ();
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let req = req.clone();

        Box::pin(async move {
            let token_data = TokenData::extract(&req).await?;

            // XXX: Tokenserver state will no longer be an Option once the Tokenserver
            // code is rolled out, so we will eventually be able to remove this unwrap().
            let state = get_server_state(&req)?.as_ref().as_ref().unwrap();
            let shared_secret = get_secret(&req)?;
            let fxa_metrics_hash_secret = &state.fxa_metrics_hash_secret.as_bytes();
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
                            version.to_owned(),
                        )
                        .into());
                    }
                } else {
                    // NOTE: It would probably be better to include the name of the unsupported
                    // application in the error message, but the old Tokenserver only includes
                    // "application" in the error message. To keep the APIs between the old and
                    // new Tokenservers as close as possible, we defer to the error message from
                    // the old Tokenserver.
                    return Err(TokenserverError::unsupported(
                        "Unsupported application",
                        "application".to_owned(),
                    )
                    .into());
                }
            };
            let email = format!("{}@{}", fxa_uid, state.fxa_email_domain);
            let user = {
                let db = <Box<dyn Db>>::extract(&req).await?;

                db.get_or_create_user(params::GetOrCreateUser {
                    service_id,
                    email: email.clone(),
                    generation: token_data.generation.unwrap_or(0),
                    client_state: key_id.client_state.clone(),
                    keys_changed_at: Some(key_id.keys_changed_at),
                    capacity_release_rate: state.node_capacity_release_rate,
                })
                .await?
            };
            let duration = {
                let params = Query::<QueryParams>::extract(&req).await?;

                // An error in the "duration" query parameter should never cause a request to fail.
                // Instead, we should simply resort to using the default token duration.
                params.duration.as_ref().and_then(|duration_string| {
                    match duration_string.parse::<u64>() {
                        // The specified token duration should never be greater than the default
                        // token duration set on the server.
                        Ok(duration) if duration <= DEFAULT_TOKEN_DURATION => Some(duration),
                        _ => None,
                    }
                })
            };

            let tokenserver_request = TokenserverRequest {
                user,
                fxa_uid,
                email,
                generation: token_data.generation,
                keys_changed_at: key_id.keys_changed_at,
                client_state: key_id.client_state,
                shared_secret,
                hashed_fxa_uid,
                hashed_device_id,
                service_id,
                duration: duration.unwrap_or(DEFAULT_TOKEN_DURATION),
                node_type: state.node_type,
            };

            tokenserver_request.validate()?;

            Ok(tokenserver_request)
        })
    }
}

#[derive(Deserialize)]
struct QueryParams {
    pub duration: Option<String>,
}

impl FromRequest for Box<dyn Db> {
    type Config = ();
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let req = req.clone();

        Box::pin(async move {
            <Box<dyn DbPool>>::extract(&req)
                .await?
                .get()
                .await
                .map_err(|_| {
                    error!("⚠️ Could not acquire database connection");

                    TokenserverError::internal_error().into()
                })
        })
    }
}

impl FromRequest for Box<dyn DbPool> {
    type Config = ();
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let req = req.clone();

        Box::pin(async move {
            // XXX: Tokenserver state will no longer be an Option once the Tokenserver
            // code is rolled out, so we will eventually be able to remove this unwrap().
            let state = get_server_state(&req)?.as_ref().as_ref().unwrap();

            Ok(state.db_pool.clone())
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
                .ok_or_else(|| TokenserverError::unauthorized("Unauthorized"))?
                .to_str()
                .map_err(|_| TokenserverError::unauthorized("Unauthorized"))?;

            // The request must use Bearer auth
            if let Some((auth_type, _)) = authorization_header.split_once(" ") {
                if auth_type.to_ascii_lowercase() != "bearer" {
                    return Err(TokenserverError::unauthorized("Unsupported").into());
                }
            }

            let auth = BearerAuth::extract(&req)
                .await
                .map_err(|_| TokenserverError::invalid_credentials("Unsupported"))?;
            // XXX: The Tokenserver state will no longer be an Option once the Tokenserver
            // code is rolled out, so we will eventually be able to remove this unwrap().
            let state = get_server_state(&req)?.as_ref().as_ref().unwrap();
            let oauth_verifier = state.oauth_verifier.clone();

            web::block(move || oauth_verifier.verify_token(auth.token()))
                .await
                .map_err(TokenserverError::from)
                .map_err(Into::into)
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
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
            let maybe_x_client_state = headers
                .get("X-Client-State")
                .and_then(|header| header.to_str().ok());

            // If there's a client state value in the X-Client-State header, make sure it is valid
            if let Some(x_client_state) = maybe_x_client_state {
                if !CLIENT_STATE_REGEX.is_match(x_client_state) {
                    return Err(TokenserverError {
                        status: "error",
                        location: ErrorLocation::Header,
                        description: "Invalid client state value",
                        name: "X-Client-State".to_owned(),
                        http_status: StatusCode::BAD_REQUEST,
                    }
                    .into());
                }
            }

            let x_key_id = headers
                .get("X-KeyID")
                .ok_or_else(|| TokenserverError::invalid_key_id("Missing X-KeyID header"))?
                .to_str()
                .map_err(|_| TokenserverError::invalid_key_id("Invalid X-KeyID header"))?;

            let (keys_changed_at_string, encoded_client_state) = x_key_id
                .split_once("-")
                .ok_or_else(|| TokenserverError::invalid_credentials("Unauthorized"))?;

            let client_state = {
                let client_state_bytes =
                    base64::decode_config(encoded_client_state, base64::URL_SAFE_NO_PAD)
                        .map_err(|_| TokenserverError::invalid_credentials("Unauthorized"))?;

                let client_state = hex::encode(client_state_bytes);

                // If there's a client state value in the X-Client-State header, verify that it matches
                // the value in X-KeyID.
                if let Some(x_client_state) = maybe_x_client_state {
                    if x_client_state != client_state {
                        return Err(TokenserverError {
                            status: "invalid-client-state",
                            location: ErrorLocation::Body,
                            ..TokenserverError::default()
                        }
                        .into());
                    }
                }

                client_state
            };

            let keys_changed_at = keys_changed_at_string
                .parse::<i64>()
                .map_err(|_| TokenserverError::invalid_credentials("Unauthorized"))?;

            Ok(KeyId {
                client_state,
                keys_changed_at,
            })
        })
    }
}

fn get_server_state(req: &HttpRequest) -> Result<&Data<Option<ServerState>>, Error> {
    req.app_data::<Data<Option<ServerState>>>().ok_or_else(|| {
        error!("⚠️ Could not load the app state");

        TokenserverError::internal_error().into()
    })
}

fn get_secret(req: &HttpRequest) -> Result<String, Error> {
    let secrets = req.app_data::<Data<Arc<Secrets>>>().ok_or_else(|| {
        error!("⚠️ Could not load the app secrets");

        Error::from(TokenserverError::internal_error())
    })?;

    String::from_utf8(secrets.master_secret.clone()).map_err(|_| {
        error!("⚠️ Failed to read master secret");

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

    use crate::settings::{Secrets, ServerLimits};
    use crate::tokenserver::{
        db::mock::MockDbPool as MockTokenserverPool, MockOAuthVerifier, ServerState,
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
            .data(Some(state))
            .data(Arc::clone(&SECRETS))
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
            user: results::GetOrCreateUser::default(),
            fxa_uid: fxa_uid.to_owned(),
            email: "test123@test.com".to_owned(),
            generation: Some(1234),
            keys_changed_at: 1234,
            client_state: "616161".to_owned(),
            shared_secret: "Ted Koppel is a robot".to_owned(),
            hashed_fxa_uid: "4d00ecae64b98dd7dc7dea68d0dd615d".to_owned(),
            hashed_device_id: "3a41cccbdd666ebc4199f1f9d1249d44".to_owned(),
            service_id: db::SYNC_1_5_SERVICE_ID,
            duration: 100,
            node_type: NodeType::default(),
        };

        assert_eq!(result, expected_tokenserver_request);
    }

    #[actix_rt::test]
    async fn test_invalid_auth_token() {
        let fxa_uid = "test123";
        let verifier = {
            let token_data = TokenData {
                user: fxa_uid.to_owned(),
                client_id: "client id".to_owned(),
                scope: vec!["scope".to_owned()],
                generation: Some(1234),
                profile_changed_at: None,
            };
            let valid = false;

            MockOAuthVerifier { valid, token_data }
        };
        let state = make_state(verifier);

        let request = TestRequest::default()
            .data(Some(state))
            .data(Arc::clone(&SECRETS))
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
                let token_data = TokenData {
                    user: fxa_uid.to_owned(),
                    client_id: "client id".to_owned(),
                    scope: vec!["scope".to_owned()],
                    generation: Some(1234),
                    profile_changed_at: None,
                };
                let valid = true;

                MockOAuthVerifier { valid, token_data }
            };

            TestRequest::default()
                .data(Some(make_state(verifier)))
                .data(Arc::clone(&SECRETS))
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

            let expected_error =
                TokenserverError::unsupported("Unsupported application version", "1.0".to_owned());
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

            let expected_error =
                TokenserverError::unsupported("Unsupported application", "application".to_owned());
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

            let expected_error =
                TokenserverError::unsupported("Unsupported application", "application".to_owned());
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
                .data(Some(make_state(verifier)))
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

            let expected_error = TokenserverError::invalid_credentials("Unauthorized");
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

            let expected_error = TokenserverError::invalid_credentials("Unauthorized");
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

            let expected_error = TokenserverError::invalid_credentials("Unauthorized");
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

            let expected_error = TokenserverError {
                status: "invalid-client-state",
                location: ErrorLocation::Body,
                ..TokenserverError::default()
            };
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

    #[actix_rt::test]
    async fn test_old_generation() {
        // The request includes a generation that is less than the generation currently stored on
        // the user record
        let tokenserver_request = TokenserverRequest {
            user: results::GetOrCreateUser {
                uid: 1,
                email: "test@test.com".to_owned(),
                client_state: "616161".to_owned(),
                generation: 1234,
                node: "node".to_owned(),
                keys_changed_at: Some(1234),
                replaced_at: None,
                created_at: 1234,
                old_client_states: vec![],
            },
            fxa_uid: "test".to_owned(),
            email: "test@test.com".to_owned(),
            generation: Some(1233),
            keys_changed_at: 1234,
            client_state: "616161".to_owned(),
            shared_secret: "secret".to_owned(),
            hashed_fxa_uid: "abcdef".to_owned(),
            hashed_device_id: "abcdef".to_owned(),
            service_id: 1,
            duration: DEFAULT_TOKEN_DURATION,
            node_type: NodeType::default(),
        };

        let error = tokenserver_request.validate().unwrap_err();
        assert_eq!(error, TokenserverError::invalid_generation());
    }

    #[actix_rt::test]
    async fn test_old_keys_changed_at() {
        // The request includes a keys_changed_at that is less than the keys_changed_at currently
        // stored on the user record
        let tokenserver_request = TokenserverRequest {
            user: results::GetOrCreateUser {
                uid: 1,
                email: "test@test.com".to_owned(),
                client_state: "616161".to_owned(),
                generation: 1234,
                node: "node".to_owned(),
                keys_changed_at: Some(1234),
                created_at: 1234,
                replaced_at: None,
                old_client_states: vec![],
            },
            fxa_uid: "test".to_owned(),
            email: "test@test.com".to_owned(),
            generation: Some(1234),
            keys_changed_at: 1233,
            client_state: "616161".to_owned(),
            shared_secret: "secret".to_owned(),
            hashed_fxa_uid: "abcdef".to_owned(),
            hashed_device_id: "abcdef".to_owned(),
            service_id: 1,
            duration: DEFAULT_TOKEN_DURATION,
            node_type: NodeType::default(),
        };

        let error = tokenserver_request.validate().unwrap_err();
        assert_eq!(error, TokenserverError::invalid_keys_changed_at());
    }

    #[actix_rt::test]
    async fn test_keys_changed_without_generation_change() {
        // The request includes a new value for keys_changed_at without a new value for generation
        let tokenserver_request = TokenserverRequest {
            user: results::GetOrCreateUser {
                uid: 1,
                email: "test@test.com".to_owned(),
                client_state: "616161".to_owned(),
                generation: 1234,
                node: "node".to_owned(),
                keys_changed_at: Some(1234),
                created_at: 1234,
                replaced_at: None,
                old_client_states: vec![],
            },
            fxa_uid: "test".to_owned(),
            email: "test@test.com".to_owned(),
            generation: Some(1234),
            keys_changed_at: 1235,
            client_state: "616161".to_owned(),
            shared_secret: "secret".to_owned(),
            hashed_fxa_uid: "abcdef".to_owned(),
            hashed_device_id: "abcdef".to_owned(),
            service_id: 1,
            duration: DEFAULT_TOKEN_DURATION,
            node_type: NodeType::default(),
        };

        let error = tokenserver_request.validate().unwrap_err();
        assert_eq!(error, TokenserverError::invalid_keys_changed_at());
    }

    #[actix_rt::test]
    async fn test_old_client_state() {
        // The request includes a previously-used client state that is not the user's current
        // client state
        let tokenserver_request = TokenserverRequest {
            user: results::GetOrCreateUser {
                uid: 1,
                email: "test@test.com".to_owned(),
                client_state: "616161".to_owned(),
                generation: 1234,
                node: "node".to_owned(),
                keys_changed_at: Some(1234),
                created_at: 1234,
                replaced_at: None,
                old_client_states: vec!["626262".to_owned()],
            },
            fxa_uid: "test".to_owned(),
            email: "test@test.com".to_owned(),
            generation: Some(1234),
            keys_changed_at: 1234,
            client_state: "626262".to_owned(),
            shared_secret: "secret".to_owned(),
            hashed_fxa_uid: "abcdef".to_owned(),
            hashed_device_id: "abcdef".to_owned(),
            service_id: 1,
            duration: DEFAULT_TOKEN_DURATION,
            node_type: NodeType::default(),
        };

        let error = tokenserver_request.validate().unwrap_err();
        let error_message = "Unacceptable client-state value stale value";
        assert_eq!(error, TokenserverError::invalid_client_state(error_message));
    }

    #[actix_rt::test]
    async fn test_new_client_state_without_generation_change() {
        // The request includes a new client state without a new generation value
        let tokenserver_request = TokenserverRequest {
            user: results::GetOrCreateUser {
                uid: 1,
                email: "test@test.com".to_owned(),
                client_state: "616161".to_owned(),
                generation: 1234,
                node: "node".to_owned(),
                keys_changed_at: Some(1234),
                created_at: 1234,
                replaced_at: None,
                old_client_states: vec![],
            },
            fxa_uid: "test".to_owned(),
            email: "test@test.com".to_owned(),
            generation: Some(1234),
            keys_changed_at: 1234,
            client_state: "626262".to_owned(),
            shared_secret: "secret".to_owned(),
            hashed_fxa_uid: "abcdef".to_owned(),
            hashed_device_id: "abcdef".to_owned(),
            service_id: 1,
            duration: DEFAULT_TOKEN_DURATION,
            node_type: NodeType::default(),
        };

        let error = tokenserver_request.validate().unwrap_err();
        let error_message = "Unacceptable client-state value new value with no generation change";
        assert_eq!(error, TokenserverError::invalid_client_state(error_message));
    }

    #[actix_rt::test]
    async fn test_new_client_state_without_key_change() {
        // The request includes a new client state without a new keys_changed_at value
        let tokenserver_request = TokenserverRequest {
            user: results::GetOrCreateUser {
                uid: 1,
                email: "test@test.com".to_owned(),
                client_state: "616161".to_owned(),
                generation: 1234,
                node: "node".to_owned(),
                keys_changed_at: Some(1234),
                created_at: 1234,
                replaced_at: None,
                old_client_states: vec![],
            },
            fxa_uid: "test".to_owned(),
            email: "test@test.com".to_owned(),
            generation: Some(1235),
            keys_changed_at: 1234,
            client_state: "626262".to_owned(),
            shared_secret: "secret".to_owned(),
            hashed_fxa_uid: "abcdef".to_owned(),
            hashed_device_id: "abcdef".to_owned(),
            service_id: 1,
            duration: DEFAULT_TOKEN_DURATION,
            node_type: NodeType::default(),
        };

        let error = tokenserver_request.validate().unwrap_err();
        let error_message =
            "Unacceptable client-state value new value with no keys_changed_at change";
        assert_eq!(error, TokenserverError::invalid_client_state(error_message));
    }

    fn extract_body_as_str(sresponse: ServiceResponse) -> String {
        String::from_utf8(block_on(test::read_body(sresponse)).to_vec()).unwrap()
    }

    fn make_state(verifier: MockOAuthVerifier) -> ServerState {
        ServerState {
            fxa_email_domain: "test.com".to_owned(),
            fxa_metrics_hash_secret: "".to_owned(),
            oauth_verifier: Box::new(verifier),
            db_pool: Box::new(MockTokenserverPool::new()),
            node_capacity_release_rate: None,
            node_type: NodeType::default(),
        }
    }
}
