//! Request header/body/query extractors
//!
//! Handles ensuring the header's, body, and query parameters are correct, extraction to
//! relevant types, and failing correctly with the appropriate errors if issues arise.
use std::error;
use std::fmt;
use std::str::FromStr;

use actix_web::http::StatusCode;
use actix_web::{Error, FromRequest, HttpRequest, HttpResponse, Path, Query, ResponseError, State};
use futures::{future, Future};
use num::Zero;
use regex::Regex;
use serde::de::{Deserialize, Deserializer, Error as SerdeError};
use serde_json::from_value;
use validator::{Validate, ValidationError, ValidationErrors};

use server::ServerState;
use settings::Settings;
use web::auth::HawkIdentifier;

const BATCH_MAX_IDS: usize = 100;

lazy_static! {
    static ref KNOWN_BAD_PAYLOAD_REGEX: Regex =
        Regex::new(r#"IV":\s*"AAAAAAAAAAAAAAAAAAAAAA=="#).unwrap();
    static ref VALID_ID_REGEX: Regex = Regex::new(r"^[ -~]{1,64}$").unwrap();
}

// XXX: Convert these to full extractors.
pub type GetCollectionRequest = (Path<CollectionParams>, HawkIdentifier, State<ServerState>);
pub type BsoRequest = (Path<BsoParams>, HawkIdentifier, State<ServerState>);

#[derive(Deserialize)]
pub struct UidParam {
    #[allow(dead_code)] // Not really dead, but Rust can't see the deserialized use.
    uid: String,
}

#[derive(Deserialize)]
pub struct CollectionParams {
    pub uid: String,
    pub collection: String,
}

#[derive(Deserialize)]
pub struct BsoParams {
    pub uid: String,
    pub collection: String,
    pub bso: String,
}

#[derive(Deserialize, Serialize)]
pub struct BsoBody {
    pub sortindex: Option<i32>,
    pub payload: Option<String>,
    pub ttl: Option<u32>,
}

/// Request arguments needed for Information Requests
///
/// Only the database and user identifier is required for information
/// requests: https://mozilla-services.readthedocs.io/en/latest/storage/apis-1.5.html#general-info
pub struct MetaRequest {
    pub state: State<ServerState>,
    pub user_id: HawkIdentifier,
}

impl FromRequest<ServerState> for MetaRequest {
    type Config = Settings;
    type Result = Box<Future<Item = MetaRequest, Error = Error>>;

    fn from_request(req: &HttpRequest<ServerState>, _: &Self::Config) -> Self::Result {
        Box::new(
            <(Path<UidParam>, HawkIdentifier, State<ServerState>)>::extract(req).and_then(
                |(_path, auth, state)| {
                    future::ok(MetaRequest {
                        state,
                        user_id: auth,
                    })
                },
            ),
        )
    }
}

/// Validator to extract BSO search parameters from the query string.
///
/// This validator will extract and validate the following search params used in
/// multiple handler functions. Not all query params are used in each handler.
#[derive(Debug, Default, Deserialize, Validate)]
#[serde(default)]
pub struct BsoQueryParams {
    /// lower-bound on last-modified time (float timestamp)
    #[validate(custom = "validate_qs_positive_value")]
    pub lower: Option<f64>,

    /// upper-bound on last-modified time (float timestamp)
    #[validate(custom = "validate_qs_positive_value")]
    pub older: Option<f64>,

    /// order in which to return results (string)
    #[validate(custom = "validate_qs_sort_option")]
    pub sort: Option<String>,

    /// maximum number of items to return (integer)
    pub limit: Option<u32>,

    /// position at which to restart search (string)
    pub offset: Option<String>,

    /// a comma-separated list of BSO ids (list of strings)
    #[validate(custom = "validate_qs_ids")]
    #[serde(deserialize_with = "deserialize_comma_sep_string",)]
    pub ids: Option<Vec<String>>,

    // flag, whether to include full bodies (bool)
    #[serde(deserialize_with = "deserialize_present_value",)]
    pub full: bool,
}

impl FromRequest<ServerState> for BsoQueryParams {
    type Config = Settings;
    type Result = Result<BsoQueryParams, Error>;

    /// Extract and validate the query parameters
    fn from_request(req: &HttpRequest<ServerState>, _: &Self::Config) -> Self::Result {
        // TODO: serde deserialize the query ourselves to catch the serde error nicely
        let params = Query::<BsoQueryParams>::from_request(req, &())?.into_inner();
        params.validate().map_err(RequestValidationErrors)?;
        Ok(params)
    }
}

/// PreCondition Header
///
/// It's valid to include a X-If-Modified-Since or X-If-Unmodified-Since header but not
/// both.
///
/// Used with Option<PreConditionHeader> to extract a possible PreConditionHeader.
#[derive(Debug, PartialEq)]
pub enum PreConditionHeader {
    IfModifiedSince(f64),
    IfUnmodifiedSince(f64),
}

impl FromRequest<ServerState> for Option<PreConditionHeader> {
    type Config = Settings;
    type Result = Result<Option<PreConditionHeader>, Error>;

    /// Extract and validate the precondition headers
    fn from_request(req: &HttpRequest<ServerState>, _: &Self::Config) -> Self::Result {
        let mut errors = ValidationErrors::new();
        let headers = req.headers();
        let modified = headers.get("X-If-Modified-Since");
        let unmodified = headers.get("X-If-Unmodified-Since");
        if modified.is_some() && unmodified.is_some() {
            errors.add(
                "X-If-Unmodified-Since",
                request_error(
                    "Cannot specify both X-If-Modified-Since and X-If-Unmodified-Since on \
                     a single request",
                    RequestErrorLocation::Header,
                ),
            );
            return Err(RequestValidationErrors(errors).into());
        };
        let (value, field_name) = if let Some(modified_value) = modified {
            (modified_value, "X-If-Modified-Since")
        } else if let Some(unmodified_value) = unmodified {
            (unmodified_value, "X-If-Unmodified-Since")
        } else {
            return Ok(None);
        };
        value
            .to_str()
            .map_err(|_| request_error("Bad value", RequestErrorLocation::Header))
            .and_then(|v| {
                v.parse::<f64>()
                    .map_err(|_| request_error("Bad value", RequestErrorLocation::Header))
                    .and_then(|v| {
                        // Don't allow negative values for the field
                        if v < 0.0 {
                            Err(request_error("Bad value", RequestErrorLocation::Header))
                        } else {
                            Ok(v)
                        }
                    })
            }).map(|v| {
                if field_name == "X-If-Modified-Since" {
                    Some(PreConditionHeader::IfModifiedSince(v))
                } else {
                    Some(PreConditionHeader::IfUnmodifiedSince(v))
                }
            }).map_err(|e: ValidationError| {
                errors.add(field_name, e);
                RequestValidationErrors(errors).into()
            })
    }
}

/// Client Request Error
///
/// Represents errors returned when a client makes an error in its request.
#[derive(Debug, Default, Deserialize, Serialize)]
struct ClientRequestError {
    status: u32,
    errors: Option<Vec<RequestValidationError>>,
}

/// Request Validation errors
///
/// Represents a single error from validating the request.
#[derive(Debug, Deserialize, Serialize)]
struct RequestValidationError {
    location: RequestErrorLocation,
    name: String,
    description: String,
}

impl RequestValidationError {
    fn with_name_and_error(name: &str, error: &ValidationError) -> RequestValidationError {
        let location = if let Some(loc) = error.params.get("location") {
            // In the event a bad location is given
            from_value(loc.clone()).unwrap_or(RequestErrorLocation::Unknown)
        } else {
            // This ideally never happens, but just in case the location wasn't supplied
            RequestErrorLocation::Unknown
        };
        RequestValidationError {
            location,
            name: name.to_string(),
            description: format!("{}", error),
        }
    }
}

/// Validation Error Location in the request
#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum RequestErrorLocation {
    Body,
    QueryString,
    Url,
    Header,
    Path,
    Cookies,
    Method,
    Unknown,
}

/// Request Validation Errors
///
/// This is a wrapper on validator's ValidationErrors so that the error can
/// be properly adapted to a ResponseError.
#[derive(Debug)]
pub struct RequestValidationErrors(ValidationErrors);

impl error::Error for RequestValidationErrors {
    fn description(&self) -> &str {
        self.0.description()
    }

    fn cause(&self) -> Option<&error::Error> {
        self.0.cause()
    }
}

impl fmt::Display for RequestValidationErrors {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl ResponseError for RequestValidationErrors {
    fn error_response(&self) -> HttpResponse {
        let error: ClientRequestError = self.into();
        HttpResponse::build(StatusCode::BAD_REQUEST).json(error)
    }
}

impl<'a> From<&'a RequestValidationErrors> for ClientRequestError {
    fn from(errors: &RequestValidationErrors) -> Self {
        let mut sync_error = ClientRequestError::default();
        sync_error.status = 400;
        let mut client_errors = Vec::new();
        let validation_errors = errors.0.clone().field_errors();
        for (ref field_name, error_kind) in validation_errors.iter() {
            for error in error_kind {
                client_errors.push(RequestValidationError::with_name_and_error(
                    field_name, error,
                ))
            }
        }
        if !client_errors.is_empty() {
            sync_error.errors = Some(client_errors)
        }
        sync_error
    }
}

/// Convenience function to create a `ValidationError` with additional context
fn request_error(message: &'static str, location: RequestErrorLocation) -> ValidationError {
    let mut err = ValidationError::new(message);
    err.add_param("location".into(), &location);
    err
}

/// Verifies that the supplied value is >= 0
fn validate_qs_positive_value<T: PartialOrd + Zero>(ts: T) -> Result<(), ValidationError> {
    if ts < Zero::zero() {
        Err(request_error(
            "Invalid value",
            RequestErrorLocation::QueryString,
        ))
    } else {
        Ok(())
    }
}

/// Verifies that the supplied sort is supported
fn validate_qs_sort_option(sort: &str) -> Result<(), ValidationError> {
    if sort == "newest" || sort == "oldest" || sort == "index" {
        Ok(())
    } else {
        Err(request_error(
            "Invalid sort option",
            RequestErrorLocation::QueryString,
        ))
    }
}

/// Verifies that the list of id's is not too long and that the ids are valid
fn validate_qs_ids(ids: &Vec<String>) -> Result<(), ValidationError> {
    if ids.len() > BATCH_MAX_IDS {
        return Err(request_error(
            "Too many ids provided",
            RequestErrorLocation::QueryString,
        ));
    }
    for ref id in ids {
        if !VALID_ID_REGEX.is_match(id) {
            return Err(request_error(
                "Invalid id in ids",
                RequestErrorLocation::QueryString,
            ));
        }
    }
    Ok(())
}

/// Deserialize a comma separated string
fn deserialize_comma_sep_string<'de, D, E>(deserializer: D) -> Result<Option<Vec<E>>, D::Error>
where
    D: Deserializer<'de>,
    E: FromStr,
{
    let maybe_str: Option<String> = Deserialize::deserialize(deserializer)?;
    let maybe_lst: Option<Vec<String>> = maybe_str.map(|s| {
        s.split(",")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    });
    if let Some(lst) = maybe_lst {
        let mut parsed_lst: Vec<E> = Vec::new();
        for item in lst {
            parsed_lst.push(
                item.parse::<E>()
                    .map_err(|_| SerdeError::custom("Invalid value in list"))?,
            );
        }
        Ok(Some(parsed_lst))
    } else {
        Ok(None)
    }
}

/// Deserialize a value as True if it exists, False otherwise
fn deserialize_present_value<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let maybe_str: Option<String> = Option::deserialize(deserializer).unwrap_or(None);
    Ok(maybe_str.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::str::from_utf8;
    use std::sync::Arc;

    use actix_web::test::TestRequest;
    use actix_web::{Binary, Body};
    use serde_json;

    use db::mock::MockDb;
    use server::ServerState;
    use settings::{Secrets, ServerLimits};

    lazy_static! {
        static ref SERVER_LIMITS: Arc<ServerLimits> = Arc::new(ServerLimits::default());
        static ref SECRETS: Arc<Secrets> = Arc::new(Secrets::new("foo"));
    }

    fn make_state() -> ServerState {
        ServerState {
            db: Box::new(MockDb::new()),
            limits: Arc::clone(&SERVER_LIMITS),
            secrets: Arc::clone(&SECRETS),
        }
    }

    fn extract_body_as_str(response: &HttpResponse) -> String {
        match response.body() {
            Body::Binary(binary) => match binary {
                Binary::Bytes(b) => from_utf8(b).unwrap().to_string(),
                Binary::Slice(s) => from_utf8(s).unwrap().to_string(),
                Binary::SharedString(s) => s.clone().to_string(),
                Binary::SharedVec(v) => from_utf8(v).unwrap().to_string(),
            },
            _ => panic!("Invalid body"),
        }
    }

    #[test]
    fn test_invalid_query_args() {
        let req = TestRequest::with_state(make_state())
            .uri("/?lower=-1.23&sort=whatever")
            .finish();
        let result = BsoQueryParams::extract(&req);
        assert!(result.is_err());
        let response: HttpResponse = result.err().unwrap().into();
        assert_eq!(response.status(), 400);
        let body = extract_body_as_str(&response);
        let err: ClientRequestError = serde_json::from_str(&body).unwrap();
        assert_eq!(err.status, 400);
        let errors = err.errors.unwrap();
        assert_eq!(errors[0].location, RequestErrorLocation::QueryString);
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn test_valid_query_args() {
        let req = TestRequest::with_state(make_state())
            .uri("/?ids=1,2,&full=&sort=index&older=2.43")
            .finish();
        let result = BsoQueryParams::extract(&req).unwrap();
        assert_eq!(result.ids.unwrap(), vec!["1", "2"]);
        assert_eq!(result.sort.unwrap(), "index");
        assert_eq!(result.older.unwrap(), 2.43);
        assert_eq!(result.full, true);
    }

    #[test]
    fn test_invalid_precondition_headers() {
        fn assert_invalid_header(req: &HttpRequest<ServerState>) {
            let result = <Option<PreConditionHeader> as FromRequest<ServerState>>::extract(&req);
            assert!(result.is_err());
            let response: HttpResponse = result.err().unwrap().into();
            assert_eq!(response.status(), 400);
            let body = extract_body_as_str(&response);
            let err: ClientRequestError = serde_json::from_str(&body).unwrap();
            assert_eq!(err.status, 400);
            let errors = err.errors.unwrap();
            assert_eq!(errors[0].location, RequestErrorLocation::Header);
            assert_eq!(errors.len(), 1);
        }
        let req = TestRequest::with_state(make_state())
            .header("X-If-Modified-Since", "32124.32")
            .header("X-If-Unmodified-Since", "4212.12")
            .uri("/")
            .finish();
        assert_invalid_header(&req);
        let req = TestRequest::with_state(make_state())
            .header("X-If-Modified-Since", "-32.1")
            .uri("/")
            .finish();
        assert_invalid_header(&req);
    }

    #[test]
    fn test_valid_precondition_headers() {
        let req = TestRequest::with_state(make_state())
            .header("X-If-Modified-Since", "32.1")
            .uri("/")
            .finish();
        let result = <Option<PreConditionHeader> as FromRequest<ServerState>>::extract(&req)
            .unwrap()
            .unwrap();
        assert_eq!(result, PreConditionHeader::IfModifiedSince(32.1));
        let req = TestRequest::with_state(make_state())
            .header("X-If-Unmodified-Since", "32.14")
            .uri("/")
            .finish();
        let result = <Option<PreConditionHeader> as FromRequest<ServerState>>::extract(&req)
            .unwrap()
            .unwrap();
        assert_eq!(result, PreConditionHeader::IfUnmodifiedSince(32.14));
    }
}
