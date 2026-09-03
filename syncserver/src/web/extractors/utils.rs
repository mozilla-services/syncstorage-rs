use std::str::FromStr;

use actix_web::{
    HttpRequest,
    http::header::{self, Accept, Header, QualityItem},
};
use mime::STAR_STAR;
use serde::{Deserialize, Serialize};

use crate::{
    error::{ApiError, ApiErrorKind},
    web::error::{ValidationError, ValidationErrorKind},
};

/// UID parameter from URL path
#[allow(dead_code)] // Not really dead, but Rust can't see the deserialized use.
#[derive(Deserialize)]
pub struct UidParam {
    pub uid: u64,
}

/// Validation Error Location in the request
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RequestErrorLocation {
    Body,
    QueryString,
    Url,
    Header,
    Path,
    Cookies,
    Method,
    Unknown,
}

pub fn urldecode(s: &str) -> Result<String, ApiError> {
    let decoded: String = urlencoding::decode(s)
        .map_err(|e| {
            trace!("Extract: unclean urldecode entry: {:?} {:?}", s, e);
            ApiErrorKind::Internal(e.to_string())
        })?
        .into_owned();
    Ok(decoded)
}

// This tries to do the right thing to get the Accepted header according to
// https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Accept, but some corners can absolutely be cut.
// This will pull the first accepted content type listed, or the highest rated non-accepted type.
pub fn get_accepted(req: &HttpRequest, accepted: &[&str], default: &'static str) -> String {
    let mut candidates = Accept::parse(req).unwrap_or_else(|_| {
        Accept(vec![QualityItem::max(
            mime::Mime::from_str(default).expect("Could not get accept in get_accepted"),
        )])
    });
    if candidates.is_empty() {
        return default.to_owned();
    }
    candidates.sort_by(|a, b| {
        b.quality
            .partial_cmp(&a.quality)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for qitem in candidates.iter().cloned() {
        if qitem.item == STAR_STAR {
            return default.to_owned();
        }
        let lc = qitem.item.to_string().to_lowercase();
        if accepted.contains(&lc.as_str()) {
            return lc;
        }
    }
    "invalid".to_string()
}

/// Reject when the request advertises more bytes than this
/// collection allows. Actix's resource-level `PayloadConfig` is
/// sized to the *maximum* `max_request_bytes` across all overrides;
/// this check enforces the *per-collection* ceiling.
pub fn check_content_length(
    req: &HttpRequest,
    max_request_bytes: usize,
) -> Result<(), ValidationError> {
    if let Some(len) = req
        .headers()
        .get(header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<usize>().ok())
        && len > max_request_bytes
    {
        return Err(size_limit_exceeded());
    }
    Ok(())
}

/// The 413 for a request larger than its collection's `max_request_bytes`.
pub fn size_limit_exceeded() -> ValidationError {
    ValidationErrorKind::FromDetails(
        "size-limit-exceeded".to_owned(),
        RequestErrorLocation::Header,
        Some("Content-Length".to_owned()),
        Some("request.validate.request_bytes_exceeded"),
    )
    .into()
}

#[cfg(test)]
mod tests {
    use actix_web::{
        http::header::{ACCEPT, HeaderValue},
        test::TestRequest,
    };

    use super::get_accepted;
    use crate::web::extractors::ACCEPTED_CONTENT_TYPES;

    #[test]
    fn test_weighted_header() {
        // test non-priority, full weight selection
        let req = TestRequest::default().insert_header((
            ACCEPT,
            HeaderValue::from_static("application/json;q=0.9,text/plain"),
        ));
        let selected = get_accepted(
            &req.to_http_request(),
            &ACCEPTED_CONTENT_TYPES,
            "application/json",
        );
        assert_eq!(selected, "text/plain".to_owned());

        // test default for */*
        let req = TestRequest::default()
            .insert_header((ACCEPT, HeaderValue::from_static("*/*;q=0.2,foo/bar")));
        let selected = get_accepted(
            &req.to_http_request(),
            &ACCEPTED_CONTENT_TYPES,
            "application/json",
        );
        assert_eq!(selected, "application/json".to_owned());

        // test default for selected weighted.
        let req = TestRequest::default().insert_header((
            ACCEPT,
            HeaderValue::from_static("foo/bar;q=0.1,application/json;q=0.5,text/plain;q=0.9"),
        ));
        let selected = get_accepted(
            &req.to_http_request(),
            &ACCEPTED_CONTENT_TYPES,
            "application/json",
        );
        assert_eq!(selected, "text/plain".to_owned());
    }
}
