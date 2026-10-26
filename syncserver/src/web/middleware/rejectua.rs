#![allow(clippy::type_complexity)]

use actix_web::{
    FromRequest, HttpResponse,
    body::EitherBody,
    dev::{Service, ServiceRequest, ServiceResponse},
    http::header::USER_AGENT,
};
use futures::future::LocalBoxFuture;
use lazy_static::lazy_static;
use regex::Regex;

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

pub fn reject_user_agent<B>(
    request: ServiceRequest,
    service: &(
         impl Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error> + 'static
     ),
) -> LocalBoxFuture<'static, Result<ServiceResponse<EitherBody<B>>, actix_web::Error>> {
    match request.headers().get(USER_AGENT).cloned() {
        Some(header) if header.to_str().is_ok_and(should_reject) => Box::pin(async move {
            trace!("Rejecting User-Agent: {:?}", header);
            let (req, payload) = request.into_parts();
            MetricsWrapper::extract(&req)
                .await?
                .0
                .incr("error.rejectua");
            let sreq = ServiceRequest::from_parts(req, payload);

            Ok(sreq.into_response(
                HttpResponse::ServiceUnavailable()
                    .body("0")
                    .map_into_right_body(),
            ))
        }),
        _ => {
            let fut = service.call(request);
            Box::pin(async move { fut.await.map(|resp| resp.map_into_left_body()) })
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

#[cfg(test)]
mod tests {
    use super::should_reject;

    fn ios_ua(version: &str) -> String {
        format!("Firefox-iOS-Sync/{version} (iPhone; iPhone OS 13.2.2) (Firefox)")
    }

    #[test]
    fn test_reject_ios_below_v20() {
        for version in ["1.0b1", "18.0b1", "19.0b1", "19.9.1b12345"] {
            assert!(should_reject(&ios_ua(version)), "{version}");
        }
    }

    #[test]
    fn test_accept_ios_v20_and_above() {
        for version in ["20.0b1", "20.1b1", "23.0b17297", "100.0b1"] {
            assert!(!should_reject(&ios_ua(version)), "{version}");
        }
    }

    #[test]
    fn test_accept_ios_dev_builds() {
        assert!(!should_reject(&ios_ua("0.0.1b1")));
    }

    #[test]
    fn test_accept_unmatched_user_agents() {
        for ua in [
            "Mozilla/5.0 (X11; Linux x86_64; rv:140.0) Gecko/20100101 Firefox/140.0",
            // dev build with no numeric version
            &ios_ua("dev"),
            // no build number
            "Firefox-iOS-Sync/18.0 (iPhone; iPhone OS 13.2.2) (Firefox)",
            // no iPhone OS system version
            "Firefox-iOS-Sync/18.0b1 (iPhone) (Firefox)",
            // major version overflows u32
            &ios_ua("99999999999.0b1"),
        ] {
            assert!(!should_reject(ua), "{ua}");
        }
    }
}
