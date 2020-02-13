use std::task::Context;
use std::{cell::RefCell, rc::Rc};

use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage,
};
use futures::future::{self, LocalBoxFuture, TryFutureExt};
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
            // Fetch out the tags (in case any have been added.)
            match sresp.response().error() {
                None => {}
                Some(e) => {
                    // The extensions defined in the request do not get populated
                    // into the response. There can be two different, and depending
                    // on where a tag may be set, only one set may be available.
                    // Base off of the request, then overwrite/suppliment with the
                    // response.
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
                    // deriving the sentry event from a fail directly from the error
                    // is not currently thread safe. Downcasting the error to an
                    // ApiError resolves this.
                    let apie: Option<&ApiError> = e.as_error();
                    if let Some(apie) = apie {
                        let mut event = sentry::integrations::failure::event_from_fail(apie);
                        event.tags = tags.clone().tag_tree();
                        event.extra = tags.extra_tree();
                        sentry::capture_event(event);
                    }
                }
            }
            future::ok(sresp)
        }))
    }
}
