#![allow(clippy::type_complexity)]
use std::task::{Context, Poll};

use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    http::header::USER_AGENT,
    web::Data,
    Error, HttpResponse,
};
use futures::future::{self, Either, Ready};
use lazy_static::lazy_static;
use regex::Regex;

use crate::server::{metrics::Metrics, ServerState};

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
    type Future = Either<Ready<Result<Self::Response, Self::Error>>, S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, sreq: ServiceRequest) -> Self::Future {
        match sreq.headers().get(USER_AGENT) {
            Some(header) if header.to_str().map_or(false, should_reject) => {
                let data = sreq
                    .app_data::<Data<ServerState>>()
                    .expect("No app_data ServerState");
                trace!("Rejecting User-Agent: {:?}", header);
                Metrics::from(data.get_ref()).incr("error.rejectua");

                Either::Left(future::ok(
                    sreq.into_response(
                        HttpResponse::ServiceUnavailable()
                            .body("0".to_owned())
                            .into_body(),
                    ),
                ))
            }
            _ => Either::Right(self.service.call(sreq)),
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
