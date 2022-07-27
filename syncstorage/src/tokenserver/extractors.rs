//! Request header/body/query extractors
//!
//! Handles ensuring the header's, body, and query parameters are correct, extraction to
//! relevant types, and failing correctly with the appropriate errors if issues arise.

use core::fmt::Debug;
use std::sync::Arc;

use actix_web::{
    dev::Payload,
    http::StatusCode,
    web::{Data, Query},
    FromRequest, HttpRequest,
};
use futures::future::{self, LocalBoxFuture, Ready};
use hex;
use hmac::{Hmac, Mac, NewMac};
use lazy_static::lazy_static;
use regex::Regex;
use serde::Deserialize;
use sha2::Sha256;
use tokenserver_common::error::{ErrorLocation, TokenserverError};

use super::{
    db::{models::Db, params, pool::DbPool, results},
    LogItemsMutator, NodeType, ServerState, TokenserverMetrics,
};
use crate::{server::metrics, settings::Secrets, web::tags::Tags};

lazy_static! {
    static ref CLIENT_STATE_REGEX: Regex = Regex::new("^[a-zA-Z0-9._-]{1,32}$").unwrap();
}

const DEFAULT_TOKEN_DURATION: u64 = 5 * 60;
const SYNC_SERVICE_NAME: &str = "sync-1.5";

/// Information from the request needed to process a Tokenserver request.
#[derive(Debug, Default, PartialEq)]
pub struct TokenserverRequest {
    pub user: results::GetOrCreateUser,
    pub auth_data: AuthData,
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
        let auth_keys_changed_at = self.auth_data.keys_changed_at;
        let auth_generation = self.auth_data.generation;
        let user_keys_changed_at = self.user.keys_changed_at;
        let user_generation = Some(self.user.generation);

        /// `$left` and `$right` must both be `Option`s, and `$op` must be a binary infix
        /// operator. If `$left` and `$right` are both `Some`, this macro returns
        /// `$left $op $right`; otherwise, it returns `false`.
        macro_rules! opt_cmp {
            ($left:ident $op:tt $right:ident) => {
                $left.zip($right).map(|(l, r)| l $op r).unwrap_or(false)
            }
        }

        // If the caller reports a generation number, then a change
        // in keys should correspond to a change in generation number.
        // Unfortunately a previous version of the server that didn't
        // have `keys_changed_at` support may have already seen and
        // written the new value of `generation`. The best we can do
        // here is enforce that `keys_changed_at` <= `generation`.
        if opt_cmp!(auth_keys_changed_at > user_keys_changed_at)
            && opt_cmp!(auth_generation < auth_keys_changed_at)
        {
            return Err(TokenserverError {
                context: "keys_changed_at greater than generation".to_owned(),
                ..TokenserverError::invalid_keys_changed_at()
            });
        }

        // The client state on the request must not have been used in the past.
        if self
            .user
            .old_client_states
            .contains(&self.auth_data.client_state)
        {
            let error_message = "Unacceptable client-state value stale value".to_owned();
            return Err(TokenserverError::invalid_client_state(error_message));
        }

        // If the client state on the request differs from the most recently-used client state, it must
        // be accompanied by a valid change in generation (if the client reports a generation).
        if self.auth_data.client_state != self.user.client_state
            && opt_cmp!(auth_generation <= user_generation)
        {
            let error_message =
                "Unacceptable client-state value new value with no generation change".to_owned();
            return Err(TokenserverError::invalid_client_state(error_message));
        }

        // If the client state on the request differs from the most recently-used client state, it must
        // be accompanied by a valid change in keys_changed_at
        if self.auth_data.client_state != self.user.client_state
            && opt_cmp!(auth_keys_changed_at <= user_keys_changed_at)
        {
            let error_message =
                "Unacceptable client-state value new value with no keys_changed_at change"
                    .to_owned();
            return Err(TokenserverError::invalid_client_state(error_message));
        }

        // The generation on the request cannot be earlier than the generation stored on the user
        // record.
        if opt_cmp!(user_generation > auth_generation) {
            return Err(TokenserverError {
                context: "New generation less than previously-seen generation".to_owned(),
                ..TokenserverError::invalid_generation()
            });
        }

        // The keys_changed_at on the request cannot be earlier than the keys_changed_at stored on
        // the user record.
        if opt_cmp!(user_keys_changed_at > auth_keys_changed_at) {
            return Err(TokenserverError {
                context: "New keys_changed_at less than previously-seen keys_changed_at".to_owned(),
                ..TokenserverError::invalid_keys_changed_at()
            });
        }

        Ok(())
    }
}

impl FromRequest for TokenserverRequest {
    type Config = ();
    type Error = TokenserverError;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let req = req.clone();

        Box::pin(async move {
            let mut log_items_mutator = LogItemsMutator::from(&req);
            let auth_data = AuthData::extract(&req).await?;

            // XXX: Tokenserver state will no longer be an Option once the Tokenserver
            // code is rolled out, so we will eventually be able to remove this unwrap().
            let state = get_server_state(&req)?.as_ref().as_ref().unwrap();
            let shared_secret = get_secret(&req)?;
            let fxa_metrics_hash_secret = &state.fxa_metrics_hash_secret.as_bytes();

            // To preserve anonymity, compute a hash of the FxA UID to be used for reporting
            // metrics
            let hashed_fxa_uid = {
                let hashed_fxa_uid_full =
                    fxa_metrics_hash(&auth_data.fxa_uid, fxa_metrics_hash_secret);
                log_items_mutator.insert("uid".to_owned(), hashed_fxa_uid_full.clone());
                hashed_fxa_uid_full[0..32].to_owned()
            };
            log_items_mutator.insert("metrics_uid".to_owned(), hashed_fxa_uid.clone());

            // To preserve anonymity, compute a hash of the FxA device ID to be used for reporting
            // metrics. Only requests using BrowserID will have a device ID, so use "none" as
            // a placeholder for OAuth requests.
            let hashed_device_id = {
                let device_id = auth_data.device_id.as_deref().unwrap_or("none");
                hash_device_id(&hashed_fxa_uid, device_id, fxa_metrics_hash_secret)
            };

            let db = <Box<dyn Db>>::extract(&req).await?;
            let service_id = {
                let path = req.match_info();

                // If we've reached this extractor, we know that the Tokenserver path was matched,
                // meaning "application" and "version" are both present in the URL. So, we can use
                // `unwrap()` here.
                let application = path.get("application").unwrap();
                let version = path.get("version").unwrap();

                if application == "sync" {
                    if version == "1.5" {
                        db.get_service_id(params::GetServiceId {
                            service: SYNC_SERVICE_NAME.to_owned(),
                        })
                        .await?
                        .id
                    } else {
                        return Err(TokenserverError::unsupported(
                            "Unsupported application version".to_owned(),
                            version.to_owned(),
                        ));
                    }
                } else {
                    // NOTE: It would probably be better to include the name of the unsupported
                    // application in the error message, but the old Tokenserver only includes
                    // "application" in the error message. To keep the APIs between the old and
                    // new Tokenservers as close as possible, we defer to the error message from
                    // the old Tokenserver.
                    return Err(TokenserverError::unsupported(
                        "Unsupported application".to_owned(),
                        "application".to_owned(),
                    ));
                }
            };
            let user = db
                .get_or_create_user(params::GetOrCreateUser {
                    service_id,
                    email: auth_data.email.clone(),
                    generation: auth_data.generation.unwrap_or(0),
                    client_state: auth_data.client_state.clone(),
                    keys_changed_at: auth_data.keys_changed_at,
                    capacity_release_rate: state.node_capacity_release_rate,
                })
                .await?;
            log_items_mutator.insert("first_seen_at".to_owned(), user.first_seen_at.to_string());

            let duration = {
                let params =
                    Query::<QueryParams>::extract(&req)
                        .await
                        .map_err(|_| TokenserverError {
                            description: "invalid query params".to_owned(),
                            context: "invalid query params".to_owned(),
                            http_status: StatusCode::BAD_REQUEST,
                            location: ErrorLocation::Url,
                            ..Default::default()
                        })?;

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
                auth_data,
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
    type Error = TokenserverError;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let req = req.clone();

        Box::pin(async move {
            <Box<dyn DbPool>>::extract(&req)
                .await?
                .get()
                .await
                .map_err(|e| TokenserverError {
                    context: format!("Couldn't acquire a database connection: {}", e),
                    ..TokenserverError::internal_error()
                })
        })
    }
}

impl FromRequest for Box<dyn DbPool> {
    type Config = ();
    type Error = TokenserverError;
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

/// An authentication token as parsed from the `Authorization` header. Both BrowserID assertions
/// and OAuth tokens are opaque to Tokenserver and must be verified via FxA.
pub enum Token {
    BrowserIdAssertion(String),
    OAuthToken(String),
}

impl FromRequest for Token {
    type Config = ();
    type Error = TokenserverError;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let req = req.clone();

        Box::pin(async move {
            // The request must have a valid Authorization header
            let authorization_header = req
                .headers()
                .get("Authorization")
                .ok_or_else(|| TokenserverError {
                    description: "Unauthorized".to_owned(),
                    location: ErrorLocation::Body,
                    context: "No Authorization header".to_owned(),
                    ..Default::default()
                })?
                .to_str()
                .map_err(|e| TokenserverError {
                    description: "Unauthorized".to_owned(),
                    location: ErrorLocation::Body,
                    context: format!(
                        "Authorization header contains invalid ASCII characters: {}",
                        e
                    ),
                    ..Default::default()
                })?;

            if let Some((auth_type, token)) = authorization_header.split_once(' ') {
                let auth_type = auth_type.to_ascii_lowercase();

                if auth_type == "bearer" {
                    Ok(Token::OAuthToken(token.to_owned()))
                } else if auth_type == "browserid" {
                    Ok(Token::BrowserIdAssertion(token.to_owned()))
                } else {
                    // The request must use a Bearer token or BrowserID token
                    Err(TokenserverError {
                        description: "Unsupported".to_owned(),
                        location: ErrorLocation::Body,
                        context: "Invalid authorization scheme".to_owned(),
                        ..Default::default()
                    })
                }
            } else {
                // Headers that are not of the format "[AUTH TYPE] [TOKEN]" are invalid
                Err(TokenserverError {
                    description: "Unauthorized".to_owned(),
                    location: ErrorLocation::Body,
                    context: "Invalid Authorization header format".to_owned(),
                    ..Default::default()
                })
            }
        })
    }
}

/// The data extracted from the authentication token.
#[derive(Debug, Default, PartialEq)]
pub struct AuthData {
    pub client_state: String,
    pub device_id: Option<String>,
    pub email: String,
    pub fxa_uid: String,
    pub generation: Option<i64>,
    pub keys_changed_at: Option<i64>,
}

impl FromRequest for AuthData {
    type Config = ();
    type Error = TokenserverError;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let req = req.clone();

        Box::pin(async move {
            // XXX: The Tokenserver state will no longer be an Option once the Tokenserver
            // code is rolled out, so we will eventually be able to remove this unwrap().
            let state = get_server_state(&req)?.as_ref().as_ref().unwrap();
            let token = Token::extract(&req).await?;

            let TokenserverMetrics(mut metrics) = TokenserverMetrics::extract(&req).await?;

            match token {
                Token::BrowserIdAssertion(assertion) => {
                    let mut tags = Tags::default();
                    tags.tags
                        .insert("token_type".to_owned(), "BrowserID".to_owned());
                    metrics.start_timer("token_verification", Some(tags));
                    let verify_output = state.browserid_verifier.verify(assertion).await?;

                    // For requests using BrowserID, the client state is embedded in the
                    // X-Client-State header, and the generation and keys_changed_at are extracted
                    // from the assertion as part of the verification process.
                    let XClientStateHeader(client_state) =
                        XClientStateHeader::extract(&req).await?;
                    let (fxa_uid, _) = verify_output
                        .email
                        .split_once('@')
                        .unwrap_or((&verify_output.email, ""));

                    Ok(AuthData {
                        client_state: client_state.unwrap_or_else(|| "".to_owned()),
                        device_id: verify_output.device_id,
                        email: verify_output.email.clone(),
                        fxa_uid: fxa_uid.to_owned(),
                        generation: verify_output.generation,
                        keys_changed_at: verify_output.keys_changed_at,
                    })
                }
                Token::OAuthToken(token) => {
                    let mut tags = Tags::default();
                    tags.tags
                        .insert("token_type".to_owned(), "OAuth".to_owned());
                    metrics.start_timer("token_verification", Some(tags));
                    let verify_output = state.oauth_verifier.verify(token).await?;

                    // For requests using OAuth, the keys_changed_at and client state are embedded
                    // in the X-KeyID header.
                    let key_id = KeyId::extract(&req).await?;
                    let fxa_uid = verify_output.fxa_uid;
                    let email = format!("{}@{}", fxa_uid, state.fxa_email_domain);

                    Ok(AuthData {
                        client_state: key_id.client_state,
                        email,
                        device_id: None,
                        fxa_uid,
                        generation: verify_output.generation,
                        keys_changed_at: Some(key_id.keys_changed_at),
                    })
                }
            }
        })
    }
}

/// The value extracted from the X-Client-State header if it was present. The value in this header
/// consists of the raw client state bytes encoded as a hexadecimal string.
struct XClientStateHeader(Option<String>);

impl FromRequest for XClientStateHeader {
    type Config = ();
    type Error = TokenserverError;
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
                        description: "Invalid client state value".to_owned(),
                        name: "X-Client-State".to_owned(),
                        http_status: StatusCode::BAD_REQUEST,
                        context: "Invalid client state value".to_owned(),
                        ..Default::default()
                    });
                }
            }

            Ok(Self(maybe_x_client_state.map(ToOwned::to_owned)))
        })
    }
}

// The key ID, as extracted from the X-KeyID header. The X-KeyID header is of the format
// `[keys_changed_at]-[base64-encoded client state]` (e.g. `00000000000001234-qqo`)
#[derive(Clone, Debug, PartialEq)]
struct KeyId {
    client_state: String,
    keys_changed_at: i64,
}

impl FromRequest for KeyId {
    type Config = ();
    type Error = TokenserverError;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let req = req.clone();

        Box::pin(async move {
            let headers = req.headers();

            // The X-KeyID header must be present for requests using OAuth
            let x_key_id = headers
                .get("X-KeyID")
                .ok_or_else(|| {
                    TokenserverError::invalid_key_id("Missing X-KeyID header".to_owned())
                })?
                .to_str()
                .map_err(|_| {
                    TokenserverError::invalid_key_id("Invalid X-KeyID header".to_owned())
                })?;

            // The X-KeyID header is of the format `[keys_changed_at]-[base64-encoded client state]` (e.g. `00000000000001234-qqo`)
            let (keys_changed_at_string, encoded_client_state) =
                x_key_id.split_once('-').ok_or_else(|| TokenserverError {
                    context: "X-KeyID header has invalid format".to_owned(),
                    ..TokenserverError::invalid_credentials("Unauthorized".to_owned())
                })?;

            let client_state = {
                // The client state in the X-KeyID header consists of the raw client state bytes
                // encoded as URL-safe base64 with the padding removed. We convert it to hex
                // because we store the client state as hex in the database.
                let client_state_hex = {
                    let bytes =
                        base64::decode_config(encoded_client_state, base64::URL_SAFE_NO_PAD)
                            .map_err(|e| TokenserverError {
                                context: format!(
                                    "Failed to decode client state base64 in X-KeyID: {}",
                                    e
                                ),
                                ..TokenserverError::invalid_credentials("Unauthorized".to_owned())
                            })?;

                    hex::encode(bytes)
                };
                // The client state from the X-Client-State header is already properly encoded as
                // hex
                let XClientStateHeader(x_client_state) = XClientStateHeader::extract(&req).await?;

                // If there's a client state value in the X-Client-State header, verify that it matches
                // the value in X-KeyID.
                if let Some(x_client_state) = x_client_state {
                    if x_client_state != client_state_hex {
                        return Err(TokenserverError {
                            status: "invalid-client-state",
                            location: ErrorLocation::Body,
                            context: "Client state mismatch in X-Client-State header".to_owned(),
                            ..TokenserverError::default()
                        });
                    }
                }

                client_state_hex
            };

            let keys_changed_at =
                keys_changed_at_string
                    .parse::<i64>()
                    .map_err(|e| TokenserverError {
                        context: format!("Non-integral keys_changed_at in X-KeyID: {}", e),
                        ..TokenserverError::invalid_credentials("Unauthorized".to_owned())
                    })?;

            Ok(KeyId {
                client_state,
                keys_changed_at,
            })
        })
    }
}

impl FromRequest for TokenserverMetrics {
    type Config = ();
    type Error = TokenserverError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let state = match get_server_state(req) {
            // XXX: Tokenserver state will no longer be an Option once the Tokenserver
            // code is rolled out, so we will eventually be able to remove this unwrap().
            Ok(state) => state.as_ref().as_ref().unwrap(),
            Err(e) => return future::err(e),
        };

        future::ok(TokenserverMetrics(metrics::metrics_from_request(
            req,
            Some(state.metrics.clone()),
        )))
    }
}

fn get_server_state(req: &HttpRequest) -> Result<&Data<Option<ServerState>>, TokenserverError> {
    req.app_data::<Data<Option<ServerState>>>()
        .ok_or_else(|| TokenserverError {
            context: "Failed to load the application state".to_owned(),
            ..TokenserverError::internal_error()
        })
}

fn get_secret(req: &HttpRequest) -> Result<String, TokenserverError> {
    let secrets = req
        .app_data::<Data<Arc<Secrets>>>()
        .ok_or_else(|| TokenserverError {
            context: "Failed to load the application secrets".to_owned(),
            ..TokenserverError::internal_error()
        })?;

    String::from_utf8(secrets.master_secret.clone()).map_err(|e| TokenserverError {
        context: format!("Failed to read the master secret: {}", e),
        ..TokenserverError::internal_error()
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

    use crate::server::metrics;
    use crate::settings::{Secrets, ServerLimits, Settings};
    use crate::tokenserver::{
        auth::{browserid, oauth, MockVerifier},
        db::mock::MockDbPool as MockTokenserverPool,
        ServerState,
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
        let oauth_verifier = {
            let verify_output = oauth::VerifyOutput {
                fxa_uid: fxa_uid.to_owned(),
                generation: Some(1234),
            };
            let valid = true;

            MockVerifier {
                valid,
                verify_output,
            }
        };
        let state = make_state(oauth_verifier, MockVerifier::default());

        let req = TestRequest::default()
            .data(Some(state))
            .data(Arc::clone(&SECRETS))
            .header("authorization", "Bearer fake_token")
            .header("accept", "application/json,text/plain:q=0.5")
            .header("x-keyid", "0000000001234-qqo")
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
            auth_data: AuthData {
                device_id: None,
                fxa_uid: fxa_uid.to_owned(),
                email: "test123@test.com".to_owned(),
                generation: Some(1234),
                keys_changed_at: Some(1234),
                client_state: "aaaa".to_owned(),
            },
            shared_secret: "Ted Koppel is a robot".to_owned(),
            hashed_fxa_uid: "4d00ecae64b98dd7dc7dea68d0dd615d".to_owned(),
            hashed_device_id: "3a41cccbdd666ebc4199f1f9d1249d44".to_owned(),
            service_id: i32::default(),
            duration: 100,
            node_type: NodeType::default(),
        };

        assert_eq!(result, expected_tokenserver_request);
    }

    #[actix_rt::test]
    async fn test_invalid_auth_token() {
        let fxa_uid = "test123";
        let oauth_verifier = {
            let verify_output = oauth::VerifyOutput {
                fxa_uid: fxa_uid.to_owned(),
                generation: Some(1234),
            };
            let valid = false;

            MockVerifier {
                valid,
                verify_output,
            }
        };
        let state = make_state(oauth_verifier, MockVerifier::default());

        let request = TestRequest::default()
            .data(Some(state))
            .data(Arc::clone(&SECRETS))
            .header("authorization", "Bearer fake_token")
            .header("accept", "application/json,text/plain:q=0.5")
            .header("x-keyid", "0000000001234-qqo")
            .param("application", "sync")
            .param("version", "1.5")
            .method(Method::GET)
            .to_http_request();

        let response: HttpResponse = TokenserverRequest::extract(&request)
            .await
            .unwrap_err()
            .into();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let expected_error = TokenserverError::invalid_credentials("Unauthorized".to_owned());
        let body = extract_body_as_str(ServiceResponse::new(request, response));
        assert_eq!(body, serde_json::to_string(&expected_error).unwrap());
    }

    #[actix_rt::test]
    async fn test_application_and_version() {
        fn build_request() -> TestRequest {
            let fxa_uid = "test123";
            let oauth_verifier = {
                let verify_output = oauth::VerifyOutput {
                    fxa_uid: fxa_uid.to_owned(),
                    generation: Some(1234),
                };
                let valid = true;

                MockVerifier {
                    valid,
                    verify_output,
                }
            };

            TestRequest::default()
                .data(Some(make_state(oauth_verifier, MockVerifier::default())))
                .data(Arc::clone(&SECRETS))
                .header("authorization", "Bearer fake_token")
                .header("accept", "application/json,text/plain:q=0.5")
                .header("x-keyid", "0000000001234-qqo")
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

            let expected_error = TokenserverError::unsupported(
                "Unsupported application version".to_owned(),
                "1.0".to_owned(),
            );
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

            let expected_error = TokenserverError::unsupported(
                "Unsupported application".to_owned(),
                "application".to_owned(),
            );
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

            let expected_error = TokenserverError::unsupported(
                "Unsupported application".to_owned(),
                "application".to_owned(),
            );
            let body = extract_body_as_str(ServiceResponse::new(request, response));
            assert_eq!(body, serde_json::to_string(&expected_error).unwrap());
        }

        // Valid application and valid version
        {
            let request = build_request()
                .param("application", "sync")
                .param("version", "1.5")
                .to_http_request();

            assert!(TokenserverRequest::extract(&request).await.is_ok());
        }
    }

    #[actix_rt::test]
    async fn test_key_id() {
        fn build_request() -> TestRequest {
            let fxa_uid = "test123";
            let oauth_verifier = {
                let start = SystemTime::now();
                let current_time = start.duration_since(UNIX_EPOCH).unwrap();
                let verify_output = oauth::VerifyOutput {
                    fxa_uid: fxa_uid.to_owned(),
                    generation: Some(current_time.as_secs() as i64),
                };
                let valid = true;

                MockVerifier {
                    valid,
                    verify_output,
                }
            };

            TestRequest::default()
                .data(Some(make_state(oauth_verifier, MockVerifier::default())))
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

            let expected_error =
                TokenserverError::invalid_key_id("Missing X-KeyID header".to_owned());
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

            let expected_error =
                TokenserverError::invalid_key_id("Invalid X-KeyID header".to_owned());
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

            let expected_error = TokenserverError::invalid_credentials("Unauthorized".to_owned());
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

            let expected_error = TokenserverError::invalid_credentials("Unauthorized".to_owned());
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

            let expected_error =
                TokenserverError::invalid_key_id("Invalid X-KeyID header".to_owned());
            let body = extract_body_as_str(ServiceResponse::new(request, response));
            assert_eq!(body, serde_json::to_string(&expected_error).unwrap());
        }

        // X-KeyID header with non-integral keys_changed_at
        {
            let request = build_request()
                .header("x-keyid", "notanumber-qqo")
                .to_http_request();
            let response: HttpResponse = KeyId::extract(&request).await.unwrap_err().into();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

            let expected_error = TokenserverError::invalid_credentials("Unauthorized".to_owned());
            let body = extract_body_as_str(ServiceResponse::new(request, response));
            assert_eq!(body, serde_json::to_string(&expected_error).unwrap());
        }

        // X-KeyID header with client state that does not match that in the X-Client-State header
        {
            let request = build_request()
                .header("x-keyid", "0000000001234-qqo")
                .header("x-client-state", "bbbb")
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
                .header("x-keyid", "0000000001234-qqo")
                .header("x-client-state", "aaaa")
                .to_http_request();
            let key_id = KeyId::extract(&request).await.unwrap();
            let expected_key_id = KeyId {
                client_state: "aaaa".to_owned(),
                keys_changed_at: 1234,
            };

            assert_eq!(key_id, expected_key_id);
        }

        // Valid X-KeyID header with no X-Client-State header
        {
            let request = build_request()
                .header("x-keyid", "0000000001234-qqo")
                .to_http_request();
            let key_id = KeyId::extract(&request).await.unwrap();
            let expected_key_id = KeyId {
                client_state: "aaaa".to_owned(),
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
                client_state: "aaaa".to_owned(),
                generation: 1234,
                node: "node".to_owned(),
                keys_changed_at: Some(1234),
                replaced_at: None,
                created_at: 1234,
                first_seen_at: 1234,
                old_client_states: vec![],
            },
            auth_data: AuthData {
                device_id: None,
                fxa_uid: "test".to_owned(),
                email: "test@test.com".to_owned(),
                generation: Some(1233),
                keys_changed_at: Some(1234),
                client_state: "aaaa".to_owned(),
            },
            shared_secret: "secret".to_owned(),
            hashed_fxa_uid: "abcdef".to_owned(),
            hashed_device_id: "abcdef".to_owned(),
            service_id: 1,
            duration: DEFAULT_TOKEN_DURATION,
            node_type: NodeType::default(),
        };

        let error = tokenserver_request.validate().unwrap_err();
        assert_eq!(
            error,
            TokenserverError {
                context: "New generation less than previously-seen generation".to_owned(),
                ..TokenserverError::invalid_generation()
            }
        );
    }

    #[actix_rt::test]
    async fn test_old_keys_changed_at() {
        // The request includes a keys_changed_at that is less than the keys_changed_at currently
        // stored on the user record
        let tokenserver_request = TokenserverRequest {
            user: results::GetOrCreateUser {
                uid: 1,
                email: "test@test.com".to_owned(),
                client_state: "aaaa".to_owned(),
                generation: 1234,
                node: "node".to_owned(),
                keys_changed_at: Some(1234),
                created_at: 1234,
                first_seen_at: 1234,
                replaced_at: None,
                old_client_states: vec![],
            },
            auth_data: AuthData {
                device_id: None,
                fxa_uid: "test".to_owned(),
                email: "test@test.com".to_owned(),
                generation: Some(1234),
                keys_changed_at: Some(1233),
                client_state: "aaaa".to_owned(),
            },
            shared_secret: "secret".to_owned(),
            hashed_fxa_uid: "abcdef".to_owned(),
            hashed_device_id: "abcdef".to_owned(),
            service_id: 1,
            duration: DEFAULT_TOKEN_DURATION,
            node_type: NodeType::default(),
        };

        let error = tokenserver_request.validate().unwrap_err();
        assert_eq!(
            error,
            TokenserverError {
                context: "New keys_changed_at less than previously-seen keys_changed_at".to_owned(),
                ..TokenserverError::invalid_keys_changed_at()
            }
        );
    }

    #[actix_rt::test]
    async fn test_keys_changed_without_generation_change() {
        // The request includes a new value for keys_changed_at without a new value for generation
        let tokenserver_request = TokenserverRequest {
            user: results::GetOrCreateUser {
                uid: 1,
                email: "test@test.com".to_owned(),
                client_state: "aaaa".to_owned(),
                generation: 1234,
                node: "node".to_owned(),
                keys_changed_at: Some(1234),
                created_at: 1234,
                first_seen_at: 1234,
                replaced_at: None,
                old_client_states: vec![],
            },
            auth_data: AuthData {
                device_id: None,
                fxa_uid: "test".to_owned(),
                email: "test@test.com".to_owned(),
                generation: Some(1234),
                keys_changed_at: Some(1235),
                client_state: "aaaa".to_owned(),
            },
            shared_secret: "secret".to_owned(),
            hashed_fxa_uid: "abcdef".to_owned(),
            hashed_device_id: "abcdef".to_owned(),
            service_id: 1,
            duration: DEFAULT_TOKEN_DURATION,
            node_type: NodeType::default(),
        };

        let error = tokenserver_request.validate().unwrap_err();
        assert_eq!(
            error,
            TokenserverError {
                context: "keys_changed_at greater than generation".to_owned(),
                ..TokenserverError::invalid_keys_changed_at()
            }
        );
    }

    #[actix_rt::test]
    async fn test_old_client_state() {
        // The request includes a previously-used client state that is not the user's current
        // client state
        let tokenserver_request = TokenserverRequest {
            user: results::GetOrCreateUser {
                uid: 1,
                email: "test@test.com".to_owned(),
                client_state: "aaaa".to_owned(),
                generation: 1234,
                node: "node".to_owned(),
                keys_changed_at: Some(1234),
                created_at: 1234,
                first_seen_at: 1234,
                replaced_at: None,
                old_client_states: vec!["bbbb".to_owned()],
            },
            auth_data: AuthData {
                device_id: None,
                fxa_uid: "test".to_owned(),
                email: "test@test.com".to_owned(),
                generation: Some(1234),
                keys_changed_at: Some(1234),
                client_state: "bbbb".to_owned(),
            },
            shared_secret: "secret".to_owned(),
            hashed_fxa_uid: "abcdef".to_owned(),
            hashed_device_id: "abcdef".to_owned(),
            service_id: 1,
            duration: DEFAULT_TOKEN_DURATION,
            node_type: NodeType::default(),
        };

        let error = tokenserver_request.validate().unwrap_err();
        let error_message = "Unacceptable client-state value stale value".to_owned();
        assert_eq!(error, TokenserverError::invalid_client_state(error_message));
    }

    #[actix_rt::test]
    async fn test_new_client_state_without_generation_change() {
        // The request includes a new client state without a new generation value
        let tokenserver_request = TokenserverRequest {
            user: results::GetOrCreateUser {
                uid: 1,
                email: "test@test.com".to_owned(),
                client_state: "aaaa".to_owned(),
                generation: 1234,
                node: "node".to_owned(),
                keys_changed_at: Some(1234),
                created_at: 1234,
                first_seen_at: 1234,
                replaced_at: None,
                old_client_states: vec![],
            },
            auth_data: AuthData {
                device_id: None,
                fxa_uid: "test".to_owned(),
                email: "test@test.com".to_owned(),
                generation: Some(1234),
                keys_changed_at: Some(1234),
                client_state: "bbbb".to_owned(),
            },
            shared_secret: "secret".to_owned(),
            hashed_fxa_uid: "abcdef".to_owned(),
            hashed_device_id: "abcdef".to_owned(),
            service_id: 1,
            duration: DEFAULT_TOKEN_DURATION,
            node_type: NodeType::default(),
        };

        let error = tokenserver_request.validate().unwrap_err();
        let error_message =
            "Unacceptable client-state value new value with no generation change".to_owned();
        assert_eq!(error, TokenserverError::invalid_client_state(error_message));
    }

    #[actix_rt::test]
    async fn test_new_client_state_without_key_change() {
        // The request includes a new client state without a new keys_changed_at value
        let tokenserver_request = TokenserverRequest {
            user: results::GetOrCreateUser {
                uid: 1,
                email: "test@test.com".to_owned(),
                client_state: "aaaa".to_owned(),
                generation: 1234,
                node: "node".to_owned(),
                keys_changed_at: Some(1234),
                created_at: 1234,
                first_seen_at: 1234,
                replaced_at: None,
                old_client_states: vec![],
            },
            auth_data: AuthData {
                device_id: None,
                fxa_uid: "test".to_owned(),
                email: "test@test.com".to_owned(),
                generation: Some(1235),
                keys_changed_at: Some(1234),
                client_state: "bbbb".to_owned(),
            },
            shared_secret: "secret".to_owned(),
            hashed_fxa_uid: "abcdef".to_owned(),
            hashed_device_id: "abcdef".to_owned(),
            service_id: 1,
            duration: DEFAULT_TOKEN_DURATION,
            node_type: NodeType::default(),
        };

        let error = tokenserver_request.validate().unwrap_err();
        let error_message =
            "Unacceptable client-state value new value with no keys_changed_at change".to_owned();
        assert_eq!(error, TokenserverError::invalid_client_state(error_message));
    }

    fn extract_body_as_str(sresponse: ServiceResponse) -> String {
        String::from_utf8(block_on(test::read_body(sresponse)).to_vec()).unwrap()
    }

    fn make_state(
        oauth_verifier: MockVerifier<oauth::VerifyOutput>,
        browserid_verifier: MockVerifier<browserid::VerifyOutput>,
    ) -> ServerState {
        let settings = Settings::default();

        ServerState {
            fxa_email_domain: "test.com".to_owned(),
            fxa_metrics_hash_secret: "".to_owned(),
            browserid_verifier: Box::new(browserid_verifier),
            oauth_verifier: Box::new(oauth_verifier),
            db_pool: Box::new(MockTokenserverPool::new()),
            node_capacity_release_rate: None,
            node_type: NodeType::default(),
            metrics: Box::new(
                metrics::metrics_from_opts(
                    &settings.tokenserver.statsd_label,
                    settings.statsd_host.as_deref(),
                    settings.statsd_port,
                )
                .unwrap(),
            ),
        }
    }
}
