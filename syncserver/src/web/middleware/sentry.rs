use std::collections::HashMap;
use std::error::Error as StdError;
use std::task::{Context, Poll};
use std::{cell::RefCell, rc::Rc};

use actix_http::HttpMessage;
use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    http::header::USER_AGENT,
    Error, FromRequest,
};
use futures::future::{self, LocalBoxFuture};
use sentry::protocol::Event;
use sentry_backtrace::parse_stacktrace;
use serde_json::value::Value;
use syncserver_common::{Metrics, ReportableError};
use tokenserver_common::error::TokenserverError;

use crate::error::ApiError;
use crate::server::{tags::Taggable, user_agent, MetricsWrapper};

pub struct SentryWrapper;

impl SentryWrapper {
    pub fn new() -> Self {
        SentryWrapper::default()
    }
}

impl Default for SentryWrapper {
    fn default() -> Self {
        Self
    }
}

impl<S, B> Transform<S> for SentryWrapper
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = SentryWrapperMiddleware<S>;
    type Future = LocalBoxFuture<'static, Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        Box::pin(future::ok(SentryWrapperMiddleware {
            service: Rc::new(RefCell::new(service)),
        }))
    }
}

#[derive(Debug)]
pub struct SentryWrapperMiddleware<S> {
    service: Rc<RefCell<S>>,
}

pub fn report(
    tags: HashMap<String, String>,
    extra: HashMap<String, String>,
    mut event: Event<'static>,
) {
    event.tags.extend(tags.into_iter());
    event
        .extra
        .extend(extra.into_iter().map(|(k, v)| (k, Value::from(v))));
    trace!("Sentry: Sending error: {:?}", &event);
    sentry::capture_event(event);
}

impl<S, B> Service for SentryWrapperMiddleware<S>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, sreq: ServiceRequest) -> Self::Future {
        add_initial_tags(&sreq, sreq.head().method.to_string());
        add_initial_extras(&sreq, sreq.head().uri.to_string());

        let fut = self.service.call(sreq);

        Box::pin(async move {
            let mut sresp = fut.await?;
            let tags = sresp.request().get_tags();
            let extras = sresp.request().get_extras();

            match sresp.response().error() {
                None => {
                    // Middleware errors are eaten by current versions of Actix. Errors are now added
                    // to the extensions. Need to check both for any errors and report them.
                    if let Some(events) = sresp
                        .request()
                        .extensions_mut()
                        .remove::<Vec<Event<'static>>>()
                    {
                        for event in events {
                            trace!("Sentry: found an error stored in request: {:?}", &event);
                            report(tags.clone(), extras.clone(), event);
                        }
                    }
                    if let Some(events) = sresp
                        .response_mut()
                        .extensions_mut()
                        .remove::<Vec<Event<'static>>>()
                    {
                        for event in events {
                            trace!("Sentry: Found an error stored in response: {:?}", &event);
                            report(tags.clone(), extras.clone(), event);
                        }
                    }
                }
                Some(e) => {
                    let metrics = MetricsWrapper::extract(sresp.request()).await.unwrap().0;

                    if let Some(apie) = e.as_error::<ApiError>() {
                        process_error(apie, metrics, tags, extras);
                    } else if let Some(tokenserver_error) = e.as_error::<TokenserverError>() {
                        process_error(tokenserver_error, metrics, tags, extras);
                    }
                }
            }
            Ok(sresp)
        })
    }
}

fn process_error<E>(
    err: &E,
    metrics: Metrics,
    tags: HashMap<String, String>,
    extras: HashMap<String, String>,
) where
    E: ReportableError + StdError + 'static,
{
    if let Some(label) = err.metric_label() {
        metrics.incr(&label);
    }

    if err.is_sentry_event() {
        report(tags, extras, event_from_error(err));
    } else {
        trace!("Sentry: Not reporting error: {:?}", err);
    }
}

/// Custom `sentry::event_from_error` for `ReportableError`
///
/// `sentry::event_from_error` can't access `std::Error` backtraces as its
/// `backtrace()` method is currently Rust nightly only. This function works
/// against `ReportableError` instead to access its backtrace.
pub fn event_from_error<E>(err: &E) -> Event<'static>
where
    E: ReportableError + StdError + 'static,
{
    let mut exceptions = vec![exception_from_error_with_backtrace(err)];

    let mut source = err.source();
    while let Some(err) = source {
        let exception = if let Some(err) = err.downcast_ref::<E>() {
            exception_from_error_with_backtrace(err)
        } else {
            exception_from_error(err)
        };
        exceptions.push(exception);
        source = err.source();
    }

    exceptions.reverse();
    Event {
        exception: exceptions.into(),
        level: sentry::protocol::Level::Error,
        ..Default::default()
    }
}

/// Custom `exception_from_error` support function for `ReportableError`
///
/// Based moreso on sentry_failure's `exception_from_single_fail`.
fn exception_from_error_with_backtrace<E>(err: &E) -> sentry::protocol::Exception
where
    E: ReportableError + StdError,
{
    let mut exception = exception_from_error(err);
    exception.stacktrace = parse_stacktrace(&err.error_backtrace());
    exception
}

/// Exact copy of sentry's unfortunately private `exception_from_error`
fn exception_from_error<E: StdError + ?Sized>(err: &E) -> sentry::protocol::Exception {
    let dbg = format!("{:?}", err);
    sentry::protocol::Exception {
        ty: sentry::parse_type_from_debug(&dbg).to_owned(),
        value: Some(err.to_string()),
        ..Default::default()
    }
}

/// Adds HTTP-related tags to be included in every syncstorage or tokenserver request.
fn add_initial_tags<T>(msg: &T, method: String)
where
    T: Taggable + HttpMessage,
{
    msg.add_tag("uri.method".to_owned(), method);
}

/// Adds HTTP-related extras to be included in every syncstorage or tokenserver request.
fn add_initial_extras<T>(msg: &T, uri: String)
where
    T: Taggable + HttpMessage,
{
    if let Some(ua) = msg.headers().get(USER_AGENT) {
        if let Ok(uas) = ua.to_str() {
            let (ua_result, metrics_os, metrics_browser) = user_agent::parse_user_agent(uas);
            msg.add_extra("ua.os.family".to_owned(), metrics_os.to_owned());
            msg.add_extra("ua.browser.family".to_owned(), metrics_browser.to_owned());
            msg.add_extra("ua.name".to_owned(), ua_result.name.to_owned());
            msg.add_extra("ua.os.ver".to_owned(), ua_result.os_version.to_string());
            msg.add_extra("ua.browser.ver".to_owned(), ua_result.version.to_owned());
            msg.add_extra("ua".to_owned(), uas.to_string());
        }
    }

    msg.add_extra("uri.path".to_owned(), uri);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tags() {
        use actix_web::{http::header, test::TestRequest};
        use std::collections::HashMap;

        let uri = "/1.5/42/storage/meta/global".to_owned();
        let ua = "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:72.0) Gecko/20100101 Firefox/72.0";
        let req = TestRequest::default()
            .uri(&uri)
            .header(header::USER_AGENT, header::HeaderValue::from_static(ua))
            .to_http_request();

        add_initial_tags(&req, "GET".to_owned());
        add_initial_extras(&req, uri.clone());

        let mut tags = HashMap::<String, String>::new();
        tags.insert("uri.method".to_owned(), "GET".to_owned());

        for tag in tags.clone() {
            req.add_tag(tag.0.clone(), tag.1.clone());
        }

        let mut extras = HashMap::<String, String>::new();
        extras.insert("ua.os.ver".to_owned(), "NT 10.0".to_owned());
        extras.insert("ua.os.family".to_owned(), "Windows".to_owned());
        extras.insert("ua.browser.ver".to_owned(), "72.0".to_owned());
        extras.insert("ua.name".to_owned(), "Firefox".to_owned());
        extras.insert("ua.browser.family".to_owned(), "Firefox".to_owned());
        extras.insert("ua".to_owned(), ua.to_owned());
        extras.insert("uri.path".to_owned(), uri);

        for extra in extras.clone() {
            req.add_extra(extra.0.clone(), extra.1.clone())
        }

        assert_eq!(req.get_tags(), tags);
        assert_eq!(req.get_extras(), extras);
    }

    #[test]
    fn no_empty_tags() {
        use actix_web::{http::header, test::TestRequest};

        let uri = "/1.5/42/storage/meta/global".to_owned();
        let req = TestRequest::default()
            .uri(&uri)
            .header(
                header::USER_AGENT,
                header::HeaderValue::from_static("Mozilla/5.0 (curl) Gecko/20100101 curl"),
            )
            .to_http_request();
        add_initial_tags(&req, "GET".to_owned());
        add_initial_extras(&req, uri);

        assert!(!req.get_tags().contains_key("ua.os.ver"));
    }
}
