pub mod db;
pub mod error;
pub mod extractors;
pub mod handlers;
pub mod settings;
pub mod support;

pub use self::support::{MockOAuthVerifier, OAuthVerifier, TestModeOAuthVerifier, VerifyToken};

use db::pool::{DbPool, TokenserverPool};
use settings::Settings;

use crate::error::ApiError;

#[derive(Clone)]
pub struct ServerState {
    pub db_pool: Box<dyn DbPool>,
    pub fxa_email_domain: String,
    pub fxa_metrics_hash_secret: String,
    pub oauth_verifier: Box<dyn VerifyToken>,
    pub node_capacity_release_rate: Option<f32>,
}

impl ServerState {
    pub fn from_settings(settings: &Settings) -> Result<Self, ApiError> {
        let oauth_verifier: Box<dyn VerifyToken> = if settings.test_mode_enabled {
            #[cfg(debug_assertions)]
            let oauth_verifier = Box::new(TestModeOAuthVerifier);

            #[cfg(not(debug_assertions))]
            let oauth_verifier = Box::new(OAuthVerifier {
                fxa_oauth_server_url: settings.fxa_oauth_server_url.clone(),
            });

            oauth_verifier
        } else {
            Box::new(OAuthVerifier {
                fxa_oauth_server_url: settings.fxa_oauth_server_url.clone(),
            })
        };
        let use_test_transactions = false;

        TokenserverPool::new(settings, use_test_transactions)
            .map(|db_pool| ServerState {
                fxa_email_domain: settings.fxa_email_domain.clone(),
                fxa_metrics_hash_secret: settings.fxa_metrics_hash_secret.clone(),
                oauth_verifier,
                db_pool: Box::new(db_pool),
                node_capacity_release_rate: settings.node_capacity_release_rate,
            })
            .map_err(Into::into)
    }
}
