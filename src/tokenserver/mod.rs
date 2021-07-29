pub mod db;
pub mod extractors;
pub mod handlers;
pub mod settings;
pub mod support;

pub use self::support::{MockOAuthVerifier, OAuthVerifier, VerifyToken};

use db::pool::{DbPool, TokenserverPool};
use settings::Settings;

use crate::error::ApiError;

#[derive(Clone)]
pub struct ServerState {
    pub db_pool: Box<dyn DbPool>,
    pub fxa_email_domain: String,
    pub fxa_metrics_hash_secret: String,
    pub oauth_verifier: Box<dyn VerifyToken>,
}

impl ServerState {
    pub fn from_settings(settings: &Settings) -> Result<Self, ApiError> {
        let oauth_verifier = OAuthVerifier {
            fxa_oauth_server_url: settings.fxa_oauth_server_url.clone(),
        };
        let use_test_transactions = false;

        TokenserverPool::new(settings, use_test_transactions)
            .map(|db_pool| ServerState {
                fxa_email_domain: settings.fxa_email_domain.clone(),
                fxa_metrics_hash_secret: settings.fxa_metrics_hash_secret.clone(),
                oauth_verifier: Box::new(oauth_verifier),
                db_pool: Box::new(db_pool),
            })
            .map_err(Into::into)
    }
}
