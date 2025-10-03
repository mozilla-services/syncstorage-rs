use actix_web::{dev::Payload, http::header::HeaderMap, Error, FromRequest, HttpRequest};
use futures::future::LocalBoxFuture;

use syncstorage_db::SyncTimestamp;

use super::RequestErrorLocation;
use crate::web::error::ValidationErrorKind;

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
    #[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use actix_web::{dev::ServiceResponse, test::TestRequest, HttpResponse};

    use syncstorage_db::SyncTimestamp;

    use super::{PreConditionHeader, PreConditionHeaderOpt};
    use crate::web::extractors::test_utils::{extract_body_as_str, make_state};

    #[test]
    fn test_invalid_precondition_headers() {
        fn assert_invalid_header(
            req: actix_web::HttpRequest,
            _error_header: &str,
            _error_message: &str,
        ) {
            let result = PreConditionHeaderOpt::extrude(req.headers());
            assert!(result.is_err());
            let response: HttpResponse = result.err().unwrap().into();
            assert_eq!(response.status(), 400);
            let body = extract_body_as_str(ServiceResponse::new(req, response));

            assert_eq!(body, "0");

            /* New tests for when we can use descriptive errors
            let err: serde_json::Value = serde_json::from_str(&body).unwrap();
            assert_eq!(err["status"], 400);

            assert_eq!(err["errors"][0]["description"], error_message);
            assert_eq!(err["errors"][0]["location"], "header");
            assert_eq!(err["errors"][0]["name"], error_header);
            */
        }
        let req = TestRequest::with_uri("/")
            .data(make_state())
            .insert_header(("X-If-Modified-Since", "32124.32"))
            .insert_header(("X-If-Unmodified-Since", "4212.12"))
            .to_http_request();
        assert_invalid_header(
            req,
            "X-If-Unmodified-Since",
            "conflicts with X-If-Modified-Since",
        );
        let req = TestRequest::with_uri("/")
            .data(make_state())
            .insert_header(("X-If-Modified-Since", "-32.1"))
            .to_http_request();
        assert_invalid_header(req, "X-If-Modified-Since", "Invalid value");
    }

    #[test]
    fn test_valid_precondition_headers() {
        let req = TestRequest::with_uri("/")
            .data(make_state())
            .insert_header(("X-If-Modified-Since", "32.1"))
            .to_http_request();
        let result = PreConditionHeaderOpt::extrude(req.headers())
            .unwrap()
            .opt
            .unwrap();
        assert_eq!(
            result,
            PreConditionHeader::IfModifiedSince(SyncTimestamp::from_seconds(32.1))
        );
        let req = TestRequest::with_uri("/")
            .data(make_state())
            .insert_header(("X-If-Unmodified-Since", "32.14"))
            .to_http_request();
        let result = PreConditionHeaderOpt::extrude(req.headers())
            .unwrap()
            .opt
            .unwrap();
        assert_eq!(
            result,
            PreConditionHeader::IfUnmodifiedSince(SyncTimestamp::from_seconds(32.14))
        );
    }
}
