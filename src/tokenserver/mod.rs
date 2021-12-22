pub mod db;
pub mod error;
pub mod extractors;
pub mod handlers;
pub mod settings;
pub mod support;

pub use self::support::{MockOAuthVerifier, OAuthVerifier, TestModeOAuthVerifier, VerifyToken};

use db::{
    params,
    pool::{DbPool, TokenserverPool},
};
use serde::{Deserialize, Serialize};
use settings::Settings;

use crate::error::ApiError;

#[derive(Clone)]
pub struct ServerState {
    pub db_pool: Box<dyn DbPool>,
    pub fxa_email_domain: String,
    pub fxa_metrics_hash_secret: String,
    pub oauth_verifier: Box<dyn VerifyToken>,
    pub node_capacity_release_rate: Option<f32>,
    pub node_type: NodeType,
    pub service_id: Option<i32>,
}

impl ServerState {
    pub fn from_settings(settings: &Settings) -> Result<Self, ApiError> {
        let oauth_verifier: Box<dyn VerifyToken> = if settings.test_mode_enabled {
            #[cfg(feature = "tokenserver_test_mode")]
            let oauth_verifier = Box::new(TestModeOAuthVerifier);

            #[cfg(not(feature = "tokenserver_test_mode"))]
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
            .and_then(|db_pool| {
                let service_id = db_pool
                    .get_sync()
                    .and_then(|db| {
                        db.get_service_id_sync(params::GetServiceId {
                            service: "sync-1.5".to_owned(),
                        })
                    })
                    .ok()
                    .map(|result| result.id);

                Ok(ServerState {
                    fxa_email_domain: settings.fxa_email_domain.clone(),
                    fxa_metrics_hash_secret: settings.fxa_metrics_hash_secret.clone(),
                    oauth_verifier,
                    db_pool: Box::new(db_pool),
                    node_capacity_release_rate: settings.node_capacity_release_rate,
                    node_type: settings.node_type,
                    service_id,
                })
            })
            .map_err(Into::into)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum NodeType {
    #[serde(rename = "mysql")]
    MySql,
    #[serde(rename = "spanner")]
    Spanner,
}

impl Default for NodeType {
    fn default() -> Self {
        Self::Spanner
    }
}
