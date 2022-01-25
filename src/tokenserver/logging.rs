use std::task::Context;
use std::{cell::RefCell, rc::Rc};

use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage,
};
use futures::future::{self, LocalBoxFuture, TryFutureExt};
use std::task::Poll;

use super::LogItems;

#[derive(Default)]
pub struct LoggingWrapper;

impl LoggingWrapper {
    pub fn new() -> Self {
        LoggingWrapper::default()
    }
}

impl<S, B> Transform<S> for LoggingWrapper
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = LoggingWrapperMiddleware<S>;
    type Future = LocalBoxFuture<'static, Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        Box::pin(future::ok(LoggingWrapperMiddleware {
            service: Rc::new(RefCell::new(service)),
        }))
    }
}

#[derive(Debug)]
pub struct LoggingWrapperMiddleware<S> {
    service: Rc<RefCell<S>>,
}

impl<S, B> Service for LoggingWrapperMiddleware<S>
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
        let items = LogItems::from(sreq.head());
        sreq.extensions_mut().insert(items);

        Box::pin(self.service.call(sreq).and_then(move |sresp| {
            if let Some(items) = sresp.request().extensions().get::<LogItems>() {
                info!("{}", items);
            }

            future::ok(sresp)
        }))
    }
}
