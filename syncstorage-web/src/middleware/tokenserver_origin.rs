use std::{
    collections::HashMap,
    task::{Context, Poll},
};

use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    web::Data,
    Error,
};
use futures::future::{self, LocalBoxFuture};
use syncserver_common::Metrics;
use tokenserver_auth::TokenserverOrigin;

use crate::{
    error::{ApiError, ApiErrorKind},
    ServerState,
};

pub struct EmitTokenserverOriginMiddleware<S> {
    service: S,
}

impl<S, B> Service for EmitTokenserverOriginMiddleware<S>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
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
        let fut = self.service.call(sreq);

        Box::pin(async move {
            let res = fut.await?;
            let req = res.request();
            let metrics = {
                let statsd_client = req
                    .app_data::<Data<ServerState>>()
                    .map(|state| state.statsd_client.clone())
                    .ok_or_else(|| ApiError::from(ApiErrorKind::NoServerState))?;

                Metrics::from(&*statsd_client)
            };

            let mut tags = HashMap::new();

            if let Some(origin) = req.extensions().get::<TokenserverOrigin>().copied() {
                tags.insert("tokenserver_origin".to_string(), origin.to_string());
            }

            if res.status().is_informational() {
                metrics.incr_with_tags("http_1XX", tags);
            } else if res.status().is_success() {
                metrics.incr_with_tags("http_2XX", tags);
            } else if res.status().is_redirection() {
                metrics.incr_with_tags("http_3XX", tags);
            } else if res.status().is_client_error() {
                metrics.incr_with_tags("http_4XX", tags);
            } else if res.status().is_server_error() {
                metrics.incr_with_tags("http_5XX", tags);
            }

            Ok(res)
        })
    }
}

#[derive(Default)]
pub struct EmitTokenserverOrigin;

impl<S: 'static, B> Transform<S> for EmitTokenserverOrigin
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = EmitTokenserverOriginMiddleware<S>;
    type Future = LocalBoxFuture<'static, Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        Box::pin(future::ok(EmitTokenserverOriginMiddleware { service }))
    }
}
