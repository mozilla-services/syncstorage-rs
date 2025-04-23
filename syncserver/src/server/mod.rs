//! Main application server

use std::{convert::Infallible, num::NonZeroUsize, sync::Arc, time::Duration};

use actix_cors::Cors;
use actix_web::{
    dev::{self, Payload},
    http::StatusCode,
    http::{header::LOCATION, Method},
    middleware::ErrorHandlers,
    web::{self, Data},
    App, FromRequest, HttpRequest, HttpResponse, HttpServer,
};
use cadence::{Gauged, StatsdClient};
use futures::future::{self, Ready};
use http::Uri;
use syncserver_common::{
    middleware::sentry::SentryWrapper, BlockingThreadpool, BlockingThreadpoolMetrics, Metrics,
    Taggable,
};
use syncserver_db_common::{GetPoolState, PoolState};
use syncserver_settings::Settings;
use syncstorage_db::{DbError, DbPool, DbPoolImpl};
use syncstorage_settings::{Deadman, ServerLimits};
use tokio::{sync::RwLock, time};

use crate::error::ApiError;
use glean::server_events::GleanEventsLogger;

use crate::tokenserver;
use crate::web::{handlers, middleware};

pub const BSO_ID_REGEX: &str = r"[ -~]{1,64}";
pub const COLLECTION_ID_REGEX: &str = r"[a-zA-Z0-9._-]{1,32}";
pub const SYNC_DOCS_URL: &str =
    "https://mozilla-services.readthedocs.io/en/latest/storage/apis-1.5.html";
const MYSQL_UID_REGEX: &str = r"[0-9]{1,10}";
const SYNC_VERSION_PATH: &str = "1.5";

#[cfg(test)]
mod test;
pub mod user_agent;

/// This is the global HTTP state object that will be made available to all
/// HTTP API calls.
pub struct ServerState {
    pub db_pool: Box<dyn DbPool<Error = DbError>>,

    /// Server-enforced limits for request payloads.
    pub limits: Arc<ServerLimits>,

    /// limits rendered as JSON
    pub limits_json: String,

    /// Metric reporting
    pub metrics: Arc<StatsdClient>,

    pub port: u16,

    pub quota_enabled: bool,

    pub deadman: Arc<RwLock<Deadman>>,

    /// Glean metrics logger.
    pub glean_logger: Arc<GleanEventsLogger>,

    pub glean_enabled: bool,
}

/// This is a state holding data about the reverse proxy configuration.
/// This state object will be made available to all HTTP API calls.
pub struct ReverseProxyState {
    /// public facing URL of the server
    pub public_url: Option<String>,
}

impl ReverseProxyState {
    pub fn from_settings(settings: &Settings) -> ReverseProxyState {
        ReverseProxyState {
            public_url: settings.public_url.clone(),
        }
    }

    pub fn get_webroot(&self) -> String {
        match &self.public_url {
            None => "".to_owned(),
            Some(url) => match url.parse::<Uri>() {
                Err(_) => "".to_owned(),
                Ok(uri) => uri.path().to_owned(),
            },
        }
    }
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
    ($reverse_proxy_state: expr, $syncstorage_state: expr, $tokenserver_state: expr, $secrets: expr, $limits: expr, $cors: expr, $metrics: expr) => {
        App::new()
            .configure(|cfg| {
                cfg.app_data(Data::new($syncstorage_state));

                if let Some(tokenserver_state) = $tokenserver_state {
                    let state = tokenserver_state.clone();
                    cfg.app_data(Data::new(state));
                }
            })
            .app_data(Data::new($secrets))
            .app_data(Data::new($reverse_proxy_state))
            // Middleware is applied LIFO
            // These will wrap all outbound responses with matching status codes.
            .wrap(ErrorHandlers::new().handler(StatusCode::NOT_FOUND, ApiError::render_404))
            // These are our wrappers
            .wrap(SentryWrapper::<ApiError>::new($metrics.clone()))
            .wrap_fn(middleware::weave::set_weave_timestamp)
            .wrap_fn(tokenserver::logging::handle_request_log_line)
            .wrap_fn(middleware::rejectua::reject_user_agent)
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
                web::resource("/__version__").route(web::get().to(|_: HttpRequest| async {
                    // return the contents of the version.json file created by circleci
                    // and stored in the docker root
                    HttpResponse::Ok()
                        .content_type("application/json")
                        .body(include_str!("../../version.json"))
                })),
            )
            .service(web::resource("/__error__").route(web::get().to(handlers::test_error)))
            .service(
                web::resource("/").route(web::get().to(|_: HttpRequest| async {
                    HttpResponse::Found()
                        .insert_header((LOCATION, SYNC_DOCS_URL))
                        .finish()
                })),
            )
    };
}

#[macro_export]
macro_rules! build_app_without_syncstorage {
    ($reverse_proxy_state: expr, $state: expr, $secrets: expr, $cors: expr, $metrics: expr) => {
        App::new()
            .app_data(Data::new($state))
            .app_data(Data::new($secrets))
            .app_data(Data::new($reverse_proxy_state))
            // Middleware is applied LIFO
            // These will wrap all outbound responses with matching status codes.
            .wrap(ErrorHandlers::new().handler(StatusCode::NOT_FOUND, ApiError::render_404))
            .wrap(SentryWrapper::<tokenserver_common::TokenserverError>::new(
                $metrics.clone(),
            ))
            // These are our wrappers
            .wrap_fn(tokenserver::logging::handle_request_log_line)
            .wrap_fn(middleware::rejectua::reject_user_agent)
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
            .service(web::resource("/__lbheartbeat__").route(web::get().to(
                |_: HttpRequest| async {
                    // used by the load balancers, just return OK.
                    HttpResponse::Ok()
                        .content_type("application/json")
                        .body("{}")
                },
            )))
            .service(
                web::resource("/__version__").route(web::get().to(|_: HttpRequest| async {
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
            .service(
                web::resource("/").route(web::get().to(|_: HttpRequest| async {
                    HttpResponse::Found()
                        .insert_header((LOCATION, SYNC_DOCS_URL))
                        .finish()
                })),
            )
    };
}

impl Server {
    pub async fn with_settings(settings: Settings) -> Result<dev::Server, ApiError> {
        let settings_copy = settings.clone();
        let metrics = syncserver_common::metrics_from_opts(
            &settings.syncstorage.statsd_label,
            settings.statsd_host.as_deref(),
            settings.statsd_port,
        )?;
        let host = settings.host.clone();
        let port = settings.port;
        let deadman = Arc::new(RwLock::new(Deadman::from(&settings.syncstorage)));
        let blocking_threadpool = Arc::new(BlockingThreadpool::new(
            settings.worker_max_blocking_threads,
        ));
        let db_pool = DbPoolImpl::new(
            &settings.syncstorage,
            &Metrics::from(&metrics),
            blocking_threadpool.clone(),
        )?;
        // Spawns sweeper that calls Deadpool `retain` method, clearing unused connections.
        db_pool.spawn_sweeper(Duration::from_secs(
            settings
                .syncstorage
                .database_pool_sweeper_task_interval
                .into(),
        ));
        let glean_logger = Arc::new(GleanEventsLogger {
            // app_id corresponds to probe-scraper entry.
            // https://github.com/mozilla/probe-scraper/blob/main/repositories.yaml
            app_id: "syncstorage".to_owned(),
            app_display_version: env!("CARGO_PKG_VERSION").to_owned(),
            app_channel: settings.environment.clone(),
        });
        let glean_enabled = settings.syncstorage.glean_enabled;
        let worker_thread_count =
            calculate_worker_max_blocking_threads(settings.worker_max_blocking_threads);
        let limits = Arc::new(settings.syncstorage.limits);
        let limits_json =
            serde_json::to_string(&*limits).expect("ServerLimits failed to serialize");
        let secrets = Arc::new(settings.master_secret);
        let quota_enabled = settings.syncstorage.enable_quota;
        let actix_keep_alive = settings.actix_keep_alive;
        let tokenserver_state = if settings.tokenserver.enabled {
            let state = tokenserver::ServerState::from_settings(
                &settings.tokenserver,
                syncserver_common::metrics_from_opts(
                    &settings.tokenserver.statsd_label,
                    settings.statsd_host.as_deref(),
                    settings.statsd_port,
                )?,
                blocking_threadpool,
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
                blocking_threadpool,
            )?;

            None
        };

        let mut server = HttpServer::new(move || {
            let syncstorage_state = ServerState {
                db_pool: Box::new(db_pool.clone()),
                limits: Arc::clone(&limits),
                limits_json: limits_json.clone(),
                metrics: metrics.clone(),
                port,
                quota_enabled,
                deadman: Arc::clone(&deadman),
                glean_logger: Arc::clone(&glean_logger),
                glean_enabled,
            };

            build_app!(
                ReverseProxyState::from_settings(&settings_copy),
                syncstorage_state,
                tokenserver_state.clone(),
                Arc::clone(&secrets),
                limits,
                build_cors(&settings_copy),
                metrics.clone()
            )
        });

        if let Some(keep_alive) = actix_keep_alive {
            server = server.keep_alive(Duration::from_secs(keep_alive as u64));
        }

        let server = server
            .worker_max_blocking_threads(worker_thread_count)
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
        // Adjust the thread count to include FxA blocking threads.
        let thread_count = settings.worker_max_blocking_threads
            + settings
                .tokenserver
                .additional_blocking_threads_for_fxa_requests
                .unwrap_or(0) as usize;
        let blocking_threadpool = Arc::new(BlockingThreadpool::new(thread_count));
        let worker_thread_count = calculate_worker_max_blocking_threads(thread_count);
        let tokenserver_state = tokenserver::ServerState::from_settings(
            &settings.tokenserver,
            syncserver_common::metrics_from_opts(
                &settings.tokenserver.statsd_label,
                settings.statsd_host.as_deref(),
                settings.statsd_port,
            )?,
            blocking_threadpool.clone(),
        )?;

        spawn_metric_periodic_reporter(
            Duration::from_secs(10),
            tokenserver_state.metrics.clone(),
            tokenserver_state.db_pool.clone(),
            blocking_threadpool,
        )?;

        let server = HttpServer::new(move || {
            build_app_without_syncstorage!(
                ReverseProxyState::from_settings(&settings_copy),
                tokenserver_state.clone(),
                Arc::clone(&secrets),
                build_cors(&settings_copy),
                tokenserver_state.metrics.clone()
            )
        });

        let server = server
            .worker_max_blocking_threads(worker_thread_count)
            .bind(format!("{}:{}", host, port))
            .expect("Could not get Server in Server::with_settings")
            .run();
        Ok(server)
    }
}

fn calculate_worker_max_blocking_threads(count: usize) -> usize {
    let parallelism = std::thread::available_parallelism().map_or(2, NonZeroUsize::get);
    std::cmp::max(count / parallelism, 1)
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

pub struct MetricsWrapper(pub Metrics);

impl FromRequest for MetricsWrapper {
    type Error = Infallible;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let client = {
            let syncstorage_metrics = req
                .app_data::<Data<ServerState>>()
                .map(|state| state.metrics.clone());
            let tokenserver_metrics = req
                .app_data::<Data<tokenserver::ServerState>>()
                .map(|state| state.metrics.clone());

            syncstorage_metrics.or(tokenserver_metrics)
        };

        if client.is_none() {
            warn!("⚠️ metric error: No App State");
        }

        future::ok(MetricsWrapper(Metrics {
            client,
            tags: req.get_tags(),
            timer: None,
        }))
    }
}

/// Emit database pool and threadpool metrics periodically
fn spawn_metric_periodic_reporter<T: GetPoolState + Send + 'static>(
    interval: Duration,
    metrics: Arc<StatsdClient>,
    pool: T,
    blocking_threadpool: Arc<BlockingThreadpool>,
) -> Result<(), DbError> {
    let hostname = hostname::get()
        .expect("Couldn't get hostname")
        .into_string()
        .expect("Couldn't get hostname");
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

            let BlockingThreadpoolMetrics {
                queued_tasks,
                active_threads,
                max_idle_threads,
            } = blocking_threadpool.metrics();
            metrics
                .gauge_with_tags("blocking_threadpool.queued", queued_tasks)
                .with_tag("hostname", &hostname)
                .send();
            metrics
                .gauge_with_tags("blocking_threadpool.active", active_threads)
                .with_tag("hostname", &hostname)
                .send();
            metrics
                .gauge_with_tags("blocking_threadpool.max_idle", max_idle_threads)
                .with_tag("hostname", &hostname)
                .send();

            time::sleep(interval).await;
        }
    });

    Ok(())
}
