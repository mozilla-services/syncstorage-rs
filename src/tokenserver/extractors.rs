//! Request header/body/query extractors
//!
//! Handles ensuring the header's, body, and query parameters are correct, extraction to
//! relevant types, and failing correctly with the appropriate errors if issues arise.

use actix_web::{dev::Payload, web::Data, Error, FromRequest, HttpRequest};
use actix_web_httpauth::extractors::bearer::BearerAuth;

use futures::future::LocalBoxFuture;

use super::support::TokenData;
use crate::server::ServerState;
use crate::web::error::ValidationErrorKind;
use crate::web::extractors::RequestErrorLocation;

/// Information from the request needed to process a Tokenserver request.
pub struct TokenserverRequest {
    pub fxa_uid: String,
    pub generation: i64,
}

impl From<TokenData> for TokenserverRequest {
    fn from(token_data: TokenData) -> Self {
        Self {
            fxa_uid: token_data.user,
            generation: token_data.generation,
        }
    }
}

impl FromRequest for TokenserverRequest {
    type Config = ();
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let req = req.clone();
        let mut payload = payload.take();

        Box::pin(async move {
            let token_data = TokenData::from_request(&req, &mut payload).await?;

            Ok(token_data.into())
        })
    }
}

impl FromRequest for TokenData {
    type Config = ();
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let req = req.clone();
        let mut payload = payload.take();

        Box::pin(async move {
            let auth = BearerAuth::from_request(&req, &mut payload).await?;
            let state = match req.app_data::<Data<ServerState>>() {
                Some(s) => s,
                None => {
                    error!("⚠️ Could not load the app state");
                    return Err(ValidationErrorKind::FromDetails(
                        "Internal error".to_owned(),
                        RequestErrorLocation::Unknown,
                        Some("state".to_owned()),
                        None,
                    )
                    .into());
                }
            };
            state.tokenserver_oauth_verifier.verify_token(auth.token())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use actix_web::{http::Method, test::TestRequest, HttpResponse};
    use lazy_static::lazy_static;
    use tokio::sync::RwLock;

    use crate::db::mock::MockDbPool;
    use crate::server::{metrics, ServerState};
    use crate::settings::{Deadman, Secrets, ServerLimits, Settings};
    use crate::tokenserver::MockOAuthVerifier;

    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    lazy_static! {
        static ref SECRETS: Arc<Secrets> = Arc::new(Secrets::new("Ted Koppel is a robot").unwrap());
        static ref SERVER_LIMITS: Arc<ServerLimits> = Arc::new(ServerLimits::default());
    }

    const TOKENSERVER_PATH: &str = "/1.0/sync/1.5";

    #[actix_rt::test]
    async fn test_valid_tokenserver_request() {
        let fxa_uid = "test123";
        let verifier = {
            let start = SystemTime::now();
            let current_time = start.duration_since(UNIX_EPOCH).unwrap();
            let token_data = TokenData {
                user: fxa_uid.to_owned(),
                client_id: "client id".to_owned(),
                scope: vec!["scope".to_owned()],
                generation: current_time.as_secs() as i64,
                profile_changed_at: current_time.as_secs() as i64,
            };
            let valid = true;

            MockOAuthVerifier { valid, token_data }
        };
        let state = make_state(verifier);

        let req = TestRequest::with_uri(TOKENSERVER_PATH)
            .data(state)
            .header("authorization", "Bearer fake_token")
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
        let fxa_uid = "test123";
        let verifier = {
            let start = SystemTime::now();
            let current_time = start.duration_since(UNIX_EPOCH).unwrap();
            let token_data = TokenData {
                user: fxa_uid.to_owned(),
                client_id: "client id".to_owned(),
                scope: vec!["scope".to_owned()],
                generation: current_time.as_secs() as i64,
                profile_changed_at: current_time.as_secs() as i64,
            };
            let valid = false;

            MockOAuthVerifier { valid, token_data }
        };
        let state = make_state(verifier);

        let req = TestRequest::with_uri(TOKENSERVER_PATH)
            .data(state)
            .header("authorization", "Bearer fake_token")
            .header("accept", "application/json,text/plain:q=0.5")
            .method(Method::GET)
            .to_http_request();

        let mut payload = Payload::None;
        let result = TokenserverRequest::from_request(&req, &mut payload).await;
        assert!(result.is_err());

        let response: HttpResponse = result.err().unwrap().into();
        assert_eq!(response.status(), 400);
    }

    fn make_state(verifier: MockOAuthVerifier) -> ServerState {
        let settings = Settings::default();

        ServerState {
            db_pool: Box::new(MockDbPool::new()),
            limits: Arc::clone(&SERVER_LIMITS),
            limits_json: serde_json::to_string(&**SERVER_LIMITS).unwrap(),
            secrets: Arc::clone(&SECRETS),
            tokenserver_database_url: None,
            fxa_metrics_hash_secret: None,
            tokenserver_oauth_verifier: Box::new(verifier),
            port: 8000,
            metrics: Box::new(metrics::metrics_from_opts(&settings).unwrap()),
            quota_enabled: settings.enable_quota,
            deadman: Arc::new(RwLock::new(Deadman::default())),
        }
    }
}
