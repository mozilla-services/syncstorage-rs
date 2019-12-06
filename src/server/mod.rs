//! Main application server

use std::sync::Arc;

use actix_cors::Cors;
use actix_rt::{System, SystemRunner};
use actix_web::{
    http::StatusCode, middleware::errhandlers::ErrorHandlers, web, App, HttpRequest, HttpResponse,
    HttpServer,
};
// use num_cpus;
use crate::db::{pool_from_settings, DbPool};
use crate::error::ApiError;
use crate::server::metrics::Metrics;
use crate::settings::{Secrets, ServerLimits, Settings};
use crate::web::{handlers, middleware};
use cadence::StatsdClient;

pub const BSO_ID_REGEX: &str = r"[ -~]{1,64}";
pub const COLLECTION_ID_REGEX: &str = r"[a-zA-Z0-9._-]{1,32}";
const MYSQL_UID_REGEX: &str = r"[0-9]{1,10}";
const SYNC_VERSION_PATH: &str = "1.5";

pub mod metrics;
#[cfg(test)]
mod test;
pub mod user_agent;

/// This is the global HTTP state object that will be made available to all
/// HTTP API calls.
pub struct ServerState {
    pub db_pool: Box<dyn DbPool>,

    /// Server-enforced limits for request payloads.
    pub limits: Arc<ServerLimits>,

    /// Secrets used during Hawk authentication.
    pub secrets: Arc<Secrets>,

    /// Metric reporting
    pub metrics: Box<StatsdClient>,

    pub port: u16,
}

pub fn cfg_path(path: &str) -> String {
    let path = path
        .replace(
            "{collection}",
            &format!("{{collection:{}}}", COLLECTION_ID_REGEX),
        )
        .replace("{bso}", &format!("{{bso:{}}}", BSO_ID_REGEX));
    format!("/{}/{{uid:{}}}{}", SYNC_VERSION_PATH, MYSQL_UID_REGEX, path)
}

pub struct Server;

#[macro_export]
macro_rules! build_app {
    ($state: expr, $limits: expr) => {
        App::new()
            .data($state)
            // Middleware is applied LIFO
            // These will wrap all outbound responses with matching status codes.
            .wrap(ErrorHandlers::new().handler(StatusCode::NOT_FOUND, ApiError::render_404))
            // These are our wrappers
            .wrap(middleware::PreConditionCheck::new())
            .wrap(middleware::DbTransaction::new())
            .wrap(middleware::WeaveTimestamp::new())
            // Followed by the "official middleware" so they run first.
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
                web::resource(&cfg_path("")).route(web::delete().to_async(handlers::delete_all)),
            )
            .service(
                web::resource(&cfg_path("/storage"))
                    .route(web::delete().to_async(handlers::delete_all)),
            )
            .service(
                web::resource(&cfg_path("/storage/{collection}"))
                    .data(
                        // Declare the payload limit for "normal" collections.
                        web::PayloadConfig::new($limits.max_request_bytes as usize),
                    )
                    .data(
                        // Declare the payload limits for "JSON" payloads
                        // (Specify "text/plain" for legacy client reasons)
                        web::JsonConfig::default()
                            .limit($limits.max_request_bytes as usize)
                            .content_type(|ct| ct == mime::TEXT_PLAIN),
                    )
                    .route(web::delete().to_async(handlers::delete_collection))
                    .route(web::get().to_async(handlers::get_collection))
                    .route(web::post().to_async(handlers::post_collection)),
            )
            .service(
                web::resource(&cfg_path("/storage/{collection}/{bso}"))
                    .data(web::PayloadConfig::new($limits.max_request_bytes as usize))
                    .data(
                        web::JsonConfig::default()
                            .limit($limits.max_request_bytes as usize)
                            .content_type(|ct| ct == mime::TEXT_PLAIN),
                    )
                    .route(web::delete().to_async(handlers::delete_bso))
                    .route(web::get().to_async(handlers::get_bso))
                    .route(web::put().to_async(handlers::put_bso)),
            )
            // Dockerflow
            // Remember to update .::web::middleware::DOCKER_FLOW_ENDPOINTS
            // when applying changes to endpoint names.
            .service(
                web::resource("/__heartbeat__").route(web::get().to_async(handlers::heartbeat)),
            )
            .service(
                web::resource("/__lbheartbeat__").route(web::get().to(|_: HttpRequest| {
                    // used by the load balancers, just return OK.
                    HttpResponse::Ok()
                        .content_type("application/json")
                        .body("{}")
                })),
            )
            .service(
                web::resource("/__version__").route(web::get().to(|_: HttpRequest| {
                    // return the contents of the version.json file created by circleci
                    // and stored in the docker root
                    HttpResponse::Ok()
                        .content_type("application/json")
                        .body(include_str!("../../version.json"))
                })),
            )
    };
}

impl Server {
    pub fn with_settings(settings: Settings) -> Result<SystemRunner, ApiError> {
        let sys = System::new("syncserver");
        let metrics = metrics::metrics_from_opts(&settings)?;
        let db_pool = pool_from_settings(&settings, &Metrics::from(&metrics))?;
        let limits = Arc::new(settings.limits);
        let secrets = Arc::new(settings.master_secret);
        let port = settings.port;

        HttpServer::new(move || {
            // Setup the server state
            let state = ServerState {
                db_pool: db_pool.clone(),
                limits: Arc::clone(&limits),
                secrets: Arc::clone(&secrets),
                metrics: Box::new(metrics.clone()),
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
