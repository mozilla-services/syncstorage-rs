use actix_web::{
    http::header::{Accept, Header, QualityItem},
    HttpRequest,
};
use mime::STAR_STAR;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use crate::error::{ApiError, ApiErrorKind};

/// UID parameter from URL path
#[derive(Deserialize)]
pub struct UidParam {
    #[allow(dead_code)] // Not really dead, but Rust can't see the deserialized use.
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
