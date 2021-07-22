//! Main application server

use std::{sync::Arc, time::Duration};

use actix_cors::Cors;
use actix_web::{
    dev, http::StatusCode, middleware::errhandlers::ErrorHandlers, web, App, HttpRequest,
    HttpResponse, HttpServer,
};
use cadence::StatsdClient;
use tokio::sync::RwLock;

use crate::db::{pool_from_settings, spawn_pool_periodic_reporter, DbPool};
use crate::error::ApiError;
use crate::server::metrics::Metrics;
use crate::settings::{Deadman, Secrets, ServerLimits, Settings};
use crate::tokenserver;
use crate::web::{handlers, middleware};

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

    /// limits rendered as JSON
    pub limits_json: String,

    /// Secrets used during Hawk authentication.
    pub secrets: Arc<Secrets>,

    // XXX: This is only any Option temporarily. Once Tokenserver is rolled out to production,
    // it will always be enabled, and syncstorage will always have state associated with
    // Tokenserver.
    pub tokenserver_state: Option<tokenserver::ServerState>,

    /// Metric reporting
    pub metrics: Box<StatsdClient>,

    pub port: u16,

    pub quota_enabled: bool,

    pub deadman: Arc<RwLock<Deadman>>,
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
            .wrap(middleware::weave::WeaveTimestamp::new())
            .wrap(middleware::sentry::SentryWrapper::default())
            .wrap(middleware::rejectua::RejectUA::default())
            // Followed by the "official middleware" so they run first.
            // actix is getting increasingly tighter about CORS headers. Our server is
            // not a huge risk but does deliver XHR JSON content.
            // For now, let's be permissive and use NGINX (the wrapping server)
            // for finer grained specification.
            .wrap(Cors::permissive())
            .service(
                web::resource(&cfg_path("/info/collections"))
                    .route(web::get().to(handlers::get_collections)),
            )
            .service(
                web::resource(&cfg_path("/info/collection_counts"))
                    .route(web::get().to(handlers::get_collection_counts)),
            )
            .service(
                web::resource(&cfg_path("/info/collection_usage"))
                    .route(web::get().to(handlers::get_collection_usage)),
            )
            .service(
                web::resource(&cfg_path("/info/configuration"))
                    .route(web::get().to(handlers::get_configuration)),
            )
            .service(
                web::resource(&cfg_path("/info/quota")).route(web::get().to(handlers::get_quota)),
            )
            .service(web::resource(&cfg_path("")).route(web::delete().to(handlers::delete_all)))
            .service(
                web::resource(&cfg_path("/storage")).route(web::delete().to(handlers::delete_all)),
            )
            .service(
                web::resource(&cfg_path("/storage/{collection}"))
                    .app_data(
                        // Declare the payload limit for "normal" collections.
                        web::PayloadConfig::new($limits.max_request_bytes as usize),
                    )
                    .app_data(
                        // Declare the payload limits for "JSON" payloads
                        // (Specify "text/plain" for legacy client reasons)
                        web::JsonConfig::default()
                            .limit($limits.max_request_bytes as usize)
                            .content_type(|ct| ct == mime::TEXT_PLAIN),
                    )
                    .route(web::delete().to(handlers::delete_collection))
                    .route(web::get().to(handlers::get_collection))
                    .route(web::post().to(handlers::post_collection)),
            )
            .service(
                web::resource(&cfg_path("/storage/{collection}/{bso}"))
                    .app_data(web::PayloadConfig::new($limits.max_request_bytes as usize))
                    .app_data(
                        web::JsonConfig::default()
                            .limit($limits.max_request_bytes as usize)
                            .content_type(|ct| ct == mime::TEXT_PLAIN),
                    )
                    .route(web::delete().to(handlers::delete_bso))
                    .route(web::get().to(handlers::get_bso))
                    .route(web::put().to(handlers::put_bso)),
            )
            // XXX: This route will be enabled when we are ready to roll out Tokenserver
            // Tokenserver
            // .service(
            //     web::resource("/1.0/sync/1.5".to_string())
            //         .route(web::get().to(tokenserver::handlers::get_tokenserver_result)),
            // )
            // Dockerflow
            // Remember to update .::web::middleware::DOCKER_FLOW_ENDPOINTS
            // when applying changes to endpoint names.
            .service(web::resource("/__heartbeat__").route(web::get().to(handlers::heartbeat)))
            .service(web::resource("/__lbheartbeat__").route(web::get().to(
                handlers::lbheartbeat, /*
                                           |_: HttpRequest| {
                                           // used by the load balancers, just return OK.
                                           HttpResponse::Ok()
                                               .content_type("application/json")
                                               .body("{}")
                                       }
                                       */
            )))
            .service(
                web::resource("/__version__").route(web::get().to(|_: HttpRequest| {
                    // return the contents of the version.json file created by circleci
                    // and stored in the docker root
                    HttpResponse::Ok()
                        .content_type("application/json")
                        .body(include_str!("../../version.json"))
                })),
            )
            .service(web::resource("/__error__").route(web::get().to(handlers::test_error)))
    };
}

impl Server {
    pub async fn with_settings(settings: Settings) -> Result<dev::Server, ApiError> {
        let metrics = metrics::metrics_from_opts(&settings)?;
        let db_pool = pool_from_settings(&settings, &Metrics::from(&metrics)).await?;
        let limits = Arc::new(settings.limits);
        let limits_json =
            serde_json::to_string(&*limits).expect("ServerLimits failed to serialize");
        let secrets = Arc::new(settings.master_secret);
        let host = settings.host.clone();
        let port = settings.port;
        let quota_enabled = settings.enable_quota;
        let actix_keep_alive = settings.actix_keep_alive;
        let deadman = Arc::new(RwLock::new(Deadman {
            max_size: settings.database_pool_max_size,
            ..Default::default()
        }));
        let tokenserver_state = if settings.tokenserver.enabled {
            Some(tokenserver::ServerState::from_settings(
                &settings.tokenserver,
            )?)
        } else {
            None
        };

        spawn_pool_periodic_reporter(Duration::from_secs(10), metrics.clone(), db_pool.clone())?;

        let mut server = HttpServer::new(move || {
            // Setup the server state
            let state = ServerState {
                db_pool: db_pool.clone(),
                limits: Arc::clone(&limits),
                limits_json: limits_json.clone(),
                secrets: Arc::clone(&secrets),
                tokenserver_state: tokenserver_state.clone(),
                metrics: Box::new(metrics.clone()),
                port,
                quota_enabled,
                deadman: Arc::clone(&deadman),
            };

            build_app!(state, limits)
        });
        if let Some(keep_alive) = actix_keep_alive {
            server = server.keep_alive(keep_alive as usize);
        }
        let server = server
            .bind(format!("{}:{}", host, port))
            .expect("Could not get Server in Server::with_settings")
            .run();
        Ok(server)
    }
}
