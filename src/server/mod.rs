//! Main application server

use std::sync::Arc;

use actix::{System, SystemRunner};
use actix_web::{http, middleware::cors::Cors, server::HttpServer, App, HttpResponse};
//use num_cpus;
use serde_json::json;

use crate::db::{mysql::MysqlDbPool, spanner::SpannerDbPool, DbError, DbPool};
use crate::settings::{Secrets, ServerLimits, Settings};
use crate::web::handlers;
use crate::web::middleware;

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
        .prefix("/1.5")
        .middleware(middleware::WeaveTimestamp)
        .middleware(middleware::DbTransaction)
        .middleware(middleware::PreConditionCheck)
        .configure(|app| {
            Cors::for_app(app)
                .resource("/{uid}/info/collections", |r| {
                    r.method(http::Method::GET).with(handlers::get_collections);
                })
                .resource("/{uid}/info/collection_counts", |r| {
                    r.method(http::Method::GET)
                        .with(handlers::get_collection_counts);
                })
                .resource("/{uid}/info/collection_usage", |r| {
                    r.method(http::Method::GET)
                        .with(handlers::get_collection_usage);
                })
                .resource("/{uid}/info/configuration", |r| {
                    r.method(http::Method::GET)
                        .with(handlers::get_configuration);
                })
                .resource("/{uid}/info/quota", |r| {
                    r.method(http::Method::GET).with(handlers::get_quota);
                })
                .resource("/{uid}", |r| {
                    r.method(http::Method::DELETE).with(handlers::delete_all);
                })
                .resource("/{uid}/storage", |r| {
                    r.method(http::Method::DELETE).with(handlers::delete_all);
                })
                .resource("/{uid}/storage/{collection}", |r| {
                    r.method(http::Method::DELETE)
                        .with(handlers::delete_collection);
                    r.method(http::Method::GET).with(handlers::get_collection);
                    r.method(http::Method::POST).with(handlers::post_collection);
                })
                .resource("/{uid}/storage/{collection}/{bso}", |r| {
                    r.method(http::Method::DELETE).with(handlers::delete_bso);
                    r.method(http::Method::GET).with(handlers::get_bso);
                    r.method(http::Method::PUT).with(handlers::put_bso);
                })
                .register()
        })
}

pub fn build_dockerflow(state: ServerState) -> App<ServerState> {
    App::with_state(state)
        // Handle the resource that don't need to go through middleware
        .resource("/__heartbeat__", |r| {
            // if addidtional information is desired, point to an appropriate handler.
            r.method(http::Method::GET).f(|_| {
                let body = json!({"status": "ok", "version": env!("CARGO_PKG_VERSION")});
                HttpResponse::Ok()
                    .content_type("application/json")
                    .body(body.to_string())
            });
        })
        .resource("/__lbheartbeat__", |r| {
            // used by the load balancers, just return OK.
            r.method(http::Method::GET).f(|_| {
                HttpResponse::Ok()
                    .content_type("application/json")
                    .body("{}")
            });
        })
        .resource("/__version__", |r| {
            // return the contents of the version.json file created by circleci and stored in the docker root
            r.method(http::Method::GET).f(|_| {
                HttpResponse::Ok()
                    .content_type("application/json")
                    .body(include_str!("../../version.json"))
            });
        })
}

pub struct Server {}

impl Server {
    pub fn with_settings(settings: Settings) -> Result<SystemRunner, DbError> {
        let sys = System::new("syncserver");
        let db_pool = Box::new(SpannerDbPool::new(&settings)?);
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

            let dfstate = ServerState {
                db_pool: db_pool.clone(),
                limits: Arc::clone(&limits),
                secrets: Arc::clone(&secrets),
                port,
            };

            vec![build_app(state), build_dockerflow(dfstate)]
        })
        .bind(format!("{}:{}", settings.host, settings.port))
        .unwrap()
        .start();
        Ok(sys)
    }
}
