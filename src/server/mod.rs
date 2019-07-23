//! Main application server

use std::sync::Arc;

use actix_cors::Cors;
use actix_rt::{System, SystemRunner};
use actix_web::http::StatusCode;
use actix_web::middleware::errhandlers::ErrorHandlers;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};
// use num_cpus;
use serde_json::json;

use crate::db::{mysql::MysqlDbPool, DbError, DbPool};
use crate::error::ApiError;
use crate::settings::{Secrets, ServerLimits, Settings};
use crate::web::handlers;
use crate::web::middleware;

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

            //TOOD: limit incoming request sizes
            // See: https://actix.rs/actix-web/actix_web/web/struct.JsonConfig.html
            // Do I need to specify the holding structure or can I be more generic?
            App::new()
                .data(state)
                // Middleware is applied LIFO
                .wrap(ErrorHandlers::new().handler(StatusCode::NOT_FOUND, ApiError::render_404))
                // TODO: Is there a way to define this by default?
                .wrap(
                    ErrorHandlers::new()
                        .handler(StatusCode::BAD_REQUEST, ApiError::add_content_type_to_err),
                )
                .wrap(ErrorHandlers::new().handler(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ApiError::add_content_type_to_err,
                ))
                .wrap(middleware::WeaveTimestamp::new())
                .wrap(middleware::PreConditionCheck::new())
                .wrap(middleware::DbTransaction::new())
                .wrap(Cors::default())
                .service(
                    web::resource("/1.5/{uid}/info/collections")
                        .route(web::get().to_async(handlers::get_collections)),
                )
                .service(
                    web::resource("/1.5/{uid}/info/collection_counts")
                        .route(web::get().to_async(handlers::get_collection_counts)),
                )
                .service(
                    web::resource("/1.5/{uid}/info/collection_usage")
                        .route(web::get().to_async(handlers::get_collection_usage)),
                )
                .service(
                    web::resource("/1.5/{uid}/info/configuration")
                        .route(web::get().to_async(handlers::get_configuration)),
                )
                .service(
                    web::resource("/1.5/{uid}/info/quota")
                        .route(web::get().to_async(handlers::get_quota)),
                )
                .service(
                    web::resource("/1.5/{uid}").route(web::delete().to_async(handlers::delete_all)),
                )
                .service(
                    web::resource("/1.5/{uid}/storage")
                        .route(web::delete().to_async(handlers::delete_all)),
                )
                .service(
                    web::resource("/1.5/{uid}/storage/{collection}")
                        .data(
                            actix_web::web::JsonConfig::default()
                                .limit(limits.max_request_bytes as usize)
                                .content_type(|ct| ct == mime::TEXT_PLAIN),
                        )
                        // TODO:
                        // .data(Bytes::configure(|cfg| {cfg.limit(settings.limits.max_request_bytes)}))
                        .route(web::delete().to_async(handlers::delete_collection))
                        .route(web::get().to_async(handlers::get_collection))
                        .route(web::post().to_async(handlers::post_collection)),
                )
                .service(
                    web::resource("/1.5/{uid}/storage/{collection}/{bso}")
                        .data(
                            actix_web::web::JsonConfig::default()
                                .limit(limits.max_request_bytes as usize)
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
        })
        .bind(format!("{}:{}", settings.host, settings.port))
        .unwrap()
        .start();
        Ok(sys)
    }
}
