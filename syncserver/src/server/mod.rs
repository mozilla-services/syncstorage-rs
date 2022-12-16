//! Main application server

use std::{
    env, fmt,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use actix_cors::Cors;
use actix_web::{
    dev,
    error::BlockingError,
    http::StatusCode,
    http::{header::LOCATION, Method},
    middleware::errhandlers::ErrorHandlers,
    web, App, HttpRequest, HttpResponse, HttpServer,
};
use cadence::{Gauged, StatsdClient};
use syncserver_common::InternalError;
use syncserver_db_common::{error::DbError, DbPool, GetPoolState, PoolState};
use syncserver_settings::Settings;
use syncstorage_settings::{Deadman, ServerLimits};
use tokio::{sync::RwLock, time};

use crate::db::pool_from_settings;
use crate::error::ApiError;
use crate::server::metrics::Metrics;
use crate::tokenserver;
use crate::web::{handlers, middleware};

pub const BSO_ID_REGEX: &str = r"[ -~]{1,64}";
pub const COLLECTION_ID_REGEX: &str = r"[a-zA-Z0-9._-]{1,32}";
pub const SYNC_DOCS_URL: &str =
    "https://mozilla-services.readthedocs.io/en/latest/storage/apis-1.5.html";
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
    ($syncstorage_state: expr, $tokenserver_state: expr, $secrets: expr, $limits: expr, $cors: expr) => {
        App::new()
            .configure(|cfg| {
                cfg.data($syncstorage_state);

                if let Some(tokenserver_state) = $tokenserver_state {
                    let state = tokenserver_state.clone();
                    cfg.data(state);
                }
            })
            .data($secrets)
            // Middleware is applied LIFO
            // These will wrap all outbound responses with matching status codes.
            .wrap(ErrorHandlers::new().handler(StatusCode::NOT_FOUND, ApiError::render_404))
            // These are our wrappers
            .wrap(middleware::weave::WeaveTimestamp::new())
            .wrap(tokenserver::logging::LoggingWrapper::new())
            .wrap(middleware::sentry::SentryWrapper::default())
            .wrap(middleware::rejectua::RejectUA::default())
            .wrap($cors)
            .wrap_fn(middleware::emit_http_status_with_tokenserver_origin)
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
            // Tokenserver
            .service(
                web::resource("/1.0/{application}/{version}")
                    .route(web::get().to(tokenserver::handlers::get_tokenserver_result)),
            )
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
            .service(web::resource("/").route(web::get().to(|_: HttpRequest| {
                HttpResponse::Found()
                    .header(LOCATION, SYNC_DOCS_URL)
                    .finish()
            })))
    };
}

#[macro_export]
macro_rules! build_app_without_syncstorage {
    ($state: expr, $secrets: expr, $cors: expr) => {
        App::new()
            .data($state)
            .data($secrets)
            // Middleware is applied LIFO
            // These will wrap all outbound responses with matching status codes.
            .wrap(ErrorHandlers::new().handler(StatusCode::NOT_FOUND, ApiError::render_404))
            // These are our wrappers
            .wrap(middleware::sentry::SentryWrapper::default())
            .wrap(tokenserver::logging::LoggingWrapper::new())
            .wrap(middleware::rejectua::RejectUA::default())
            // Followed by the "official middleware" so they run first.
            // actix is getting increasingly tighter about CORS headers. Our server is
            // not a huge risk but does deliver XHR JSON content.
            // For now, let's be permissive and use NGINX (the wrapping server)
            // for finer grained specification.
            .wrap($cors)
            .service(
                web::resource("/1.0/{application}/{version}")
                    .route(web::get().to(tokenserver::handlers::get_tokenserver_result)),
            )
            // Dockerflow
            // Remember to update .::web::middleware::DOCKER_FLOW_ENDPOINTS
            // when applying changes to endpoint names.
            .service(
                web::resource("/__heartbeat__")
                    .route(web::get().to(tokenserver::handlers::heartbeat)),
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
            .service(
                web::resource("/__error__").route(web::get().to(tokenserver::handlers::test_error)),
            )
            .service(web::resource("/").route(web::get().to(|_: HttpRequest| {
                HttpResponse::Found()
                    .header(LOCATION, SYNC_DOCS_URL)
                    .finish()
            })))
    };
}

impl Server {
    pub async fn with_settings(settings: Settings) -> Result<dev::Server, ApiError> {
        let settings_copy = settings.clone();
        let metrics = metrics::metrics_from_opts(
            &settings.syncstorage.statsd_label,
            settings.statsd_host.as_deref(),
            settings.statsd_port,
        )?;
        let host = settings.host.clone();
        let port = settings.port;
        let deadman = Arc::new(RwLock::new(Deadman::from(&settings.syncstorage)));
        let blocking_threadpool = Arc::new(BlockingThreadpool::default());
        let db_pool = pool_from_settings(
            &settings.syncstorage,
            &Metrics::from(&metrics),
            blocking_threadpool.clone(),
        )
        .await?;
        let limits = Arc::new(settings.syncstorage.limits);
        let limits_json =
            serde_json::to_string(&*limits).expect("ServerLimits failed to serialize");
        let secrets = Arc::new(settings.master_secret);
        let quota_enabled = settings.syncstorage.enable_quota;
        let actix_keep_alive = settings.actix_keep_alive;
        let tokenserver_state = if settings.tokenserver.enabled {
            let state = tokenserver::ServerState::from_settings(
                &settings.tokenserver,
                metrics::metrics_from_opts(
                    &settings.tokenserver.statsd_label,
                    settings.statsd_host.as_deref(),
                    settings.statsd_port,
                )?,
                blocking_threadpool.clone(),
            )?;

            Some(state)
        } else {
            // Only spawn a metric-reporting task if tokenserver is not running; if tokenserver is
            // running, we are running syncstorange and tokenserver as a single service, which
            // is only done for self-hosters.
            spawn_metric_periodic_reporter(
                Duration::from_secs(10),
                metrics.clone(),
                db_pool.clone(),
                blocking_threadpool.clone(),
            )?;

            None
        };

        let mut server = HttpServer::new(move || {
            let syncstorage_state = ServerState {
                db_pool: db_pool.clone(),
                limits: Arc::clone(&limits),
                limits_json: limits_json.clone(),
                metrics: Box::new(metrics.clone()),
                port,
                quota_enabled,
                deadman: Arc::clone(&deadman),
            };

            build_app!(
                syncstorage_state,
                tokenserver_state.clone(),
                Arc::clone(&secrets),
                limits,
                build_cors(&settings_copy)
            )
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

    pub async fn tokenserver_only_with_settings(
        settings: Settings,
    ) -> Result<dev::Server, ApiError> {
        let settings_copy = settings.clone();
        let host = settings.host.clone();
        let port = settings.port;
        let secrets = Arc::new(settings.master_secret.clone());
        let blocking_threadpool = Arc::new(BlockingThreadpool::default());
        let tokenserver_state = tokenserver::ServerState::from_settings(
            &settings.tokenserver,
            metrics::metrics_from_opts(
                &settings.tokenserver.statsd_label,
                settings.statsd_host.as_deref(),
                settings.statsd_port,
            )?,
            blocking_threadpool.clone(),
        )?;

        spawn_metric_periodic_reporter(
            Duration::from_secs(10),
            *tokenserver_state.metrics.clone(),
            tokenserver_state.db_pool.clone(),
            blocking_threadpool,
        )?;

        let server = HttpServer::new(move || {
            build_app_without_syncstorage!(
                tokenserver_state.clone(),
                Arc::clone(&secrets),
                build_cors(&settings_copy)
            )
        });

        let server = server
            .bind(format!("{}:{}", host, port))
            .expect("Could not get Server in Server::with_settings")
            .run();
        Ok(server)
    }
}

fn build_cors(settings: &Settings) -> Cors {
    // Followed by the "official middleware" so they run first.
    // actix is getting increasingly tighter about CORS headers. Our server is
    // not a huge risk but does deliver XHR JSON content.
    // For now, let's be permissive and use NGINX (the wrapping server)
    // for finer grained specification.
    let mut cors = Cors::default();

    if let Some(allowed_methods) = &settings.cors_allowed_methods {
        let mut methods = vec![];
        for method_string in allowed_methods {
            let method = Method::from_bytes(method_string.as_bytes()).unwrap();
            methods.push(method);
        }
        cors = cors.allowed_methods(methods);
    }
    if let Some(allowed_headers) = &settings.cors_allowed_headers {
        cors = cors.allowed_headers(allowed_headers);
    }

    if let Some(max_age) = &settings.cors_max_age {
        cors = cors.max_age(*max_age);
    }

    // explicitly set the CORS allow origin, since Default does not
    // appear to set the `allow-origins: *` header.
    if let Some(ref origin) = settings.cors_allowed_origin {
        if origin == "*" {
            cors = cors.allow_any_origin();
        } else {
            cors = cors.allowed_origin(origin);
        }
    }

    cors
}

/// Emit database pool and threadpool metrics periodically
fn spawn_metric_periodic_reporter<T: GetPoolState + Send + 'static>(
    interval: Duration,
    metrics: StatsdClient,
    pool: T,
    blocking_threadpool: Arc<BlockingThreadpool>,
) -> Result<(), DbError> {
    let hostname = hostname::get()
        .expect("Couldn't get hostname")
        .into_string()
        .expect("Couldn't get hostname");
    let blocking_threadpool_size =
        str::parse::<u64>(&env::var("ACTIX_THREADPOOL").unwrap()).unwrap();
    tokio::spawn(async move {
        loop {
            let PoolState {
                connections,
                idle_connections,
            } = pool.state();
            metrics
                .gauge_with_tags(
                    "storage.pool.connections.active",
                    (connections - idle_connections) as u64,
                )
                .with_tag("hostname", &hostname)
                .send();
            metrics
                .gauge_with_tags("storage.pool.connections.idle", idle_connections as u64)
                .with_tag("hostname", &hostname)
                .send();

            let active_threads = blocking_threadpool.active_threads();
            let idle_threads = blocking_threadpool_size - active_threads;
            metrics
                .gauge_with_tags("blocking_threadpool.active", active_threads)
                .with_tag("hostname", &hostname)
                .send();
            metrics
                .gauge_with_tags("blocking_threadpool.idle", idle_threads)
                .with_tag("hostname", &hostname)
                .send();

            time::delay_for(interval).await;
        }
    });

    Ok(())
}

/// A threadpool on which callers can spawn non-CPU-bound tasks that block their thread (this is
/// mostly useful for running I/O tasks). `BlockingThreadpool` intentionally does not implement
/// `Clone`: `Arc`s are not used internally, so a `BlockingThreadpool` should be instantiated once
/// and shared by passing around `Arc<BlockingThreadpool>`s.
#[derive(Debug, Default)]
pub struct BlockingThreadpool {
    spawned_tasks: AtomicU64,
}

impl BlockingThreadpool {
    /// Runs a function as a task on the blocking threadpool.
    ///
    /// WARNING: Spawning a blocking task through means other than calling this method will
    /// result in inaccurate threadpool metrics being reported. If you want to spawn a task on
    /// the blocking threadpool, you **must** use this function.
    pub async fn spawn<F, T, E>(&self, f: F) -> Result<T, E>
    where
        F: FnOnce() -> Result<T, E> + Send + 'static,
        T: Send + 'static,
        E: fmt::Debug + Send + InternalError + 'static,
    {
        self.spawned_tasks.fetch_add(1, Ordering::Relaxed);

        let result = web::block(f).await.map_err(|e| match e {
            BlockingError::Error(e) => e,
            BlockingError::Canceled => {
                E::internal_error("Blocking threadpool operation canceled".to_owned())
            }
        });

        self.spawned_tasks.fetch_sub(1, Ordering::Relaxed);

        result
    }

    fn active_threads(&self) -> u64 {
        self.spawned_tasks.load(Ordering::Relaxed)
    }
}
