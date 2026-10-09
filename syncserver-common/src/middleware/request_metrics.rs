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
//! [`UNMATCHED_ROUTE`]. `method` is one of the standard HTTP methods or
//! [`OTHER_METHOD`]: the method token is client-controlled, so it is never
//! emitted verbatim. Telegraf surfaces the timing as
//! `syncstorage_request_duration_{count,sum,mean,…}` (or under the
//! `syncstorage_tokenserver_` prefix for a tokenserver-only deployment), where
//! `rate(_sum) / rate(_count)` by `route` is an exact windowed mean per route
//! and `_count` by `route, status` is a per-route status breakdown.
//!
//! Durations are whole milliseconds, truncated, the same as the `Metrics`
//! drop timer. Sub-millisecond requests such as health checks and 304s record
//! as `0`, so a 0 ms mean on `/__heartbeat__` is expected, not a bug.
//!
//! Install it outermost in the middleware chain so the timing covers every
//! other middleware as well as the handler.

use std::{collections::HashMap, sync::Arc, time::Instant};

use actix_web::{
    Error,
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    http::Method,
    middleware::Next,
};
use cadence::StatsdClient;
use futures::{FutureExt, future::LocalBoxFuture};

use crate::Metrics;

/// The statsd timing emitted once per request.
pub const REQUEST_DURATION_METRIC: &str = "request.duration";

/// `route` tag value for requests that matched no resource.
pub const UNMATCHED_ROUTE: &str = "unmatched";

/// `method` tag value for anything other than a standard HTTP method.
pub const OTHER_METHOD: &str = "OTHER";

/// Build the middleware function. Pass it to
/// [`from_fn`](actix_web::middleware::from_fn) and wrap the `App` with it,
/// outermost.
pub fn request_metrics<B>(
    metrics: Arc<StatsdClient>,
) -> impl Fn(ServiceRequest, Next<B>) -> LocalBoxFuture<'static, Result<ServiceResponse<B>, Error>> + Clone
where
    B: MessageBody + 'static,
{
    move |req, next| time_request(req, next, metrics.clone()).boxed_local()
}

/// Time one request and emit [`REQUEST_DURATION_METRIC`].
async fn time_request<B>(
    req: ServiceRequest,
    next: Next<B>,
    metrics: Arc<StatsdClient>,
) -> Result<ServiceResponse<B>, Error>
where
    B: MessageBody + 'static,
{
    let start = Instant::now();
    let method = method_tag(req.method());
    let route = req
        .match_pattern()
        .map(|pattern| route_tag(&pattern))
        .unwrap_or_else(|| UNMATCHED_ROUTE.to_owned());

    let result = next.call(req).await;
    // An `Err` here is still a response to the client once the outer layers
    // render it, so time it under the status it will get. actix renders it
    // with `error_response()`, and not every error type overrides
    // `status_code()` to match (`ApiError` does not, so it would report 500
    // for everything), so read the status off the rendered response.
    let status = match &result {
        Ok(resp) => resp.status(),
        Err(err) => err.as_response_error().error_response().status(),
    };

    let mut tags = HashMap::with_capacity(3);
    tags.insert("route".to_owned(), route);
    tags.insert("method".to_owned(), method.to_owned());
    tags.insert("status".to_owned(), status.as_u16().to_string());
    Metrics::from(&metrics).timing_with_tags(
        REQUEST_DURATION_METRIC,
        start.elapsed().as_millis() as u64,
        tags,
    );

    result
}

/// The method as a bounded tag value.
///
/// `http::Method` accepts any extension token, so a client could mint a new
/// time series per made-up method. Only the standard methods pass through;
/// everything else collapses to [`OTHER_METHOD`].
fn method_tag(method: &Method) -> &'static str {
    match method.as_str() {
        "GET" => "GET",
        "HEAD" => "HEAD",
        "POST" => "POST",
        "PUT" => "PUT",
        "DELETE" => "DELETE",
        "PATCH" => "PATCH",
        "OPTIONS" => "OPTIONS",
        _ => OTHER_METHOD,
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
    use actix_web::{App, HttpResponse, http::StatusCode, middleware::from_fn, web};
    use cadence::SpyMetricSink;

    use super::*;

    #[test]
    fn method_tag_bounds_the_label() {
        assert_eq!(method_tag(&Method::GET), "GET");
        assert_eq!(method_tag(&Method::DELETE), "DELETE");
        let brew = Method::from_bytes(b"BREW").unwrap();
        assert_eq!(method_tag(&brew), OTHER_METHOD);
    }

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
            .wrap(from_fn(request_metrics(client)))
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
            .wrap(from_fn(request_metrics(client)))
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
    async fn non_standard_methods_collapse_to_other() {
        let (client, drain) = recording_client();
        let resource = web::resource("/__heartbeat__").route(web::get().to(ok));
        let app = App::new()
            .wrap(from_fn(request_metrics(client)))
            .service(resource);
        let app = init_service(app).await;

        let brew = Method::from_bytes(b"BREW").unwrap();
        let req = TestRequest::default()
            .method(brew)
            .uri("/__heartbeat__")
            .to_request();
        let resp = call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);
        assert_tags(
            &one_line(drain()),
            &["route:/__heartbeat__", "method:OTHER", "status:405"],
        );
    }

    #[actix_web::test]
    async fn handler_errors_are_timed_under_their_status() {
        let (client, drain) = recording_client();
        let resource = web::resource("/__error__").route(web::get().to(bad_gateway));
        let app = App::new()
            .wrap(from_fn(request_metrics(client)))
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
