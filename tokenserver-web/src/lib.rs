#[macro_use]
extern crate slog_scope;

mod error;
pub mod extractors;
pub mod handlers;
pub mod middleware;

use std::{collections::HashMap, convert::TryFrom, fmt, sync::Arc, time::Duration};

use actix_web::{
    dev::RequestHead,
    http::header::USER_AGENT,
    web::{self, ServiceConfig},
    HttpRequest, HttpResponse,
};
use cadence::StatsdClient;
use serde::{
    ser::{SerializeMap, Serializer},
    Serialize,
};
use syncserver_common::{BlockingThreadpool, Metrics};
use syncserver_web_common::user_agent;
use tokenserver_auth::{browserid, oauth, VerifyToken};
use tokenserver_common::{NodeType, TokenserverError};
use tokenserver_db::{params, DbPool, TokenserverPool};
use tokenserver_settings::Settings;

pub use error::ApiError;
pub use extractors::HeartbeatResponse;

pub fn get_configurator<'a>(
    settings: &'a Settings,
    statsd_host: Option<&'a str>,
    statsd_port: u16,
    blocking_threadpool: Arc<BlockingThreadpool>,
) -> Result<impl FnOnce(&mut ServiceConfig) + 'a, ApiError> {
    let statsd_client = syncserver_common::statsd_client_from_opts(
        &settings.statsd_label,
        statsd_host,
        statsd_port,
    )
    .map_err(|e| TokenserverError {
        context: e.to_string(),
        ..TokenserverError::internal_error()
    })?;
    let state = ServerState::from_settings(settings, statsd_client.clone(), blocking_threadpool)?;

    Ok(move |cfg: &mut ServiceConfig| {
        syncserver_db_common::spawn_pool_periodic_reporter(
            Duration::from_secs(10),
            statsd_client,
            state.db_pool.clone(),
        );

        cfg.data(state)
            .service(
                web::resource("/1.0/{application}/{version}")
                    .route(web::get().to(handlers::get_tokenserver_result)),
            )
            .service(web::resource("/__heartbeat__").route(web::get().to(handlers::heartbeat)))
            .service(
                web::resource("/__lbheartbeat__").route(web::get().to(|_: HttpRequest| {
                    // used by the load balancers, just return OK.
                    HttpResponse::Ok()
                        .content_type("application/json")
                        .body("{}")
                })),
            );
    })
}

#[derive(Clone)]
pub struct ServerState {
    pub db_pool: Box<dyn DbPool>,
    pub fxa_email_domain: String,
    pub fxa_metrics_hash_secret: String,
    pub oauth_verifier: Box<dyn VerifyToken<Output = oauth::VerifyOutput>>,
    pub browserid_verifier: Box<dyn VerifyToken<Output = browserid::VerifyOutput>>,
    pub node_capacity_release_rate: Option<f32>,
    pub node_type: NodeType,
    pub statsd_client: Box<StatsdClient>,
    pub token_duration: u64,
}

impl ServerState {
    pub fn from_settings(
        settings: &Settings,
        statsd_client: StatsdClient,
        blocking_threadpool: Arc<BlockingThreadpool>,
    ) -> Result<Self, ApiError> {
        let oauth_verifier = Box::new(
            oauth::Verifier::new(settings, blocking_threadpool.clone())
                .expect("failed to create Tokenserver OAuth verifier"),
        );
        let browserid_verifier = Box::new(
            browserid::Verifier::try_from(settings)
                .expect("failed to create Tokenserver BrowserID verifier"),
        );
        let use_test_transactions = false;

        TokenserverPool::new(
            settings,
            &Metrics::from(&statsd_client),
            blocking_threadpool,
            use_test_transactions,
        )
        .map(|mut db_pool| {
            // NOTE: Provided there's a "sync-1.5" service record in the database, it is highly
            // unlikely for this query to fail outside of network failures or other random
            // errors
            db_pool.service_id = db_pool
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
                browserid_verifier,
                db_pool: Box::new(db_pool),
                node_capacity_release_rate: settings.node_capacity_release_rate,
                node_type: settings.node_type,
                statsd_client: Box::new(statsd_client),
                token_duration: settings.token_duration,
            }
        })
        .map_err(|_| {
            TokenserverError {
                description: "Failed to create Tokenserver pool".to_owned(),
                context: "Failed to create Tokenserver pool".to_owned(),
                ..TokenserverError::internal_error()
            }
            .into()
        })
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
