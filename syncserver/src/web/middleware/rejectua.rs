#![allow(clippy::type_complexity)]
use std::task::{Context, Poll};

use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    http::header::USER_AGENT,
    Error, FromRequest, HttpResponse,
};
use futures::future::{self, LocalBoxFuture, Ready};
use lazy_static::lazy_static;
use regex::Regex;

use crate::error::{ApiError, ApiErrorKind};
use crate::server::MetricsWrapper;

lazy_static! {
    // e.g. "Firefox-iOS-Sync/18.0b1 (iPhone; iPhone OS 13.2.2) (Fennec (synctesting))"
    // https://github.com/mozilla-mobile/firefox-ios/blob/v19.x/Shared/UserAgent.swift#L12
    static ref IOS_UA_REGEX: Regex = Regex::new(
        r"(?x)
^
Firefox-iOS-Sync/
(?P<major>[0-9]+)\.[.0-9]+    # <appVersion-major>.<appVersion-minor-etc>
b.*                           # b<buildNumber>
\s\(.+                        #  (<deviceModel>
;\siPhone\sOS                 # ; iPhone OS
\s.+\)                        #  <systemVersion>)
\s\(.*\)                      #  (<displayName>)
$
"
    )
    .unwrap();
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Default)]
pub struct RejectUA;

impl<S, B> Transform<S> for RejectUA
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = RejectUAMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        future::ok(RejectUAMiddleware { service })
    }
}
#[allow(clippy::upper_case_acronyms)]
pub struct RejectUAMiddleware<S> {
    service: S,
}

impl<S, B> Service for RejectUAMiddleware<S>
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
        match sreq.headers().get(USER_AGENT).cloned() {
            Some(header) if header.to_str().map_or(false, should_reject) => Box::pin(async move {
                trace!("Rejecting User-Agent: {:?}", header);
                let (req, payload) = sreq.into_parts();
                MetricsWrapper::extract(&req)
                    .await?
                    .0
                    .incr("error.rejectua");
                let sreq = ServiceRequest::from_parts(req, payload).map_err(|_| {
                    ApiError::from(ApiErrorKind::Internal(
                        "failed to reconstruct ServiceRequest from its parts".to_owned(),
                    ))
                })?;

                Ok(sreq.into_response(
                    HttpResponse::ServiceUnavailable()
                        .body("0".to_owned())
                        .into_body(),
                ))
            }),
            _ => Box::pin(self.service.call(sreq)),
        }
    }
}

/// Determine if a User-Agent should be rejected w/ an error response.
///
/// firefox-ios < v20 suffers from a bug where our response headers
/// can cause it to crash. They're sent an error response instead that
/// avoids the crash.
///
/// Dev builds were originally labeled as v0 (or now "Firefox-iOS-Sync/dev") so
/// we don't reject those.
///
/// https://github.com/mozilla-services/syncstorage-rs/issues/293
fn should_reject(ua: &str) -> bool {
    let major = IOS_UA_REGEX
        .captures(ua)
        .and_then(|captures| captures.name("major"))
        .and_then(|major| major.as_str().parse::<u32>().ok())
        .unwrap_or(20);
    0 < major && major < 20
}
