//! Main application server

use std::sync::Arc;

use actix::{System, SystemRunner};
use actix_web::{http, middleware::cors::Cors, server::HttpServer, App};
//use num_cpus;

use db::{mock::MockDb, Db};
use settings::{Secrets, ServerLimits, Settings};
use web::handlers;
use web::middleware;

macro_rules! init_routes {
    ($app:expr) => {
        $app.resource("/{uid}/info/collections", |r| {
            r.method(http::Method::GET).with(handlers::get_collections);
        }).resource("/{uid}/info/collection_counts", |r| {
            r.method(http::Method::GET)
                .with(handlers::get_collection_counts);
        }).resource("/{uid}/info/collection_usage", |r| {
            r.method(http::Method::GET)
                .with(handlers::get_collection_usage);
        }).resource("/{uid}/info/configuration", |r| {
            r.method(http::Method::GET)
                .with(handlers::get_configuration);
        }).resource("/{uid}/info/quota", |r| {
            r.method(http::Method::GET).with(handlers::get_quota);
        }).resource("/{uid}", |r| {
            r.method(http::Method::DELETE).with(handlers::delete_all);
        }).resource("/{uid}/storage", |r| {
            r.method(http::Method::DELETE).with(handlers::delete_all);
        }).resource("/{uid}/storage/{collection}", |r| {
            r.method(http::Method::DELETE)
                .with(handlers::delete_collection);
            r.method(http::Method::GET).with(handlers::get_collection);
            r.method(http::Method::POST).with(handlers::post_collection);
        }).resource("/{uid}/storage/{collection}/{bso}", |r| {
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
    pub db: Box<Db>,

    /// Server-enforced limits for request payloads.
    pub limits: Arc<ServerLimits>,

    /// Secrets used during Hawk authentication.
    pub secrets: Arc<Secrets>,
}

pub struct Server {}

impl Server {
    pub fn with_settings(settings: Settings) -> SystemRunner {
        let sys = System::new("syncserver");
        let limits = Arc::new(settings.limits);
        let secrets = Arc::new(settings.master_secret);

        HttpServer::new(move || {
            // Setup the server state
            let state = ServerState {
                // TODO: replace MockDb with a real implementation
                db: Box::new(MockDb::new()),
                limits: Arc::clone(&limits),
                secrets: Arc::clone(&secrets),
            };

            App::with_state(state)
                .middleware(middleware::WeaveTimestamp)
                .configure(|app| init_routes!(Cors::for_app(app)).register())
        }).bind(format!("127.0.0.1:{}", settings.port))
        .unwrap()
        .start();
        sys
    }
}
