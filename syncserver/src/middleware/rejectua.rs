#![allow(clippy::type_complexity)]

use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse},
    http::header::USER_AGENT,
    FromRequest, HttpResponse,
};
use futures::future::LocalBoxFuture;
use lazy_static::lazy_static;
use regex::Regex;

use crate::error::{ApiError, ApiErrorKind};
use crate::server::{MetricsWrapper, ServerState};

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

pub fn reject_user_agent(
    request: ServiceRequest,
    service: &mut (impl Service<
        Request = ServiceRequest,
        Response = ServiceResponse,
        Error = actix_web::Error,
    > + 'static),
) -> LocalBoxFuture<'static, Result<ServiceResponse, actix_web::Error>> {
    match request.headers().get(USER_AGENT).cloned() {
        Some(header) if header.to_str().map_or(false, should_reject) => Box::pin(async move {
            trace!("Rejecting User-Agent: {:?}", header);
            let (req, payload) = request.into_parts();
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
        _ => Box::pin(service.call(request)),
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
