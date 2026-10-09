//! Per-request timing, tagged by route, method and response status.
//!
//! Every handler already increments a `request.<handler>` counter, but those
//! carry no status and no duration, and the GCLB histograms cannot be split by
//! route (it tags every sync request `matched_url_path_rule="UNMATCHED"`). So
//! until now there was no per-route latency or per-route error rate anywhere.
//! This middleware fills that gap with one statsd timing per request:
//!
//! ```text
//! request.duration:<ms>|ms|#route:/1.5/{uid}/storage/{collection},method:POST,status:200
//! ```
//!
//! `route` is the matched resource pattern with the parameter regexes
//! stripped, so it is low-cardinality and never contains a uid, collection
//! or BSO id. Requests that match no resource (404s) are tagged
//! [`UNMATCHED_ROUTE`]. Telegraf surfaces the timing as
//! `syncstorage_request_duration_{count,sum,mean,…}` (or under the
//! `syncstorage_tokenserver_` prefix for a tokenserver-only deployment), where
//! `rate(_sum) / rate(_count)` by `route` is an exact windowed mean per route
//! and `_count` by `route, status` is a per-route status breakdown.
//!
//! Install it outermost in the middleware chain so the timing covers every
//! other middleware as well as the handler.

use std::{collections::HashMap, rc::Rc, sync::Arc, time::Instant};

use actix_web::{
    Error,
    dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready},
};
use cadence::StatsdClient;
use futures::{
    FutureExt,
    future::{LocalBoxFuture, Ready, ok},
};

use crate::Metrics;

/// The statsd timing emitted once per request.
pub const REQUEST_DURATION_METRIC: &str = "request.duration";

/// `route` tag value for requests that matched no resource.
pub const UNMATCHED_ROUTE: &str = "unmatched";

/// Middleware factory. Wrap the `App` with it, outermost.
#[derive(Clone)]
pub struct RequestMetrics {
    metrics: Arc<StatsdClient>,
}

impl RequestMetrics {
    pub fn new(metrics: Arc<StatsdClient>) -> Self {
        Self { metrics }
    }
}

impl<S, B> Transform<S, ServiceRequest> for RequestMetrics
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = RequestMetricsMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(RequestMetricsMiddleware {
            service: Rc::new(service),
            metrics: self.metrics.clone(),
        })
    }
}

pub struct RequestMetricsMiddleware<S> {
    service: Rc<S>,
    metrics: Arc<StatsdClient>,
}

impl<S, B> Service<ServiceRequest> for RequestMetricsMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let start = Instant::now();
        let method = req.method().to_string();
        let route = req
            .match_pattern()
            .map(|pattern| route_tag(&pattern))
            .unwrap_or_else(|| UNMATCHED_ROUTE.to_owned());
        let metrics = self.metrics.clone();
        let fut = self.service.call(req);

        async move {
            let result = fut.await;
            // An `Err` here is still a response to the client once the outer
            // layers render it, so time it under the status it will get.
            let status = match &result {
                Ok(resp) => resp.status(),
                Err(err) => err.as_response_error().status_code(),
            };

            let mut tags = HashMap::with_capacity(3);
            tags.insert("route".to_owned(), route);
            tags.insert("method".to_owned(), method);
            tags.insert("status".to_owned(), status.as_u16().to_string());
            Metrics::from(&metrics).timing_with_tags(
                REQUEST_DURATION_METRIC,
                start.elapsed().as_millis() as u64,
                tags,
            );

            result
        }
        .boxed_local()
    }
}

/// Reduce a matched resource pattern to its parameter names.
///
/// `cfg_path` registers routes with inline regexes, e.g.
/// `/1.5/{uid:[0-9]{1,10}}/storage/{collection:[a-zA-Z0-9._-]{1,32}}`. Those
/// regexes contain their own braces, so this walks the string tracking brace
/// depth and drops everything from a top-level `:` to the matching `}`,
/// yielding `/1.5/{uid}/storage/{collection}`.
fn route_tag(pattern: &str) -> String {
    let mut out = String::with_capacity(pattern.len());
    let mut depth = 0usize;
    let mut skipping = false;
    for c in pattern.chars() {
        match c {
            '{' => {
                depth += 1;
                if !skipping {
                    out.push(c);
                }
            }
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    skipping = false;
                    out.push(c);
                } else if !skipping {
                    out.push(c);
                }
            }
            ':' if depth == 1 && !skipping => skipping = true,
            _ if skipping => {}
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use actix_web::test::{TestRequest, call_service, init_service};
    use actix_web::{App, HttpResponse, http::StatusCode, web};
    use cadence::SpyMetricSink;

    use super::*;

    #[test]
    fn route_tag_strips_parameter_regexes() {
        let bso =
            "/1.5/{uid:[0-9]{1,10}}/storage/{collection:[a-zA-Z0-9._-]{1,32}}/{bso:[ -~]{1,64}}";
        assert_eq!(route_tag(bso), "/1.5/{uid}/storage/{collection}/{bso}");
        assert_eq!(
            route_tag("/1.0/{application}/{version}"),
            "/1.0/{application}/{version}"
        );
        assert_eq!(route_tag("/swagger-ui/{_:.*}"), "/swagger-ui/{_}");
        assert_eq!(route_tag("/__heartbeat__"), "/__heartbeat__");
        assert_eq!(route_tag("/"), "/");
    }

    /// A client that records every statsd line, and a drain that returns the
    /// lines recorded since it was last called.
    fn recording_client() -> (Arc<StatsdClient>, impl Fn() -> Vec<String>) {
        let (recorded, sink) = SpyMetricSink::new();
        let client = Arc::new(StatsdClient::builder("syncstorage", sink).build());
        let drain = move || {
            recorded
                .try_iter()
                .map(|line| String::from_utf8(line).expect("statsd line was not utf-8"))
                .collect()
        };
        (client, drain)
    }

    /// Exactly one line was emitted; return it.
    fn one_line(lines: Vec<String>) -> String {
        assert_eq!(lines.len(), 1, "expected one metric, got {lines:?}");
        lines.into_iter().next().unwrap()
    }

    async fn ok() -> HttpResponse {
        HttpResponse::Ok().finish()
    }

    async fn not_modified() -> HttpResponse {
        HttpResponse::NotModified().finish()
    }

    async fn bad_gateway() -> actix_web::Result<HttpResponse> {
        Err(actix_web::error::ErrorBadGateway("boom"))
    }

    fn assert_tags(line: &str, tags: &[&str]) {
        assert!(
            line.starts_with("syncstorage.request.duration:"),
            "unexpected metric name: {line}"
        );
        assert!(line.contains("|ms|#"), "not a tagged timing: {line}");
        for tag in tags {
            assert!(line.contains(tag), "missing tag {tag}: {line}");
        }
    }

    #[actix_web::test]
    async fn tags_route_pattern_method_and_status() {
        let (client, drain) = recording_client();
        let resource = web::resource("/1.5/{uid:[0-9]{1,10}}/storage/{collection:[a-z]+}")
            .route(web::get().to(ok))
            .route(web::delete().to(not_modified));
        let app = App::new()
            .wrap(RequestMetrics::new(client))
            .service(resource);
        let app = init_service(app).await;

        let req = TestRequest::get()
            .uri("/1.5/42/storage/bookmarks")
            .to_request();
        let resp = call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert_tags(
            &one_line(drain()),
            &[
                "route:/1.5/{uid}/storage/{collection}",
                "method:GET",
                "status:200",
            ],
        );

        let req = TestRequest::delete()
            .uri("/1.5/42/storage/history")
            .to_request();
        let resp = call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_MODIFIED);
        assert_tags(
            &one_line(drain()),
            &[
                "route:/1.5/{uid}/storage/{collection}",
                "method:DELETE",
                "status:304",
            ],
        );
    }

    #[actix_web::test]
    async fn unmatched_requests_are_tagged_unmatched() {
        let (client, drain) = recording_client();
        let resource = web::resource("/__heartbeat__").route(web::get().to(ok));
        let app = App::new()
            .wrap(RequestMetrics::new(client))
            .service(resource);
        let app = init_service(app).await;

        let req = TestRequest::get().uri("/no/such/route").to_request();
        let resp = call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        assert_tags(
            &one_line(drain()),
            &["route:unmatched", "method:GET", "status:404"],
        );
    }

    #[actix_web::test]
    async fn handler_errors_are_timed_under_their_status() {
        let (client, drain) = recording_client();
        let resource = web::resource("/__error__").route(web::get().to(bad_gateway));
        let app = App::new()
            .wrap(RequestMetrics::new(client))
            .service(resource);
        let app = init_service(app).await;

        let req = TestRequest::get().uri("/__error__").to_request();
        let resp = call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
        assert_tags(
            &one_line(drain()),
            &["route:/__error__", "method:GET", "status:502"],
        );
    }
}
