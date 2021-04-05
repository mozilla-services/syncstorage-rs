use std::task::Context;
use std::{
    cell::{RefCell, RefMut},
    rc::Rc,
};

use actix_http::Extensions;
use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    web::Data,
    Error, HttpMessage,
};
use futures::future::{self, LocalBoxFuture, TryFutureExt};
use sentry::protocol::Event;
use std::task::Poll;

use crate::error::ApiError;
use crate::server::{metrics::Metrics, ServerState};
use crate::web::tags::Tags;

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

pub fn queue_report(mut ext: RefMut<'_, Extensions>, err: &Error) {
    let apie: Option<&ApiError> = err.as_error();
    if let Some(apie) = apie {
        if !apie.is_reportable() {
            trace!("Sentry Not reporting error: {:?}", err);
            return;
        }
        let event = sentry::integrations::failure::event_from_fail(apie);
        if let Some(events) = ext.get_mut::<Vec<Event<'static>>>() {
            events.push(event);
        } else {
            let events: Vec<Event<'static>> = vec![event];
            ext.insert(events);
        }
    }
}

pub fn report(tags: &Tags, mut event: Event<'static>) {
    let tags = tags.clone();
    event.tags = tags.clone().tag_tree();
    event.extra = tags.extra_tree();
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
        let mut tags = Tags::from(sreq.head());
        sreq.extensions_mut().insert(tags.clone());
        let metrics = if let Some(state) = sreq.app_data::<Data<ServerState>>() {
            Some(Metrics::from(state.get_ref()))
        } else {
            None
        };

        Box::pin(self.service.call(sreq).and_then(move |mut sresp| {
            // handed an actix_error::error::Error;
            // Fetch out the tags (in case any have been added.) NOTE: request extensions
            // are NOT automatically passed to responses. You need to check both.
            if let Some(t) = sresp.request().extensions().get::<Tags>() {
                trace!("Sentry: found tags in request: {:?}", &t.tags);
                for (k, v) in t.tags.clone() {
                    tags.tags.insert(k, v);
                }
                for (k, v) in t.extra.clone() {
                    tags.extra.insert(k, v);
                }
            };
            if let Some(t) = sresp.response().extensions().get::<Tags>() {
                trace!("Sentry: found tags in response: {:?}", &t.tags);
                for (k, v) in t.tags.clone() {
                    tags.tags.insert(k, v);
                }
                for (k, v) in t.extra.clone() {
                    tags.extra.insert(k, v);
                }
            };
            //dbg!(&tags);
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
                            report(&tags, event);
                        }
                    }
                    if let Some(events) = sresp
                        .response_mut()
                        .extensions_mut()
                        .remove::<Vec<Event<'static>>>()
                    {
                        for event in events {
                            trace!("Sentry: Found an error stored in response: {:?}", &event);
                            report(&tags, event);
                        }
                    }
                }
                Some(e) => {
                    if let Some(apie) = e.as_error::<ApiError>() {
                        if let Some(metrics) = metrics {
                            if let Some(label) = apie.kind().metric_label() {
                                metrics.incr(&label);
                            }
                        }
                        if !apie.is_reportable() {
                            trace!("Sentry: Not reporting error: {:?}", apie);
                            return future::ok(sresp);
                        }
                        report(&tags, sentry::integrations::failure::event_from_fail(apie));
                    }
                }
            }
            future::ok(sresp)
        }))
    }
}
