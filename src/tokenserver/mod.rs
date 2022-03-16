pub mod db;
pub mod error;
pub mod extractors;
pub mod handlers;
pub mod logging;
pub mod settings;
pub mod support;

pub use self::support::{MockOAuthVerifier, OAuthVerifier, TestModeOAuthVerifier, VerifyToken};

use actix_web::{dev::RequestHead, http::header::USER_AGENT, HttpRequest};
use cadence::StatsdClient;
use db::{
    params,
    pool::{DbPool, TokenserverPool},
};
use serde::{
    ser::{SerializeMap, Serializer},
    Deserialize, Serialize,
};
use settings::Settings;

use crate::error::ApiError;
use crate::server::{metrics::Metrics, user_agent};

use std::sync::Arc;
use std::{collections::HashMap, fmt};

#[derive(Clone)]
pub struct ServerState {
    pub db_pool: Box<dyn DbPool>,
    pub fxa_email_domain: String,
    pub fxa_metrics_hash_secret: String,
    pub oauth_verifier: Box<dyn VerifyToken>,
    pub node_capacity_release_rate: Option<f32>,
    pub node_type: NodeType,
    pub service_id: Option<i32>,
    pub metrics: Arc<StatsdClient>,
}

impl ServerState {
    pub fn from_settings(settings: &Settings, metrics: StatsdClient) -> Result<Self, ApiError> {
        let oauth_verifier: Box<dyn VerifyToken> = if settings.test_mode_enabled {
            #[cfg(feature = "tokenserver_test_mode")]
            let oauth_verifier = Box::new(TestModeOAuthVerifier);

            #[cfg(not(feature = "tokenserver_test_mode"))]
            let oauth_verifier = Box::new(
                OAuthVerifier::new(settings.fxa_oauth_server_url.as_deref())
                    .expect("failed to create Tokenserver OAuth verifier"),
            );

            oauth_verifier
        } else {
            Box::new(
                OAuthVerifier::new(settings.fxa_oauth_server_url.as_deref())
                    .expect("failed to create Tokenserver OAuth verifier"),
            )
        };
        let use_test_transactions = false;
        let ametrics = Arc::new(metrics);

        TokenserverPool::new(
            settings,
            Arc::new(Metrics::from(ametrics.clone())),
            use_test_transactions,
        )
        .map(|db_pool| {
            let service_id = db_pool
                .get_sync()
                .and_then(|db| {
                    db.get_service_id_sync(params::GetServiceId {
                        service: "sync-1.5".to_owned(),
                    })
                })
                .ok()
                .map(|result| result.id);

            ServerState {
                fxa_email_domain: settings.fxa_email_domain.clone(),
                fxa_metrics_hash_secret: settings.fxa_metrics_hash_secret.clone(),
                oauth_verifier,
                db_pool: Box::new(db_pool),
                node_capacity_release_rate: settings.node_capacity_release_rate,
                node_type: settings.node_type,
                metrics: ametrics,
                service_id,
            }
        })
        .map_err(Into::into)
    }
}

pub struct TokenserverMetrics(Metrics);

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

#[derive(Clone, Debug)]
struct LogItems(HashMap<String, String>);

impl LogItems {
    fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn insert(&mut self, k: String, v: String) {
        self.0.insert(k, v);
    }

    fn insert_if_not_empty(&mut self, k: &str, v: &str) {
        if !v.is_empty() {
            self.0.insert(k.to_owned(), v.to_owned());
        }
    }
}

impl From<&RequestHead> for LogItems {
    fn from(req_head: &RequestHead) -> Self {
        let mut items = Self::new();
        if let Some(ua) = req_head.headers().get(USER_AGENT) {
            if let Ok(uas) = ua.to_str() {
                let (ua_result, metrics_os, metrics_browser) = user_agent::parse_user_agent(uas);
                items.insert_if_not_empty("ua.os.family", metrics_os);
                items.insert_if_not_empty("ua.browser.family", metrics_browser);
                items.insert_if_not_empty("ua.name", ua_result.name);
                items.insert_if_not_empty("ua.os.ver", &ua_result.os_version.to_owned());
                items.insert_if_not_empty("ua.browser.ver", ua_result.version);
                items.insert_if_not_empty("ua", ua_result.version);
            }
        }
        items.insert("uri.method".to_owned(), req_head.method.to_string());
        items.insert("uri.path".to_owned(), req_head.uri.to_string());

        items
    }
}

impl<'a> IntoIterator for &'a LogItems {
    type Item = (&'a String, &'a String);
    type IntoIter = <&'a HashMap<String, String> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl Serialize for LogItems {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_map(Some(self.0.len()))?;
        for item in self {
            if !item.1.is_empty() {
                seq.serialize_entry(&item.0, &item.1)?;
            }
        }
        seq.end()
    }
}

impl fmt::Display for LogItems {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            serde_json::to_string(&self).map_err(|_| fmt::Error)?
        )
    }
}

struct LogItemsMutator<'a>(&'a HttpRequest);

impl<'a> LogItemsMutator<'a> {
    pub fn insert(&mut self, k: String, v: String) {
        let mut exts = self.0.extensions_mut();

        if !exts.contains::<LogItems>() {
            exts.insert(LogItems::from(self.0.head()));
        }

        let log_items = exts.get_mut::<LogItems>().unwrap();

        log_items.insert(k, v);
    }
}

impl<'a> From<&'a HttpRequest> for LogItemsMutator<'a> {
    fn from(req: &'a HttpRequest) -> Self {
        Self(req)
    }
}
