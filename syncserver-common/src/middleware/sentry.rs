use std::{cell::RefCell, collections::BTreeMap, marker::PhantomData, rc::Rc, sync::Arc};

use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    Error,
};
use cadence::{CountedExt, StatsdClient};
use futures::{future::LocalBoxFuture, FutureExt};
use futures_util::future::{ok, Ready};
use sentry::{protocol::Event, Hub};

use crate::{ReportableError, Taggable};

#[derive(Clone)]
pub struct SentryWrapper<E> {
    metrics: Arc<StatsdClient>,
    phantom: PhantomData<E>,
}

impl<E> SentryWrapper<E> {
    pub fn new(metrics: Arc<StatsdClient>) -> Self {
        Self {
            metrics,
            phantom: PhantomData,
        }
    }
}

impl<S, B, E> Transform<S, ServiceRequest> for SentryWrapper<E>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    E: ReportableError + actix_web::ResponseError + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = SentryWrapperMiddleware<S, E>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(SentryWrapperMiddleware {
            service: Rc::new(RefCell::new(service)),
            metrics: self.metrics.clone(),
            phantom: PhantomData,
        })
    }
}

#[derive(Debug)]
pub struct SentryWrapperMiddleware<S, E> {
    service: Rc<RefCell<S>>,
    metrics: Arc<StatsdClient>,
    phantom: PhantomData<E>,
}

impl<S, B, E> Service<ServiceRequest> for SentryWrapperMiddleware<S, E>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    E: ReportableError + actix_web::ResponseError + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    actix_web::dev::forward_ready!(service);

    fn call(&self, sreq: ServiceRequest) -> Self::Future {
        // Set up the hub to add request data to events
        let hub = Hub::new_from_top(Hub::main());
        let _ = hub.push_scope();
        let sentry_request = sentry_request_from_http(&sreq);
        hub.configure_scope(|scope| {
            scope.add_event_processor(Box::new(move |event| process_event(event, &sentry_request)))
        });

        // get the tag information
        let metrics = self.metrics.clone();
        let tags = sreq.get_tags();
        let extras = sreq.get_extras();

        let fut = self.service.call(sreq);

        async move {
            let response: Self::Response = match fut.await {
                Ok(response) => response,
                Err(error) => {
                    if let Some(reportable_err) = error.as_error::<E>() {
                        // if it's not reportable, and we have access to the metrics, record it as a metric.
                        if !reportable_err.is_sentry_event() {
                            // The error (e.g. VapidErrorKind::InvalidKey(String)) might be too cardinal,
                            // but we may need that information to debug a production issue. We can
                            // add an info here, temporarily turn on info level debugging on a given server,
                            // capture it, and then turn it off before we run out of money.
                            maybe_emit_metrics(&metrics, reportable_err);
                            debug!("Sentry: Not reporting error (service error): {:?}", error);
                            return Err(error);
                        }
                    };
                    debug!("Reporting error to Sentry (service error): {}", error);
                    let mut event = event_from_actix_error::<E>(&error);
                    // Add in the tags from the request
                    event.tags.extend(tags);
                    event.extra.extend(extras);
                    let event_id = hub.capture_event(event);
                    trace!("event_id = {}", event_id);
                    return Err(error);
                }
            };
            // Check for errors inside the response
            if let Some(error) = response.response().error() {
                if let Some(reportable_err) = error.as_error::<E>() {
                    if !reportable_err.is_sentry_event() {
                        maybe_emit_metrics(&metrics, reportable_err);
                        debug!("Not reporting error (service error): {:?}", error);
                        return Ok(response);
                    }
                }
                debug!("Reporting error to Sentry (response error): {}", error);
                let event = event_from_actix_error::<E>(error);
                let event_id = hub.capture_event(event);
                trace!("event_id = {}", event_id);
            }
            Ok(response)
        }
        .boxed_local()
    }
}

/// Emit metrics when a [ReportableError::metric_label] is returned
fn maybe_emit_metrics<E>(metrics: &StatsdClient, err: &E)
where
    E: ReportableError,
{
    let Some(label) = err.metric_label() else {
        return;
    };
    debug!("Sending error to metrics: {:?}", err);
    let mut builder = metrics.incr_with_tags(label);
    let tags = err.tags();
    for (key, val) in &tags {
        builder = builder.with_tag(key, val);
    }
    builder.send();
}

/// Build a Sentry request struct from the HTTP request
fn sentry_request_from_http(request: &ServiceRequest) -> sentry::protocol::Request {
    sentry::protocol::Request {
        url: format!(
            "{}://{}{}",
            request.connection_info().scheme(),
            request.connection_info().host(),
            request.uri()
        )
        .parse()
        .ok(),
        method: Some(request.method().to_string()),
        headers: request
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or_default().to_string()))
            .collect(),
        ..Default::default()
    }
}

/// Add request data to a Sentry event
#[allow(clippy::unnecessary_wraps)]
fn process_event(
    mut event: Event<'static>,
    request: &sentry::protocol::Request,
) -> Option<Event<'static>> {
    if event.request.is_none() {
        event.request = Some(request.clone());
    }

    // TODO: Use ServiceRequest::match_pattern for the event transaction.
    //       Coming in Actix v3.

    Some(event)
}

/// Convert Actix errors into a Sentry event. ReportableError is handled
/// explicitly so the event can include a backtrace and source error
/// information.
fn event_from_actix_error<E>(error: &actix_web::Error) -> sentry::protocol::Event<'static>
where
    E: ReportableError + actix_web::ResponseError + 'static,
{
    // Actix errors don't have support source/cause, so to get more information
    // about the error we need to downcast.
    if let Some(reportable_err) = error.as_error::<E>() {
        // Use our error and associated backtrace for the event
        event_from_error(reportable_err)
    } else {
        // Fallback to the Actix error
        sentry::event_from_error(error)
    }
}

/// Custom `sentry::event_from_error` for `ReportableError`
///
/// `std::error::Error` doesn't support backtraces, thus `sentry::event_from_error`
/// doesn't either. This function works against `ReportableError` instead to
/// extract backtraces, etc. from it and its chain of `reportable_source's.
///
/// A caveat of this function is that it cannot extract
/// `ReportableError`s/backtraces, etc. that occur in a chain after a
/// `std::error::Error` occurs: as `std::error::Error::source` only allows
/// downcasting to a concrete type, not `dyn ReportableError`.
pub fn event_from_error(
    mut reportable_err: &dyn ReportableError,
) -> sentry::protocol::Event<'static> {
    let mut exceptions = vec![];
    let mut tags = BTreeMap::new();
    let mut extra = BTreeMap::new();

    // Gather reportable_source()'s for their backtraces, etc
    loop {
        exceptions.push(exception_from_reportable_error(reportable_err));
        for (k, v) in reportable_err.tags() {
            // NOTE: potentially overwrites other tags/extras from this chain
            tags.insert(k.to_owned(), v);
        }
        for (k, v) in reportable_err.extras() {
            extra.insert(k.to_owned(), v);
        }
        reportable_err = match reportable_err.reportable_source() {
            Some(reportable_err) => reportable_err,
            None => break,
        };
    }

    // Then fallback to source() for remaining Errors
    let mut source = reportable_err.reportable_source();
    while let Some(err) = source {
        exceptions.push(exception_from_reportable_error(err));
        source = err.reportable_source();
    }

    exceptions.reverse();
    sentry::protocol::Event {
        exception: exceptions.into(),
        level: sentry::protocol::Level::Error,
        tags,
        extra,
        ..Default::default()
    }
}

/// Custom `exception_from_error` support function for `ReportableError`
///
/// Based moreso on sentry_failure's `exception_from_single_fail`. Includes a
/// stacktrace if available.
pub fn exception_from_reportable_error(err: &dyn ReportableError) -> sentry::protocol::Exception {
    let dbg = format!("{:?}", &err);
    sentry::protocol::Exception {
        ty: sentry::parse_type_from_debug(&dbg).to_owned(),
        value: Some(err.to_string()),
        stacktrace: err
            .backtrace()
            .map(sentry_backtrace::backtrace_to_stacktrace)
            .unwrap_or_default(),
        ..Default::default()
    }
}
