//! Request header/body/query extractors
//!
//! Handles ensuring the header's, body, and query parameters are correct, extraction to
//! relevant types, and failing correctly with the appropriate errors if issues arise.

use actix_web::{dev::Payload, http::{HeaderMap, HeaderValue}, web::Data, Error, FromRequest, HttpRequest};
use actix_web_httpauth::extractors::bearer::BearerAuth;

use futures::future::LocalBoxFuture;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};

use crate::server::ServerState;
use crate::web::error::ValidationErrorKind;
use crate::web::extractors::RequestErrorLocation;

// TODO: These claims came from a JWT dumped from the tokenserver request made
// by my local firefox browser after clicking sync. It would be good to find
// documentation about this somewhere to ensure that this is the correct format
#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
struct Claims {
    pub aud: String,
    pub iss: String,
    #[serde(rename = "fxa-profileChangedAt")]
    pub profile_changed_at: i64,
    pub jti: String,
    pub exp: i64,
    pub scope: String,
    pub client_id: String,
    pub iat: i64,
    #[serde(rename = "fxa-generation")]
    pub generation: i64,
    pub sub: String,
}
pub struct TokenserverRequest {
    pub fxa_uid: String,
    pub generation: i64,
    pub keys_changed_at: i64,
    pub client_state: Vec<u8>,
}

impl TokenserverRequest {
    fn extract_claims(
        jwt: &str,
        rsa_modulus: String,
        rsa_exponent: String,
    ) -> Result<Claims, Error> {
        decode::<Claims>(
            &jwt,
            &DecodingKey::from_rsa_components(&rsa_modulus, &rsa_exponent),
            &Validation::new(Algorithm::RS256),
        )
        .map(|token_data| token_data.claims)
        .map_err(|e| {
            ValidationErrorKind::FromDetails(
                format!("Unable to decode token JWT: {:?}", e),
                RequestErrorLocation::Header,
                Some("Bearer".to_owned()),
                label!("request.error.invalid_bearer_auth"),
            )
            .into()
        })
    }

    fn extract_header(headers: &HeaderMap, name: &str) -> Result<String, Error> {
        // headers.get(name).ok_or(Err("header does not exist"))?.to_str().map(ToOwned::to_owned)
        headers.get(name).ok_or("header does not exist")
        .and_then(|h: &HeaderValue| h.to_str())
        .map(ToOwned::to_owned)
    }

    // TODO we might want to put this in a util.rs file eventually. Same with
    // the functions at the bottom of handlers.rs
    fn parse_key_id(key_id: String) -> Result<(i64, Vec<u8>), Error> {
        const ERROR_MESSAGE: &str = "Invalid X-KeyID header";
        const KEY_ID_DELIMITER: &str = "-";

        let components = key_id.split(KEY_ID_DELIMITER);
        // TODO we need to add correct error handling here
        let keys_changed_at = components.next().ok_or(ERROR_MESSAGE).and_then(str::parse::<i64>)?;
        let key_hash = components.next().ok_or(ERROR_MESSAGE).and_then(|encoded_hash| {
            base64::decode_config(encoded_hash, base64::URL_SAFE_NO_PAD)
        })?;

        Ok((keys_changed_at, key_hash))
    }
}

/// Extracts data from the JWT in the Authorization header
impl FromRequest for TokenserverRequest {
    type Config = ();
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let req = req.clone();
        let mut payload = payload.take();

        Box::pin(async move {
            let state = match req.app_data::<Data<ServerState>>() {
                Some(s) => s,
                None => {
                    error!("⚠️ Could not load the app state");
                    return Err(ValidationErrorKind::FromDetails(
                        "Internal error".to_owned(),
                        RequestErrorLocation::Unknown,
                        Some("app_data".to_owned()),
                        None,
                    )
                    .into());
                }
            };
            let auth = BearerAuth::from_request(&req, &mut payload).await?;
            let claims = {
                let rsa_modulus = state.tokenserver_jwks_rsa_modulus.clone().ok_or_else(|| {
                    error!("⚠️ Tokenserver JWK RSA modulus not set");
                    ValidationErrorKind::FromDetails(
                        "Internal error".to_owned(),
                        RequestErrorLocation::Unknown,
                        Some("app_data".to_owned()),
                        None,
                    )
                })?;
                let rsa_exponent =
                    state.tokenserver_jwks_rsa_exponent.clone().ok_or_else(|| {
                        error!("⚠️ Tokenserver JWK RSA exponent not set");
                        ValidationErrorKind::FromDetails(
                            "Internal error".to_owned(),
                            RequestErrorLocation::Unknown,
                            Some("app_data".to_owned()),
                            None,
                        )
                    })?;
                // todo should this be another extractor?
                Self::extract_claims(auth.token(), rsa_modulus, rsa_exponent)?
            };

            let key_id = Self::extract_header(req.headers(), "X-KeyID")?;
            let (keys_changed_at, client_state) = Self::parse_key_id(key_id)?;

            Ok(Self {
                fxa_uid: claims.sub,
                generation: claims.generation,
                keys_changed_at,
                client_state,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use actix_web::{http::Method, test::TestRequest, HttpResponse};
    use jsonwebtoken::{encode, EncodingKey, Header};
    use lazy_static::lazy_static;
    use openssl::{pkey::Private, rsa::Rsa};
    use tokio::sync::RwLock;

    use crate::db::mock::MockDbPool;
    use crate::server::{metrics, ServerState};
    use crate::settings::{Deadman, Secrets, ServerLimits, Settings};

    use std::sync::Arc;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    lazy_static! {
        static ref SECRETS: Arc<Secrets> = Arc::new(Secrets::new("Ted Koppel is a robot").unwrap());
        static ref SERVER_LIMITS: Arc<ServerLimits> = Arc::new(ServerLimits::default());
    }

    const SECONDS_IN_A_YEAR: u64 = 60 * 60 * 24 * 365;
    const TOKENSERVER_PATH: &str = "/1.0/sync/1.5";

    #[actix_rt::test]
    async fn test_valid_tokenserver_request() {
        let rsa = Rsa::generate(2048).unwrap();
        let state = make_state(&rsa);
        let fxa_uid = "test123";
        let generation = 5;

        let bearer_token = {
            let start = SystemTime::now();
            let current_time = start.duration_since(UNIX_EPOCH).unwrap();
            let exp_duration = current_time + Duration::new(SECONDS_IN_A_YEAR, 0);
            let claims = Claims {
                aud: Default::default(),
                iat: current_time.as_secs() as i64,
                exp: exp_duration.as_secs() as i64,
                iss: Default::default(),
                profile_changed_at: Default::default(),
                jti: Default::default(),
                scope: Default::default(),
                client_id: Default::default(),
                sub: fxa_uid.to_owned(),
                generation,
            };

            encode::<Claims>(
                &Header::new(Algorithm::RS256),
                &claims,
                &EncodingKey::from_rsa_pem(&rsa.private_key_to_pem().unwrap()).unwrap(),
            )
            .unwrap()
        };

        let req = TestRequest::with_uri(TOKENSERVER_PATH)
            .data(state)
            .header("authorization", format!("Bearer {}", bearer_token))
            .header("accept", "application/json,text/plain:q=0.5")
            .method(Method::GET)
            .to_http_request();

        let mut payload = Payload::None;
        let result = TokenserverRequest::from_request(&req, &mut payload)
            .await
            .unwrap();

        assert_eq!(result.fxa_uid, fxa_uid);
    }

    #[actix_rt::test]
    async fn test_invalid_tokenserver_request() {
        let rsa = Rsa::generate(2048).unwrap();
        let state = make_state(&rsa);
        let bearer_token = "I am not a valid token";

        let req = TestRequest::with_uri(TOKENSERVER_PATH)
            .data(state)
            .header("authorization", format!("Bearer {}", bearer_token))
            .header("accept", "application/json,text/plain:q=0.5")
            .method(Method::GET)
            .to_http_request();

        let mut payload = Payload::None;
        let result = TokenserverRequest::from_request(&req, &mut payload).await;
        assert!(result.is_err());

        let response: HttpResponse = result.err().unwrap().into();
        assert_eq!(response.status(), 400);
    }

    fn make_state(rsa: &Rsa<Private>) -> ServerState {
        let settings = Settings::default();
        let modulus = base64::encode_config(rsa.n().to_vec(), base64::URL_SAFE_NO_PAD);
        let public_exponent = base64::encode_config(rsa.e().to_vec(), base64::URL_SAFE_NO_PAD);

        ServerState {
            db_pool: Box::new(MockDbPool::new()),
            limits: Arc::clone(&SERVER_LIMITS),
            limits_json: serde_json::to_string(&**SERVER_LIMITS).unwrap(),
            secrets: Arc::clone(&SECRETS),
            tokenserver_database_url: None,
            tokenserver_jwks_rsa_modulus: Some(modulus),
            tokenserver_jwks_rsa_exponent: Some(public_exponent),
            fxa_metrics_hash_secret: None,
            port: 8000,
            metrics: Box::new(metrics::metrics_from_opts(&settings).unwrap()),
            quota_enabled: settings.enable_quota,
            deadman: Arc::new(RwLock::new(Deadman::default())),
        }
    }
}
