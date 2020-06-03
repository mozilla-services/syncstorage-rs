use std::task::Context;
use std::{
    cell::{RefCell, RefMut},
    rc::Rc,
};

use actix_http::Extensions;
use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage,
};
use futures::future::{self, LocalBoxFuture, TryFutureExt};
use sentry::protocol::Event;
use std::task::Poll;

use crate::error::ApiError;
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
            debug!("Not reporting error: {:?}", err);
            return;
        }
        let event = sentry::integrations::failure::event_from_fail(apie);
        if let Some(events) = ext.get_mut::<Vec<Event<'static>>>() {
            events.push(event);
        } else {
            let mut events: Vec<Event<'static>> = Vec::new();
            events.push(event);
            ext.insert(events);
        }
    }
}

pub fn report(tags: &Tags, mut event: Event<'static>) {
    let tags = tags.clone();
    event.tags = tags.clone().tag_tree();
    event.extra = tags.extra_tree();
    debug!("Sending error to sentry: {:?}", &event);
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
        let mut tags = Tags::from_request_head(sreq.head());
        let uri = sreq.head().uri.to_string();
        sreq.extensions_mut().insert(tags.clone());

        Box::pin(self.service.call(sreq).and_then(move |sresp| {
            // handed an actix_error::error::Error;
            // Fetch out the tags (in case any have been added.) NOTE: request extensions
            // are NOT automatically passed to responses. You need to check both.
            if let Some(t) = sresp.request().extensions().get::<Tags>() {
                debug!("Found request tags: {:?}", &t.tags);
                for (k, v) in t.tags.clone() {
                    tags.tags.insert(k, v);
                }
            };
            if let Some(t) = sresp.response().extensions().get::<Tags>() {
                debug!("Found response tags: {:?}", &t.tags);
                for (k, v) in t.tags.clone() {
                    tags.tags.insert(k, v);
                }
            };
            // add the uri.path (which can cause influx to puke)
            tags.extra.insert("uri.path".to_owned(), uri);
            match sresp.response().error() {
                None => {
                    // Middleware errors are eaten by current versions of Actix. Errors are now added
                    // to the extensions. Need to check both for any errors and report them.
                    if let Some(events) = sresp.request().extensions().get::<Vec<Event<'static>>>()
                    {
                        for event in events.clone() {
                            debug!("Found an error in request: {:?}", &event);
                            report(&tags, event);
                        }
                    }
                    if let Some(events) = sresp.response().extensions().get::<Vec<Event<'static>>>()
                    {
                        for event in events.clone() {
                            debug!("Found an error in response: {:?}", &event);
                            report(&tags, event);
                        }
                    }
                }
                Some(e) => {
                    let apie: Option<&ApiError> = e.as_error();
                    if let Some(apie) = apie {
                        if !apie.is_reportable() {
                            debug!("Not reporting error to sentry: {:?}", apie);
                            return future::ok(sresp);
                        }
                    }
                    if let Some(apie) = apie {
                        report(&tags, sentry::integrations::failure::event_from_fail(apie));
                    }
                }
            }
            future::ok(sresp)
        }))
    }
}
