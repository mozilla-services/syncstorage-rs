use std::collections::HashMap;
use std::task::{Context, Poll};

use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    http::StatusCode,
    Error, HttpResponse,
};
use futures::future::{self, Either, Ready};
use serde::{Deserialize, Serialize};

// Respond to "x-debug-return" header requests.

#[derive(Debug, Default)]
pub struct DebugClient;

impl<S, B> Transform<S> for DebugClient
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = DebugClientMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        future::ok(DebugClientMiddleware { service })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RespondWith {
    status: u16,
    code: Option<u8>,
    message: Option<String>,
    headers: Option<HashMap<String, String>>,
}

impl Default for RespondWith {
    fn default() -> Self {
        Self {
            status: 200,
            code: None,
            message: None,
            headers: None,
        }
    }
}

pub struct DebugClientMiddleware<S> {
    service: S,
}

impl<S, B> Service for DebugClientMiddleware<S>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Either<Ready<Result<Self::Response, Self::Error>>, S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, sreq: ServiceRequest) -> Self::Future {
        if let Some(header) = sreq.headers().get("Client-Debug".to_owned()) {
            debug!("### Providing debug header {:?}", header);
            let resp_data: RespondWith =
                serde_json::from_str(header.to_str().unwrap_or_default()).expect("Invalid header");
            let mut builder = HttpResponse::build(
                StatusCode::from_u16(resp_data.status).unwrap_or(StatusCode::BAD_REQUEST),
            );
            if let Some(headers) = resp_data.headers {
                for (key, value) in headers {
                    builder.set_header(&key, value);
                }
            }
            builder.set_header("debug", "client");
            let response = if let Some(code) = resp_data.code {
                builder
                    .content_type("application/json")
                    .body(format!("{:?}", code))
                    .into_body()
            } else if let Some(message) = resp_data.message {
                builder
                    .content_type("application/json")
                    .body(format!("{:?}", message))
                    .into_body()
            } else {
                builder.body("").into_body()
            };
            return Either::Left(future::ok(sreq.into_response(response)));
        }
        Either::Right(self.service.call(sreq))
    }
}
