use actix_web::{dev::Payload, http::header::HeaderMap, Error, FromRequest, HttpRequest};
use futures::future::LocalBoxFuture;
use syncstorage_db::SyncTimestamp;

use crate::web::{error::ValidationErrorKind, extractors::RequestErrorLocation};

/// PreCondition Header
///
/// It's valid to include a X-If-Modified-Since or X-If-Unmodified-Since header but not
/// both.
///
/// Used with Option<PreConditionHeader> to extract a possible PreConditionHeader.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PreConditionHeader {
    IfModifiedSince(SyncTimestamp),
    IfUnmodifiedSince(SyncTimestamp),
    NoHeader,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreConditionHeaderOpt {
    pub opt: Option<PreConditionHeader>,
}

impl PreConditionHeaderOpt {
    pub fn extrude(headers: &HeaderMap) -> Result<Self, Error> {
        let modified = headers.get("X-If-Modified-Since");
        let unmodified = headers.get("X-If-Unmodified-Since");
        if modified.is_some() && unmodified.is_some() {
            // TODO: See following error,
            return Err(ValidationErrorKind::FromDetails(
                "conflicts with X-If-Modified-Since".to_owned(),
                RequestErrorLocation::Header,
                Some("X-If-Unmodified-Since".to_owned()),
                Some("request.validate.mod_header.conflict"),
            )
            .into());
        };
        let (value, field_name) = if let Some(modified_value) = modified {
            (modified_value, "X-If-Modified-Since")
        } else if let Some(unmodified_value) = unmodified {
            (unmodified_value, "X-If-Unmodified-Since")
        } else {
            return Ok(Self { opt: None });
        };
        if value
            .to_str()
            .unwrap_or("0.0")
            .parse::<f64>()
            .unwrap_or(0.0)
            < 0.0
        {
            // TODO: This is the right error, but it's not being returned correctly.
            return Err(ValidationErrorKind::FromDetails(
                "value is negative".to_owned(),
                RequestErrorLocation::Header,
                Some("X-If-Modified-Since".to_owned()),
                Some("request.validate.mod_header.negative"),
            )
            .into());
        }
        value
            .to_str()
            .map_err(|e| {
                ValidationErrorKind::FromDetails(
                    e.to_string(),
                    RequestErrorLocation::Header,
                    Some(field_name.to_owned()),
                    None,
                )
                .into()
            })
            .and_then(|v| {
                SyncTimestamp::from_header(v).map_err(|e| {
                    ValidationErrorKind::FromDetails(
                        e.to_string(),
                        RequestErrorLocation::Header,
                        Some(field_name.to_owned()),
                        None,
                    )
                    .into()
                })
            })
            .map(|v| {
                let header = if field_name == "X-If-Modified-Since" {
                    PreConditionHeader::IfModifiedSince(v)
                } else {
                    PreConditionHeader::IfUnmodifiedSince(v)
                };
                Self { opt: Some(header) }
            })
    }
}

impl FromRequest for PreConditionHeaderOpt {
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    /// Extract and validate the precondition headers
    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let req = req.clone();
        Box::pin(async move { Self::extrude(req.headers()) })
    }
}
