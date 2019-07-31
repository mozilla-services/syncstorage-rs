//! Main application server

use std::sync::Arc;

use actix_cors::Cors;
use actix_rt::{System, SystemRunner};
use actix_web::http::StatusCode;
use actix_web::middleware::errhandlers::ErrorHandlers;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};
// use num_cpus;
use serde_json::json;

use crate::db::{pool_from_settings, DbError, DbPool};
use crate::error::ApiError;
use crate::settings::{Secrets, ServerLimits, Settings};
use crate::web::handlers;
use crate::web::middleware;

pub const BSO_ID_REGEX: &str = r"[ -~]{1,64}";
pub const COLLECTION_ID_REGEX: &str = r"[a-zA-Z0-9._-]{1,32}";
const MYSQL_UID_REGEX: &str = r"[0-9]{1,10}";
const SYNC_VERSION_PATH: &str = "1.5";

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

pub fn cfg_path(path: &str) -> String {
    let path = path
        .replace(
            "{collection}",
            &format!("{{collection:{}}}", COLLECTION_ID_REGEX),
        )
        .replace("{bso}", &format!("{{bso:{}}}", BSO_ID_REGEX));
    // TODO: enforce a different uid regex under spanner
    format!("/{}/{{uid:{}}}{}", SYNC_VERSION_PATH, MYSQL_UID_REGEX, path)
}

pub struct Server {}

#[macro_export]
macro_rules! build_app{
    ($state: expr, $limits: expr) => {
            App::new()
                .data($state)
                // Middleware is applied LIFO
                // These will wrap all outbound responses with matching status codes.
                .wrap(ErrorHandlers::new().handler(StatusCode::NOT_FOUND, ApiError::render_404))
                // TODO: Is there a way to define this by default? Outboud errors don't generally set
                // content type or body.
                .wrap(
                    ErrorHandlers::new()
                        .handler(StatusCode::BAD_REQUEST, ApiError::add_content_type_to_err),
                )
                .wrap(ErrorHandlers::new().handler(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ApiError::add_content_type_to_err,
                ))
                // These are our wrappers
                .wrap(middleware::PreConditionCheck::new())
                .wrap(middleware::DbTransaction::new())
                .wrap(middleware::WeaveTimestamp::new())
                .wrap(Cors::default())
                .service(
                    web::resource(&cfg_path("/info/collections"))
                        .route(web::get().to_async(handlers::get_collections)),
                )
                .service(
                    web::resource(&cfg_path("/info/collection_counts"))
                        .route(web::get().to_async(handlers::get_collection_counts)),
                )
                .service(
                    web::resource(&cfg_path("/info/collection_usage"))
                        .route(web::get().to_async(handlers::get_collection_usage)),
                )
                .service(
                    web::resource(&cfg_path("/info/configuration"))
                        .route(web::get().to_async(handlers::get_configuration)),
                )
                .service(
                    web::resource(&cfg_path("/info/quota"))
                        .route(web::get().to_async(handlers::get_quota)),
                )
                .service(
                    web::resource(&cfg_path(""))
                        .route(web::delete().to_async(handlers::delete_all)),
                )
                .service(
                    web::resource(&cfg_path("/storage"))
                        .route(web::delete().to_async(handlers::delete_all)),
                )
                .service(
                    web::resource(&cfg_path("/storage/{collection}"))
                        .data(
                            // Declare the payload limit for "normal"
                            actix_web::web::PayloadConfig::new($limits.max_request_bytes as usize),
                        )
                        .data(
                            // Declare the payload limits for "JSON"
                            actix_web::web::JsonConfig::default()
                                .limit($limits.max_request_bytes as usize)
                                .content_type(|ct| ct == mime::TEXT_PLAIN),
                        )
                        .route(web::delete().to_async(handlers::delete_collection))
                        .route(web::get().to_async(handlers::get_collection))
                        .route(web::post().to_async(handlers::post_collection)),
                )
                .service(
                    web::resource(&cfg_path("/storage/{collection}/{bso}"))
                        .data(actix_web::web::PayloadConfig::new(
                            $limits.max_request_bytes as usize,
                        ))
                        .data(
                            actix_web::web::JsonConfig::default()
                                .limit($limits.max_request_bytes as usize)
                                .content_type(|ct| ct == mime::TEXT_PLAIN),
                        )
                        .route(web::delete().to_async(handlers::delete_bso))
                        .route(web::get().to_async(handlers::get_bso))
                        .route(web::put().to_async(handlers::put_bso)),
                )
                // Dockerflow
                .service(
                    web::resource("/__heartbeat__").route(web::get().to(|_: HttpRequest| {
                        // if addidtional information is desired, point to an appropriate handler.
                        let body = json!({"status": "ok", "version": env!("CARGO_PKG_VERSION")});
                        HttpResponse::Ok()
                            .content_type("application/json")
                            .body(body.to_string())
                    })),
                )
                .service(web::resource("/__lbheartbeat__").route(web::get().to(
                    |_: HttpRequest| {
                        // used by the load balancers, just return OK.
                        HttpResponse::Ok()
                            .content_type("application/json")
                            .body("{}")
                    },
                )))
                .service(
                    web::resource("/__version__").route(web::get().to(|_: HttpRequest| {
                        // return the contents of the version.json file created by circleci and stored in the docker root
                        HttpResponse::Ok()
                            .content_type("application/json")
                            .body(include_str!("../../version.json"))
                    })),
                )
    };
}

impl Server {
    pub fn with_settings(settings: Settings) -> Result<SystemRunner, DbError> {
        let sys = System::new("syncserver");
        let db_pool = pool_from_settings(&settings)?;
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

            build_app!(state, limits)
        })
        .bind(format!("{}:{}", settings.host, settings.port))
        .unwrap()
        .start();
        Ok(sys)
    }
}
