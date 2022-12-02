#![warn(rust_2018_idioms)]
#![allow(clippy::try_err)]

#[macro_use]
extern crate slog_scope;

pub mod logging;
mod middleware;

use std::{error::Error, sync::Arc};

use actix_cors::Cors;
#[cfg(feature = "syncstorage")]
use actix_web::http::StatusCode;
#[cfg(feature = "syncstorage")]
use actix_web::middleware::errhandlers::ErrorHandlers;
use actix_web::{dev, http::Method, web, App, HttpServer};
#[cfg(not(all(feature = "syncstorage", feature = "tokenserver")))]
use actix_web::{web::ServiceConfig, HttpRequest, HttpResponse};
use syncserver_settings::Settings;

#[cfg(not(any(feature = "syncstorage", feature = "tokenserver")))]
compile_error!("At least one of the \"syncstorage\" or \"tokenserver\" features must be enabled for this crate.");

pub struct Server;

#[cfg(all(feature = "syncstorage", not(feature = "tokenserver")))]
#[macro_export]
macro_rules! build_app {
    ($settings: expr) => {
        App::new()
            .data(Arc::new($settings.master_secret.clone()))
            // Middleware is applied LIFO
            // These will wrap all outbound responses with matching status codes.
            .wrap(ErrorHandlers::new().handler(
                StatusCode::NOT_FOUND,
                syncstorage_web::middleware::render_404,
            ))
            // These are our wrappers
            .wrap(syncstorage_web::middleware::WeaveTimestamp::default())
            .wrap(middleware::sentry::SentryWrapper::<
                syncstorage_web::ServerState,
                syncstorage_web::ApiError,
            >::default())
            .wrap(middleware::rejectua::RejectUA::<syncstorage_web::ServerState>::default())
            // Followed by the "official middleware" so they run first.
            // actix is getting increasingly tighter about CORS headers. Our server is
            // not a huge risk but does deliver XHR JSON content.
            // For now, let's be permissive and use NGINX (the wrapping server)
            // for finer grained specification.
            .wrap(build_cors($settings))
            .wrap(syncstorage_web::middleware::EmitTokenserverOrigin::default())
            .configure(
                syncstorage_web::get_configurator(
                    &$settings.syncstorage,
                    $settings.statsd_host.as_deref(),
                    $settings.statsd_port,
                )
                .expect("failed to build syncstorage configurator"),
            )
            .configure(configure_dockerflow)
    };
}

#[cfg(all(not(feature = "syncstorage"), feature = "tokenserver"))]
#[macro_export]
macro_rules! build_app {
    ($settings: expr) => {
        App::new()
            .data(Arc::new($settings.master_secret.clone()))
            // Middleware is applied LIFO
            // These will wrap all outbound responses with matching status codes.
            .wrap(middleware::sentry::SentryWrapper::<
                tokenserver_web::ServerState,
                tokenserver_web::ApiError,
            >::default())
            .wrap(tokenserver_web::middleware::LoggingWrapper::new())
            .wrap(middleware::rejectua::RejectUA::<tokenserver_web::ServerState>::default())
            // Followed by the "official middleware" so they run first.
            // actix is getting increasingly tighter about CORS headers. Our server is
            // not a huge risk but does deliver XHR JSON content.
            // For now, let's be permissive and use NGINX (the wrapping server)
            // for finer grained specification.
            .wrap(build_cors($settings))
            .configure(
                tokenserver_web::get_configurator(
                    &$settings.tokenserver,
                    $settings.statsd_host.as_deref(),
                    $settings.statsd_port,
                )
                .expect("failed to build tokenserver configurator"),
            )
            .configure(configure_dockerflow)
    };
}

#[cfg(all(feature = "syncstorage", feature = "tokenserver"))]
#[macro_export]
macro_rules! build_app {
    ($settings: expr, $blocking_threadpool: expr) => {
        App::new()
            .data(Arc::new($settings.master_secret.clone()))
            .service(
                web::scope("/token")
                    .wrap_fn(tokenserver_web::middleware::handle_request_log_line)
                    .configure(
                        tokenserver_web::get_configurator(
                            &$settings.tokenserver,
                            $settings.statsd_host.as_deref(),
                            $settings.statsd_port,
                            $blocking_threadpool,
                        )
                        .expect("failed to build tokenserver configurator"),
                    ),
            )
            .service(
                web::scope("/storage")
                    .wrap(ErrorHandlers::new().handler(
                        StatusCode::NOT_FOUND,
                        syncstorage_web::middleware::render_404,
                    ))
                    // TODO: does this do what it's supposed to?
                    .wrap(syncstorage_web::middleware::WeaveTimestamp::default())
                    .configure(
                        syncstorage_web::get_configurator(
                            &$settings.syncstorage,
                            $settings.statsd_host.as_deref(),
                            $settings.statsd_port,
                            $blocking_threadpool,
                        )
                        .expect("failed to build syncstorage configurator"),
                    ),
            )
            // Middleware is applied LIFO
            // These are our wrappers
            // Followed by the "official middleware" so they run first.
            // actix is getting increasingly tighter about CORS headers. Our server is
            // not a huge risk but does deliver XHR JSON content.
            // For now, let's be permissive and use NGINX (the wrapping server)
            // for finer grained specification.
            .wrap(build_cors($settings))
    };
}

/// This function configures the following endpoints:
/// * GET /__error__
/// * GET /__version__
/// * If syncstorage and tokenserver are both enabled, `GET __heartbeat__`, since we need to
///   combine the `GET __heartbeat__` endpoints configured by syncstorage and tokenserver
///   in their `get_configurator` functions
///
/// Any other dockerflow endpoints are configured individually by syncstorage and tokenserver.
#[cfg(not(all(feature = "syncstorage", feature = "tokenserver")))]
fn configure_dockerflow(cfg: &mut ServiceConfig) {
    // try returning an API error
    async fn test_error() -> HttpResponse {
        // generate an error for sentry.

        // ApiError will call the middleware layer to auto-append the tags.
        error!("Test Error");
        HttpResponse::ServiceUnavailable().body("Test error for Sentry")
    }

    cfg.service(web::resource("/__error__").route(web::get().to(test_error)))
        .service(
            web::resource("/__version__").route(web::get().to(|_: HttpRequest| {
                // return the contents of the version.json file created by circleci
                // and stored in the docker root
                HttpResponse::Ok()
                    .content_type("application/json")
                    .body(include_str!("../version.json"))
            })),
        );

    #[cfg(all(feature = "syncstorage", feature = "tokenserver"))]
    {
        use serde::Serialize;

        #[derive(Serialize)]
        struct HeartbeatResponse {
            syncstorage: syncstorage_web::HeartbeatResponse,
            tokenserver: tokenserver_web::HeartbeatResponse,
        }
        // TODO: does this overwrite already-registered route?
        cfg.service(web::resource("/__heartbeat__").route(web::get().to(
            |syncstorage_resp: syncstorage_web::HeartbeatResponse,
             tokenserver_resp: tokenserver_web::HeartbeatResponse|
             -> HttpResponse {
                let resp = HeartbeatResponse {
                    syncstorage: syncstorage_resp,
                    tokenserver: tokenserver_resp,
                };

                if syncstorage_resp.is_available() && tokenserver_resp.is_available() {
                    HttpResponse::Ok().json(resp)
                } else {
                    HttpResponse::ServiceUnavailable().json(resp)
                }
            },
        )));
    }
}

impl Server {
    pub async fn with_settings(settings: &'static Settings) -> Result<dev::Server, Box<dyn Error>> {
        let server = HttpServer::new(move || build_app!(settings));

        let server = server
            .bind(format!("{}:{}", settings.host, settings.port))
            .expect("Could not get Server in Server::with_settings")
            .run();
        Ok(server)
    }
}

pub fn build_cors(settings: &Settings) -> Cors {
    // Followed by the "official middleware" so they run first.
    // actix is getting increasingly tighter about CORS headers. Our server is
    // not a huge risk but does deliver XHR JSON content.
    // For now, let's be permissive and use NGINX (the wrapping server)
    // for finer grained specification.
    let mut cors = Cors::default();

    if let Some(allowed_origin) = &settings.cors_allowed_origin {
        cors = cors.allowed_origin(allowed_origin);
    }

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

    cors
}
