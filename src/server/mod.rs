//! Main application server

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use actix::{SyncArbiter, System, SystemRunner};
use actix_web::{http, middleware::cors::Cors, server::HttpServer, App};
use num_cpus;

use db::models::DBManager;
use dispatcher::DBExecutor;
use handlers;
use handlers::ServerState;
use settings::Settings;

macro_rules! init_routes {
    ($app:expr) => {
        $app.resource("{uid}/info/collections", |r| {
            r.method(http::Method::GET).with(handlers::collections);
        }).resource("{uid}/info/collection_counts", |r| {
                r.method(http::Method::GET)
                    .with(handlers::collection_counts);
            })
            .resource("{uid}/info/collection_usage", |r| {
                r.method(http::Method::GET).with(handlers::collection_usage);
            })
            .resource("{uid}/info/configuration", |r| {
                r.method(http::Method::GET).with(handlers::configuration);
            })
            .resource("{uid}/info/quota", |r| {
                r.method(http::Method::GET).with(handlers::quota);
            })
            .resource("{uid}/storage/{collection}", |r| {
                r.method(http::Method::DELETE)
                    .with(handlers::delete_collection);
                r.method(http::Method::GET).with(handlers::get_collection);
                r.method(http::Method::POST).with(handlers::post_collection);
            })
            .resource("{uid}/storage/{collection}/{bso}", |r| {
                r.method(http::Method::DELETE).with(handlers::delete_bso);
                r.method(http::Method::GET).with(handlers::get_bso);
                r.method(http::Method::PUT).with(handlers::put_bso);
            })
    };
}

// The tests depend on the init_routes! macro, so this mod must come after it
#[cfg(test)]
mod test;

pub struct Server {}

impl Server {
    pub fn with_settings(settings: &Settings) -> SystemRunner {
        let sys = System::new("syncserver");

        // Start dispatcher with the arbiter
        let db_pool = Arc::new(RwLock::new(HashMap::new()));
        let db_executor = SyncArbiter::start(num_cpus::get(), move || DBExecutor {
            db_handles: db_pool.clone(),
        });

        HttpServer::new(move || {
            // Setup the server state
            let state = ServerState {
                db_executor: db_executor.clone(),
            };

            App::with_state(state).configure(|app| init_routes!(Cors::for_app(app)).register())
        }).bind(format!("127.0.0.1:{}", settings.port))
            .unwrap()
            .start();
        sys
    }
}
