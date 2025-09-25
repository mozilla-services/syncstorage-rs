#[allow(clippy::result_large_err)]
pub mod extractors;
#[allow(clippy::result_large_err)]
pub mod handlers;
pub mod logging;

use actix_web::{dev::RequestHead, http::header::USER_AGENT, HttpMessage, HttpRequest};
use cadence::StatsdClient;
use serde::{
    ser::{SerializeMap, Serializer},
    Serialize,
};
use syncserver_common::{BlockingThreadpool, Metrics};
#[cfg(not(feature = "py_verifier"))]
use tokenserver_auth::JWTVerifierImpl;
use tokenserver_auth::{oauth, VerifyToken};
use tokenserver_common::NodeType;
use tokenserver_db::{pool_from_settings, DbPool};
use tokenserver_settings::Settings;

use crate::{error::ApiError, server::user_agent};

use std::{collections::HashMap, fmt, sync::Arc};

#[derive(Clone)]
pub struct ServerState {
    pub db_pool: Box<dyn DbPool>,
    pub fxa_email_domain: String,
    pub fxa_metrics_hash_secret: String,
    pub oauth_verifier: Box<dyn VerifyToken<Output = oauth::VerifyOutput>>,
    pub node_capacity_release_rate: Option<f32>,
    pub node_type: NodeType,
    pub metrics: Arc<StatsdClient>,
    pub token_duration: u64,
}

impl ServerState {
    pub fn from_settings(
        settings: &Settings,
        metrics: Arc<StatsdClient>,
        #[allow(unused_variables)] blocking_threadpool: Arc<BlockingThreadpool>,
    ) -> Result<Self, ApiError> {
        #[cfg(not(feature = "py_verifier"))]
        let oauth_verifier = {
            let mut jwk_verifiers: Vec<JWTVerifierImpl> = Vec::new();
            if let Some(primary) = &settings.fxa_oauth_primary_jwk {
                jwk_verifiers.push(
                    primary
                        .clone()
                        .try_into()
                        .expect("Invalid primary key, should either be fixed or removed"),
                )
            }
            if let Some(secondary) = &settings.fxa_oauth_secondary_jwk {
                jwk_verifiers.push(
                    secondary
                        .clone()
                        .try_into()
                        .expect("Invalid secondary key, should either be fixed or removed"),
                );
            }
            Box::new(
                oauth::Verifier::new(settings, jwk_verifiers)
                    .expect("failed to create Tokenserver OAuth verifier"),
            )
        };

        #[cfg(feature = "py_verifier")]
        let oauth_verifier = Box::new(
            oauth::Verifier::new(settings, blocking_threadpool.clone())
                .expect("failed to create Tokenserver OAuth verifier"),
        );
        let use_test_transactions = false;

        let db_pool = pool_from_settings(settings, &Metrics::from(&metrics), use_test_transactions)
            .expect("Failed to create Tokenserver pool");
        Ok(ServerState {
            fxa_email_domain: settings.fxa_email_domain.clone(),
            fxa_metrics_hash_secret: settings.fxa_metrics_hash_secret.clone(),
            oauth_verifier,
            db_pool,
            node_capacity_release_rate: settings.node_capacity_release_rate,
            node_type: settings.node_type,
            metrics,
            token_duration: settings.token_duration,
        })
    }

    /// Initialize the db_pool: run migrations, etc.
    pub async fn init(&mut self) {
        self.db_pool
            .init()
            .await
            .expect("Failed to init Tokenserver pool");
    }
}

pub struct TokenserverMetrics(Metrics);

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
                items.insert_if_not_empty("ua.os.ver", &ua_result.os_version);
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

impl LogItemsMutator<'_> {
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
