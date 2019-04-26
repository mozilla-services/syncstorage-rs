//! Main application server

use std::sync::Arc;

use actix::{System, SystemRunner};
use actix_web::{http, middleware::cors::Cors, server::HttpServer, App};
//use num_cpus;

use db::{mysql::MysqlDbPool, DbError, DbPool};
use settings::{Secrets, ServerLimits, Settings};
use web::handlers;
use web::middleware;

macro_rules! init_routes {
    ($app:expr) => {
        $app.resource("/1.5/{uid}/info/collections", |r| {
            r.method(http::Method::GET).with(handlers::get_collections);
        })
        .resource("/1.5/{uid}/info/collection_counts", |r| {
            r.method(http::Method::GET)
                .with(handlers::get_collection_counts);
        })
        .resource("/1.5/{uid}/info/collection_usage", |r| {
            r.method(http::Method::GET)
                .with(handlers::get_collection_usage);
        })
        .resource("/1.5/{uid}/info/configuration", |r| {
            r.method(http::Method::GET)
                .with(handlers::get_configuration);
        })
        .resource("/1.5/{uid}/info/quota", |r| {
            r.method(http::Method::GET).with(handlers::get_quota);
        })
        .resource("/1.5/{uid}", |r| {
            r.method(http::Method::DELETE).with(handlers::delete_all);
        })
        .resource("/1.5/{uid}/storage", |r| {
            r.method(http::Method::DELETE).with(handlers::delete_all);
        })
        .resource("/1.5/{uid}/storage/{collection}", |r| {
            r.method(http::Method::DELETE)
                .with(handlers::delete_collection);
            r.method(http::Method::GET).with(handlers::get_collection);
            r.method(http::Method::POST).with(handlers::post_collection);
        })
        .resource("/1.5/{uid}/storage/{collection}/{bso}", |r| {
            r.method(http::Method::DELETE).with(handlers::delete_bso);
            r.method(http::Method::GET).with(handlers::get_bso);
            r.method(http::Method::PUT).with(handlers::put_bso);
        })
    };
}

// The tests depend on the init_routes! macro, so this mod must come after it
#[cfg(test)]
mod test;

/// This is the global HTTP state object that will be made available to all
/// HTTP API calls.
pub struct ServerState {
    pub db_pool: Box<dyn DbPool>,

    /// Server-enforced limits for request payloads.
    pub limits: Arc<ServerLimits>,

    /// Secrets used during Hawk authentication.
    pub secrets: Arc<Secrets>,

    pub port: u16,
}

pub fn build_app(state: ServerState) -> App<ServerState> {
    App::with_state(state)
        .middleware(middleware::WeaveTimestamp)
        .middleware(middleware::DbTransaction)
        .middleware(middleware::PreConditionCheck)
        .configure(|app| init_routes!(Cors::for_app(app)).register())
}

pub struct Server {}

impl Server {
    pub fn with_settings(settings: Settings) -> Result<SystemRunner, DbError> {
        let sys = System::new("syncserver");
        let db_pool = Box::new(MysqlDbPool::new(&settings)?);
        let limits = Arc::new(settings.limits);
        let secrets = Arc::new(settings.master_secret);
        let port = settings.port;

        HttpServer::new(move || {
            // Setup the server state
            let state = ServerState {
                db_pool: db_pool.clone(),
                limits: Arc::clone(&limits),
                secrets: Arc::clone(&secrets),
                port,
            };

            build_app(state)
        })
        .bind(format!("127.0.0.1:{}", settings.port))
        .unwrap()
        .start();
        Ok(sys)
    }
}
