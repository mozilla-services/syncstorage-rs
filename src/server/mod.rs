// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, you can obtain one at https://mozilla.org/MPL/2.0/.

//! Main application server

use actix::{System, SystemRunner};
use actix_web::{http, middleware::cors::Cors, server::HttpServer, App};
//use num_cpus;

use db::mock::MockDb;
use handlers::{self, ServerState};
use settings::Settings;

macro_rules! init_routes {
    ($app:expr) => {
        $app.resource("{uid}/info/collections", |r| {
            r.method(http::Method::GET).with(handlers::get_collections);
        }).resource("{uid}/info/collection_counts", |r| {
                r.method(http::Method::GET)
                    .with(handlers::get_collection_counts);
            })
            .resource("{uid}/info/collection_usage", |r| {
                r.method(http::Method::GET)
                    .with(handlers::get_collection_usage);
            })
            .resource("{uid}/info/configuration", |r| {
                r.method(http::Method::GET)
                    .with(handlers::get_configuration);
            })
            .resource("{uid}/info/quota", |r| {
                r.method(http::Method::GET).with(handlers::get_quota);
            })
            .resource("{uid}", |r| {
                r.method(http::Method::DELETE).with(handlers::delete_all);
            })
            .resource("{uid}/storage", |r| {
                r.method(http::Method::DELETE).with(handlers::delete_all);
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

        HttpServer::new(move || {
            // Setup the server state
            let state = ServerState {
                // TODO: replace MockDb with a real implementation
                db: Box::new(MockDb::new()),
            };

            App::with_state(state).configure(|app| init_routes!(Cors::for_app(app)).register())
        }).bind(format!("127.0.0.1:{}", settings.port))
            .unwrap()
            .start();
        sys
    }
}
