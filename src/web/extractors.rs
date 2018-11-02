//! Request header/body/query extractors
//!
//! Handles ensuring the header's, body, and query parameters are correct, extraction to
//! relevant types, and failing correctly with the appropriate errors if issues arise.
use std::str::FromStr;

use actix_web::http::header::{HeaderValue, CONTENT_TYPE};
use actix_web::{
    dev::{JsonConfig, PayloadConfig},
    error::ErrorInternalServerError,
    Error, FromRequest, HttpRequest, Json, Path, Query,
};
use futures::{future, Future};
use num::Zero;
use regex::Regex;
use serde::de::{Deserialize, Deserializer, Error as SerdeError};
use serde_json;
use validator::{Validate, ValidationError};

use db::Db;
use error::{ApiError, ApiResult};
use server::ServerState;
use web::{auth::HawkPayload, error::ValidationErrorKind};

const BATCH_MAX_IDS: usize = 100;

// BSO const restrictions
const BSO_MAX_PAYLOAD_SIZE: usize = 2 * 1024 * 1024;
const BSO_MAX_TTL: u32 = 31536000;
const BSO_MAX_SORTINDEX_VALUE: i32 = 999999999;
const BSO_MIN_SORTINDEX_VALUE: i32 = -999999999;

// TODO: These should come from config using actix with_config
const BATCH_MAX_RECORDS: usize = 100;
const BATCH_MAX_BYTES: usize = BSO_MAX_PAYLOAD_SIZE * 150;

lazy_static! {
    static ref KNOWN_BAD_PAYLOAD_REGEX: Regex =
        Regex::new(r#"IV":\s*"AAAAAAAAAAAAAAAAAAAAAA=="#).unwrap();
    static ref VALID_ID_REGEX: Regex = Regex::new(r"^[ -~]{1,64}$").unwrap();
    static ref VALID_COLLECTION_ID_REGEX: Regex = Regex::new(r"^[a-zA-Z0-9._-]{1,32}$").unwrap();
}

#[derive(Deserialize)]
pub struct UidParam {
    #[allow(dead_code)] // Not really dead, but Rust can't see the deserialized use.
    uid: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BatchBsoBody {
    pub id: String,
    pub sortindex: Option<i32>,
    pub payload: Option<String>,
    pub ttl: Option<u32>,
}

#[derive(Default, Deserialize)]
pub struct BsoBodies {
    pub valid: Vec<BatchBsoBody>,
    pub invalid: Vec<BsoBody>,
}

impl FromRequest<ServerState> for BsoBodies {
    type Config = ();
    type Result = Box<Future<Item = BsoBodies, Error = Error>>;

    /// Extract the BSO Bodies from the request
    ///
    /// This extraction ensures the following conditions:
    ///   - Total payload size does not exceed `BATCH_MAX_BYTES`
    ///   - All BSO's deserialize from the request correctly
    ///   - Request content-type is a valid value
    ///   - Valid BSO's include a BSO id
    ///
    /// No collection id is used, so payload checks are not done here.
    fn from_request(req: &HttpRequest<ServerState>, _: &Self::Config) -> Self::Result {
        // Only try and parse the body if its a valid content-type
        let headers = req.headers();
        let default = HeaderValue::from_static("");
        let content_type = headers.get(CONTENT_TYPE).unwrap_or(&default).as_bytes();

        match content_type {
            b"application/json" | b"text/plain" | b"application/newlines" | b"" => (),
            _ => {
                return Box::new(future::err(
                    ValidationErrorKind::FromDetails(
                        "Invalid content-type".to_owned(),
                        RequestErrorLocation::Header,
                        Some("content-type".to_owned()),
                    ).into(),
                ));
            }
        }

        // Load the entire request into a String
        let mut config = PayloadConfig::default();
        config.limit(BATCH_MAX_BYTES);
        let fut = if let Ok(result) = <String>::from_request(req, &config) {
            result
        } else {
            return Box::new(future::err(
                ValidationErrorKind::FromDetails(
                    "Mimetype/encoding/content-length error".to_owned(),
                    RequestErrorLocation::Header,
                    None,
                ).into(),
            ));
        };

        // Avoid duplicating by defining our error func now, doesn't need the box wrapper
        fn make_error() -> Error {
            ValidationErrorKind::FromDetails(
                "Invalid JSON in request body".to_owned(),
                RequestErrorLocation::Body,
                Some("bsos".to_owned()),
            ).into()
        }

        // Define a new bool to check from a static closure to release the reference on the
        // content_type header
        let newlines: bool = content_type == b"application/newlines";

        let fut = fut.and_then(move |body| {
            // Parse out the body per the content type
            let bsos: Vec<BsoBody> = if newlines {
                let mut bsos = Vec::new();
                for item in body.lines() {
                    // Skip any blanks
                    if item == "" {
                        continue;
                    }
                    if let Ok(item) = serde_json::from_str(&item) {
                        bsos.push(item);
                    } else {
                        // Per Python version, all BSO's must parse or we error out
                        return future::err(make_error());
                    }
                }
                bsos
            } else {
                if let Ok(bsos) = serde_json::from_str(&body) {
                    bsos
                } else {
                    return future::err(make_error());
                }
            };

            // Validate all the BSO's, move invalid to our other list. Assume they'll all make
            // it with our pre-allocation
            let mut valid: Vec<BatchBsoBody> = Vec::with_capacity(bsos.len());
            let mut invalid: Vec<BsoBody> = Vec::new();
            for mut bso in bsos {
                if bso.validate().is_ok() {
                    // They're only valid if they include an id.
                    // XXX: Verify the id is optional, this may be a legacy thing
                    match bso {
                        BsoBody {
                            id: Some(id),
                            sortindex,
                            payload,
                            ttl,
                        } => valid.push(BatchBsoBody {
                            id,
                            sortindex,
                            payload,
                            ttl,
                        }),
                        bso => invalid.push(bso),
                    }
                } else {
                    invalid.push(bso);
                }
            }
            future::ok(BsoBodies { valid, invalid })
        });

        Box::new(fut)
    }
}

#[derive(Default, Deserialize, Serialize, Validate)]
pub struct BsoBody {
    #[validate(custom = "validate_body_bso_id")]
    pub id: Option<String>,
    #[validate(custom = "validate_body_bso_sortindex")]
    pub sortindex: Option<i32>,
    #[validate(custom = "validate_body_bso_payload")]
    pub payload: Option<String>,
    #[validate(custom = "validate_body_bso_ttl")]
    pub ttl: Option<u32>,
}

impl FromRequest<ServerState> for BsoBody {
    type Config = ();
    type Result = Box<Future<Item = BsoBody, Error = Error>>;

    fn from_request(req: &HttpRequest<ServerState>, _: &Self::Config) -> Self::Result {
        // Only try and parse the body if its a valid content-type
        let headers = req.headers();
        let default = HeaderValue::from_static("");
        match headers.get(CONTENT_TYPE).unwrap_or(&default).as_bytes() {
            b"application/json" | b"text/plain" | b"" => (),
            _ => {
                // TODO: This is supposed to return a 415 status for unknown content-type
                return Box::new(future::err(
                    ValidationErrorKind::FromDetails(
                        "Invalid content-type".to_owned(),
                        RequestErrorLocation::Header,
                        Some("content-type".to_owned()),
                    ).into(),
                ));
            }
        }
        let mut config = JsonConfig::default();
        config.limit(BSO_MAX_PAYLOAD_SIZE);
        let fut = <Json<BsoBody>>::from_request(req, &config)
            .map_err(|e| {
                let err: ApiError = ValidationErrorKind::FromDetails(
                    e.to_string(),
                    RequestErrorLocation::Body,
                    Some("bso".to_owned()),
                ).into();
                err.into()
            }).and_then(|bso: Json<BsoBody>| {
                if let Err(e) = bso.validate() {
                    let err: ApiError =
                        ValidationErrorKind::FromValidationErrors(e, RequestErrorLocation::Body)
                            .into();
                    return future::err(err.into());
                }
                future::ok(bso.into_inner())
            });

        Box::new(fut)
    }
}

/// Bso id parameter extractor
#[derive(Deserialize, Validate)]
pub struct BsoParam {
    #[validate(regex = "VALID_ID_REGEX")]
    pub bso: String,
}

impl FromRequest<ServerState> for BsoParam {
    type Config = ();
    type Result = ApiResult<BsoParam>;

    fn from_request(req: &HttpRequest<ServerState>, _: &Self::Config) -> Self::Result {
        let bso = Path::<BsoParam>::extract(req)
            .map_err(|e| {
                ValidationErrorKind::FromDetails(
                    e.to_string(),
                    RequestErrorLocation::Path,
                    Some("bso".to_owned()),
                )
            })?.into_inner();
        bso.validate().map_err(|e| {
            ValidationErrorKind::FromValidationErrors(e, RequestErrorLocation::Path)
        })?;
        Ok(bso)
    }
}

/// Collection parameter extractor
#[derive(Clone, Deserialize, Validate)]
pub struct CollectionParam {
    #[validate(regex = "VALID_COLLECTION_ID_REGEX")]
    pub collection: String,
}

impl FromRequest<ServerState> for CollectionParam {
    type Config = ();
    type Result = ApiResult<CollectionParam>;

    fn from_request(req: &HttpRequest<ServerState>, _: &Self::Config) -> Self::Result {
        if let Some(collection) = req.extensions().get::<CollectionParam>() {
            return Ok(collection.clone());
        }

        let collection = Path::<CollectionParam>::extract(req)
            .map_err(|e| {
                ValidationErrorKind::FromDetails(
                    e.to_string(),
                    RequestErrorLocation::Path,
                    Some("collection".to_owned()),
                )
            })?.into_inner();
        collection.validate().map_err(|e| {
            ValidationErrorKind::FromValidationErrors(e, RequestErrorLocation::Path)
        })?;
        req.extensions_mut().insert(collection.clone());
        Ok(collection)
    }
}

/// Information Requests extractor
///
/// Only the database and user identifier is required for information
/// requests: https://mozilla-services.readthedocs.io/en/latest/storage/apis-1.5.html#general-info
pub struct MetaRequest {
    pub user_id: HawkIdentifier,
    pub db: Box<dyn Db>,
}

impl FromRequest<ServerState> for MetaRequest {
    type Config = ();
    type Result = Result<MetaRequest, Error>;

    fn from_request(req: &HttpRequest<ServerState>, settings: &Self::Config) -> Self::Result {
        let user_id = HawkIdentifier::from_request(req, settings)?;
        let db = <Box<dyn Db>>::from_request(req, settings)?;
        Ok({ MetaRequest { user_id, db } })
    }
}

/// Collection Request Delete/Get extractor
///
/// Extracts/validates information needed for collection delete/get requests.
pub struct CollectionRequest {
    pub collection: String,
    pub db: Box<dyn Db>,
    pub user_id: HawkIdentifier,
    pub query: BsoQueryParams,
}

impl FromRequest<ServerState> for CollectionRequest {
    type Config = ();
    type Result = Result<CollectionRequest, Error>;

    fn from_request(req: &HttpRequest<ServerState>, settings: &Self::Config) -> Self::Result {
        let user_id = HawkIdentifier::from_request(req, settings)?;
        let db = <Box<dyn Db>>::from_request(req, settings)?;
        let query = BsoQueryParams::from_request(req, settings)?;
        let collection = CollectionParam::from_request(req, settings)?.collection;

        Ok(CollectionRequest {
            collection,
            db,
            user_id,
            query,
        })
    }
}

/// Collection Request Post extractor
///
/// Extracts/validates information needed for batch collection POST requests.
pub struct CollectionPostRequest {
    pub collection: String,
    pub db: Box<dyn Db>,
    pub user_id: HawkIdentifier,
    pub query: BsoQueryParams,
    pub bsos: BsoBodies,
}

impl FromRequest<ServerState> for CollectionPostRequest {
    type Config = ();
    type Result = Box<Future<Item = CollectionPostRequest, Error = Error>>;

    /// Extractor for Collection Posts (Batch BSO upload)
    ///
    /// Utilizes the `BsoBodies` for parsing, and add's two validation steps not
    /// done previously:
    ///   - If the collection is 'crypto', known bad payloads are checked for
    ///   - Any valid BSO's beyond `BATCH_MAX_RECORDS` are moved to invalid
    fn from_request(req: &HttpRequest<ServerState>, _: &Self::Config) -> Self::Result {
        let fut = <(
            HawkIdentifier,
            Box<dyn Db>,
            CollectionParam,
            BsoQueryParams,
            BsoBodies,
        )>::extract(req).and_then(|(user_id, db, collection, query, mut bsos)| {
            let collection = collection.collection.clone();
            if collection == "crypto" {
                // Verify the client didn't mess up the crypto if we have a payload
                for bso in &bsos.valid {
                    if let Some(ref data) = bso.payload {
                        if KNOWN_BAD_PAYLOAD_REGEX.is_match(data) {
                            return future::err(
                                ValidationErrorKind::FromDetails(
                                    "Known-bad BSO payload".to_owned(),
                                    RequestErrorLocation::Body,
                                    Some("bsos".to_owned()),
                                ).into(),
                            );
                        }
                    }
                }
            }

            // Trim the excess BSO's to be under the batch size
            let overage: i64 = (bsos.valid.len() as i64) - (BATCH_MAX_RECORDS as i64);
            if overage > 0 {
                for _ in 1..=overage {
                    if let Some(last) = bsos.valid.pop() {
                        bsos.invalid.push(BsoBody {
                            id: Some(last.id),
                            sortindex: last.sortindex,
                            payload: last.payload,
                            ttl: last.ttl,
                        });
                    }
                }
            }

            future::ok(CollectionPostRequest {
                collection,
                db,
                user_id,
                query,
                bsos,
            })
        });

        Box::new(fut)
    }
}

/// BSO Request Delete/Get extractor
///
/// Extracts/validates information needed for BSO delete/get requests.
pub struct BsoRequest {
    pub collection: String,
    pub db: Box<dyn Db>,
    pub user_id: HawkIdentifier,
    pub query: BsoQueryParams,
    pub bso: String,
}

impl FromRequest<ServerState> for BsoRequest {
    type Config = ();
    type Result = Result<BsoRequest, Error>;

    fn from_request(req: &HttpRequest<ServerState>, settings: &Self::Config) -> Self::Result {
        let user_id = HawkIdentifier::from_request(req, settings)?;
        let db = <Box<dyn Db>>::from_request(req, settings)?;
        let query = BsoQueryParams::from_request(req, settings)?;
        let collection = CollectionParam::from_request(req, settings)?
            .collection
            .clone();
        let bso = BsoParam::from_request(req, settings)?;

        Ok(BsoRequest {
            collection,
            db,
            user_id,
            query,
            bso: bso.bso.clone(),
        })
    }
}

/// BSO Request Put extractor
///
/// Extracts/validates information needed for BSO put requests.
pub struct BsoPutRequest {
    pub collection: String,
    pub db: Box<dyn Db>,
    pub user_id: HawkIdentifier,
    pub query: BsoQueryParams,
    pub bso: String,
    pub body: BsoBody,
}

impl FromRequest<ServerState> for BsoPutRequest {
    type Config = ();
    type Result = Box<Future<Item = BsoPutRequest, Error = Error>>;

    fn from_request(req: &HttpRequest<ServerState>, _: &Self::Config) -> Self::Result {
        let fut = <(
            HawkIdentifier,
            Box<dyn Db>,
            CollectionParam,
            BsoQueryParams,
            BsoParam,
            BsoBody,
        )>::extract(req).and_then(|(user_id, db, collection, query, bso, body)| {
            let collection = collection.collection.clone();
            if collection == "crypto" {
                // Verify the client didn't mess up the crypto if we have a payload
                if let Some(ref data) = body.payload {
                    if KNOWN_BAD_PAYLOAD_REGEX.is_match(data) {
                        return future::err(
                            ValidationErrorKind::FromDetails(
                                "Known-bad BSO payload".to_owned(),
                                RequestErrorLocation::Body,
                                Some("bsos".to_owned()),
                            ).into(),
                        );
                    }
                }
            }

            future::ok(BsoPutRequest {
                collection,
                db,
                user_id,
                query,
                bso: bso.bso.clone(),
                body,
            })
        });

        Box::new(fut)
    }
}

/// Extract a user-identifier from the authentication token and validate against the URL
///
/// This token should be adapted as needed for the storage system to store data
/// for the user.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct HawkIdentifier {
    /// For MySQL database backends as the primary key
    pub legacy_id: u64,
    /// For NoSQL database backends that require randomly distributed primary keys
    pub fxa_id: String,
}

impl FromRequest<ServerState> for HawkIdentifier {
    type Config = ();
    type Result = ApiResult<HawkIdentifier>;

    /// Use HawkPayload extraction and format as HawkIdentifier.
    fn from_request(req: &HttpRequest<ServerState>, settings: &Self::Config) -> Self::Result {
        if let Some(user_id) = req.extensions().get::<HawkIdentifier>() {
            return Ok(user_id.clone());
        }

        let payload = HawkPayload::from_request(req, settings)?;
        let path_uid = Path::<UidParam>::extract(req).map_err(|e| {
            ValidationErrorKind::FromDetails(
                e.to_string(),
                RequestErrorLocation::Path,
                Some("uid".to_owned()),
            )
        })?;
        if payload.user_id != path_uid.uid {
            Err(ValidationErrorKind::FromDetails(
                "conflicts with payload".to_owned(),
                RequestErrorLocation::Path,
                Some("uid".to_owned()),
            ))?;
        }

        let user_id = HawkIdentifier {
            legacy_id: payload.user_id,
            fxa_id: "".to_string(),
        };
        req.extensions_mut().insert(user_id.clone());
        Ok(user_id)
    }
}

impl HawkIdentifier {
    /// Create a new legacy id user identifier
    pub fn new_legacy(user_id: u64) -> HawkIdentifier {
        HawkIdentifier {
            legacy_id: user_id,
            ..Default::default()
        }
    }
}

impl FromRequest<ServerState> for Box<dyn Db> {
    type Config = ();
    type Result = Result<Self, Error>;

    fn from_request(req: &HttpRequest<ServerState>, _: &Self::Config) -> Self::Result {
        req.extensions()
            .get::<Box<dyn Db>>()
            .ok_or_else(|| ErrorInternalServerError("Unexpected Db error"))
            .map(Clone::clone)
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
    type Config = ();
    type Result = ApiResult<BsoQueryParams>;

    /// Extract and validate the query parameters
    fn from_request(req: &HttpRequest<ServerState>, _: &Self::Config) -> Self::Result {
        // TODO: serde deserialize the query ourselves to catch the serde error nicely
        let params = Query::<BsoQueryParams>::from_request(req, &())
            .map_err(|e| {
                ValidationErrorKind::FromDetails(
                    e.to_string(),
                    RequestErrorLocation::QueryString,
                    None,
                )
            })?.into_inner();
        params.validate().map_err(|e| {
            ValidationErrorKind::FromValidationErrors(e, RequestErrorLocation::QueryString)
        })?;
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
    type Config = ();
    type Result = ApiResult<Option<PreConditionHeader>>;

    /// Extract and validate the precondition headers
    fn from_request(req: &HttpRequest<ServerState>, _: &Self::Config) -> Self::Result {
        let headers = req.headers();
        let modified = headers.get("X-If-Modified-Since");
        let unmodified = headers.get("X-If-Unmodified-Since");
        if modified.is_some() && unmodified.is_some() {
            Err(ValidationErrorKind::FromDetails(
                "conflicts with X-If-Modified-Since".to_owned(),
                RequestErrorLocation::Header,
                Some("X-If-Unmodified-Since".to_owned()),
            ))?;
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
            .map_err(|e| {
                ValidationErrorKind::FromDetails(
                    e.to_string(),
                    RequestErrorLocation::Header,
                    Some(field_name.to_owned()),
                ).into()
            }).and_then(|v| {
                v.parse::<f64>()
                    .map_err(|e| {
                        ValidationErrorKind::FromDetails(
                            e.to_string(),
                            RequestErrorLocation::Header,
                            Some(field_name.to_owned()),
                        ).into()
                    }).and_then(|v| {
                        // Don't allow negative values for the field
                        if v < 0.0 {
                            Err(ValidationErrorKind::FromDetails(
                                "value is negative".to_string(),
                                RequestErrorLocation::Header,
                                Some(field_name.to_owned()),
                            ))?
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
            })
    }
}

/// Validation Error Location in the request
#[derive(Debug, Deserialize, Serialize, PartialEq)]
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

/// Verifies the BSO sortindex is in the valid range
fn validate_body_bso_sortindex(sort: i32) -> Result<(), ValidationError> {
    if BSO_MIN_SORTINDEX_VALUE <= sort && sort <= BSO_MAX_SORTINDEX_VALUE {
        Ok(())
    } else {
        Err(request_error("invalid value", RequestErrorLocation::Body))
    }
}

/// Verifies the BSO payload size is valid
fn validate_body_bso_payload(payload: &str) -> Result<(), ValidationError> {
    if payload.len() > BSO_MAX_PAYLOAD_SIZE {
        return Err(request_error("invalid size", RequestErrorLocation::Body));
    }
    Ok(())
}

/// Verifies the BSO id string is valid
fn validate_body_bso_id(id: &String) -> Result<(), ValidationError> {
    if !VALID_ID_REGEX.is_match(id) {
        return Err(request_error("Invalid id", RequestErrorLocation::Body));
    }
    Ok(())
}

/// Verifies the BSO ttl is valid
fn validate_body_bso_ttl(ttl: u32) -> Result<(), ValidationError> {
    if ttl > BSO_MAX_TTL {
        return Err(request_error("Invalid TTL", RequestErrorLocation::Body));
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
    use actix_web::HttpResponse;
    use actix_web::{http::Method, Binary, Body};
    use base64;
    use hawk::{Credentials, Key, RequestBuilder};
    use hmac::{Hmac, Mac};
    use ring;
    use serde_json;
    use sha2::Sha256;

    use db::mock::{MockDb, MockDbPool};
    use server::ServerState;
    use settings::{Secrets, ServerLimits};

    use web::auth::{hkdf_expand_32, HawkPayload};

    lazy_static! {
        static ref SERVER_LIMITS: Arc<ServerLimits> = Arc::new(ServerLimits::default());
        static ref SECRETS: Arc<Secrets> = Arc::new(Secrets::new("Ted Koppel is a robot"));
    }

    // String is too long for valid name
    const INVALID_COLLECTION_NAME: &'static str =
        "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz";
    const INVALID_BSO_NAME: &'static str =
        "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz";

    fn make_db() -> Box<dyn Db> {
        Box::new(MockDb::new())
    }

    fn make_state() -> ServerState {
        ServerState {
            db_pool: Box::new(MockDbPool::new()),
            limits: Arc::clone(&SERVER_LIMITS),
            secrets: Arc::clone(&SECRETS),
            port: 8000,
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

    fn create_valid_hawk_header(
        payload: &HawkPayload,
        state: &ServerState,
        method: &str,
        path: &str,
        host: &str,
        port: u16,
    ) -> String {
        let salt = payload.salt.clone();
        let payload = serde_json::to_string(payload).unwrap();
        let mut hmac: Hmac<Sha256> = Hmac::new_varkey(&state.secrets.signing_secret).unwrap();
        hmac.input(payload.as_bytes());
        let payload_hash = hmac.result().code();
        let mut id = payload.as_bytes().to_vec();
        id.extend(payload_hash.to_vec());
        let id = base64::encode_config(&id, base64::URL_SAFE);
        let token_secret = hkdf_expand_32(
            format!("services.mozilla.com/tokenlib/v1/derive/{}", id).as_bytes(),
            Some(salt.as_bytes()),
            &SECRETS.master_secret,
        );
        let token_secret = base64::encode_config(&token_secret, base64::URL_SAFE);
        let credentials = Credentials {
            id,
            key: Key::new(token_secret.as_bytes(), &ring::digest::SHA256),
        };
        let request = RequestBuilder::new(method, host, port, path)
            .hash(&payload_hash[..])
            .request();
        format!("Hawk {}", request.make_header(&credentials).unwrap())
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

        let err: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(err["status"], 400);
        assert_eq!(err["reason"], "Bad Request");

        let (lower_error, sort_error) = if err["errors"][0]["name"] == "lower" {
            (&err["errors"][0], &err["errors"][1])
        } else {
            (&err["errors"][1], &err["errors"][0])
        };

        assert_eq!(lower_error["description"], "Invalid value");
        assert_eq!(lower_error["location"], "querystring");
        assert_eq!(lower_error["name"], "lower");
        assert_eq!(lower_error["value"], -1.23);

        assert_eq!(sort_error["description"], "Invalid sort option");
        assert_eq!(sort_error["location"], "querystring");
        assert_eq!(sort_error["name"], "sort");
        assert_eq!(sort_error["value"], "whatever");
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
    fn test_valid_bso_request() {
        let payload = HawkPayload::test_default();
        let state = make_state();
        let header = create_valid_hawk_header(
            &payload,
            &state,
            "GET",
            "/storage/1.5/1/storage/tabs/asdf",
            "localhost",
            5000,
        );
        let req = TestRequest::with_state(state)
            .header("authorization", header)
            .method(Method::GET)
            .uri("http://localhost:5000/storage/1.5/1/storage/tabs/asdf")
            .param("uid", "1")
            .param("collection", "tabs")
            .param("bso", "asdf")
            .finish();
        req.extensions_mut().insert(make_db());
        let result = BsoRequest::extract(&req).unwrap();
        assert_eq!(result.user_id.legacy_id, 1);
        assert_eq!(&result.collection, "tabs");
        assert_eq!(&result.bso, "asdf");
    }

    #[test]
    fn test_invalid_bso_request() {
        let payload = HawkPayload::test_default();
        let state = make_state();
        let header = create_valid_hawk_header(
            &payload,
            &state,
            "GET",
            "/storage/1.5/1/storage/tabs",
            "localhost",
            5000,
        );
        let req = TestRequest::with_state(state)
            .header("authorization", header)
            .method(Method::GET)
            .uri("http://localhost:5000/storage/1.5/1/storage/tabs")
            .param("uid", "1")
            .param("collection", "tabs")
            .param("bso", INVALID_BSO_NAME)
            .finish();
        req.extensions_mut().insert(make_db());
        let result = BsoRequest::extract(&req);
        assert!(result.is_err());
        let response: HttpResponse = result.err().unwrap().into();
        assert_eq!(response.status(), 400);
        let body = extract_body_as_str(&response);

        let err: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(err["status"], 400);

        assert_eq!(err["errors"][0]["description"], "regex");
        assert_eq!(err["errors"][0]["location"], "path");
        assert_eq!(err["errors"][0]["name"], "bso");
        assert_eq!(err["errors"][0]["value"], INVALID_BSO_NAME);
    }

    #[test]
    fn test_valid_bso_post_body() {
        let payload = HawkPayload::test_default();
        let state = make_state();
        let header = create_valid_hawk_header(
            &payload,
            &state,
            "POST",
            "/storage/1.5/1/storage/tabs/asdf",
            "localhost",
            5000,
        );
        let bso_body = json!({
            "id": "128", "payload": "x"
        });
        let req = TestRequest::with_state(state)
            .header("authorization", header)
            .header("content-type", "application/json")
            .method(Method::POST)
            .uri("http://localhost:5000/storage/1.5/1/storage/tabs/asdf")
            .set_payload(bso_body.to_string())
            .param("uid", "1")
            .param("collection", "tabs")
            .param("bso", "asdf")
            .finish();
        req.extensions_mut().insert(make_db());
        let result = BsoPutRequest::extract(&req).wait().unwrap();
        assert_eq!(result.user_id.legacy_id, 1);
        assert_eq!(&result.collection, "tabs");
        assert_eq!(&result.bso, "asdf");
        assert_eq!(result.body.payload, Some("x".to_string()));
    }

    #[test]
    fn test_invalid_bso_post_body() {
        let payload = HawkPayload::test_default();
        let state = make_state();
        let header = create_valid_hawk_header(
            &payload,
            &state,
            "POST",
            "/storage/1.5/1/storage/tabs/asdf",
            "localhost",
            5000,
        );
        let bso_body = json!({
            "payload": "xxx", "sortindex": -9999999999,
        });
        let req = TestRequest::with_state(state)
            .header("authorization", header)
            .header("content-type", "application/json")
            .method(Method::POST)
            .uri("http://localhost:5000/storage/1.5/1/storage/tabs/asdf")
            .set_payload(bso_body.to_string())
            .param("uid", "1")
            .param("collection", "tabs")
            .param("bso", "asdf")
            .finish();
        req.extensions_mut().insert(make_db());
        let result = BsoPutRequest::extract(&req).wait();
        let response: HttpResponse = result.err().unwrap().into();
        assert_eq!(response.status(), 400);
        let body = extract_body_as_str(&response);

        let err: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(err["status"], 400);

        assert_eq!(err["errors"][0]["description"], "invalid value");
        assert_eq!(err["errors"][0]["location"], "body");
        assert_eq!(err["errors"][0]["name"], "sortindex");
        assert_eq!(err["errors"][0]["value"], -9999999999);
    }

    #[test]
    fn test_valid_collection_request() {
        let payload = HawkPayload::test_default();
        let state = make_state();
        let header = create_valid_hawk_header(
            &payload,
            &state,
            "GET",
            "/storage/1.5/1/storage/tabs",
            "localhost",
            5000,
        );
        let req = TestRequest::with_state(state)
            .header("authorization", header)
            .method(Method::GET)
            .uri("http://localhost:5000/storage/1.5/1/storage/tabs")
            .param("uid", "1")
            .param("collection", "tabs")
            .finish();
        req.extensions_mut().insert(make_db());
        let result = CollectionRequest::extract(&req).unwrap();
        assert_eq!(result.user_id.legacy_id, 1);
        assert_eq!(&result.collection, "tabs");
    }

    #[test]
    fn test_invalid_collection_request() {
        let payload = HawkPayload::test_default();
        let state = make_state();
        let header = create_valid_hawk_header(
            &payload,
            &state,
            "GET",
            "/storage/1.5/1/storage/tabs",
            "localhost",
            5000,
        );
        let req = TestRequest::with_state(state)
            .header("authorization", header)
            .method(Method::GET)
            .uri("http://localhost:5000/storage/1.5/1/storage/tabs")
            .param("uid", "1")
            .param("collection", INVALID_COLLECTION_NAME)
            .finish();
        req.extensions_mut().insert(make_db());
        let result = CollectionRequest::extract(&req);
        assert!(result.is_err());
        let response: HttpResponse = result.err().unwrap().into();
        assert_eq!(response.status(), 400);
        let body = extract_body_as_str(&response);

        let err: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(err["status"], 400);

        assert_eq!(err["errors"][0]["description"], "regex");
        assert_eq!(err["errors"][0]["location"], "path");
        assert_eq!(err["errors"][0]["name"], "collection");
        assert_eq!(err["errors"][0]["value"], INVALID_COLLECTION_NAME);
    }

    #[test]
    fn test_invalid_precondition_headers() {
        fn assert_invalid_header(
            req: &HttpRequest<ServerState>,
            error_header: &str,
            error_message: &str,
        ) {
            let result = <Option<PreConditionHeader> as FromRequest<ServerState>>::extract(&req);
            assert!(result.is_err());
            let response: HttpResponse = result.err().unwrap().into();
            assert_eq!(response.status(), 400);
            let body = extract_body_as_str(&response);

            let err: serde_json::Value = serde_json::from_str(&body).unwrap();
            assert_eq!(err["status"], 400);

            assert_eq!(err["errors"][0]["description"], error_message);
            assert_eq!(err["errors"][0]["location"], "header");
            assert_eq!(err["errors"][0]["name"], error_header);
        }
        let req = TestRequest::with_state(make_state())
            .header("X-If-Modified-Since", "32124.32")
            .header("X-If-Unmodified-Since", "4212.12")
            .uri("/")
            .finish();
        assert_invalid_header(
            &req,
            "X-If-Unmodified-Since",
            "conflicts with X-If-Modified-Since",
        );
        let req = TestRequest::with_state(make_state())
            .header("X-If-Modified-Since", "-32.1")
            .uri("/")
            .finish();
        assert_invalid_header(&req, "X-If-Modified-Since", "value is negative");
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

    #[test]
    fn valid_header_with_valid_path() {
        let payload = HawkPayload::test_default();
        let state = make_state();
        let header = create_valid_hawk_header(
            &payload,
            &state,
            "GET",
            "/storage/1.5/1/storage/col2",
            "localhost",
            5000,
        );
        let req = TestRequest::with_state(state)
            .header("authorization", header)
            .method(Method::GET)
            .uri("http://localhost:5000/storage/1.5/1/storage/col2")
            .param("uid", "1")
            .finish();
        let result = HawkIdentifier::extract(&req).unwrap();
        assert_eq!(result.legacy_id, 1);
    }

    #[test]
    fn valid_header_with_invalid_uid_in_path() {
        let payload = HawkPayload::test_default();
        let state = make_state();
        let header = create_valid_hawk_header(
            &payload,
            &state,
            "GET",
            "/storage/1.5/5/storage/col2",
            "localhost",
            5000,
        );
        let req = TestRequest::with_state(state)
            .header("authorization", header)
            .method(Method::GET)
            .uri("http://localhost:5000/storage/1.5/5/storage/col2")
            .param("uid", "5")
            .finish();
        let result = HawkIdentifier::extract(&req);
        assert!(result.is_err());
        let response: HttpResponse = result.err().unwrap().into();
        assert_eq!(response.status(), 400);
        let body = extract_body_as_str(&response);

        let err: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(err["status"], 400);

        assert_eq!(err["errors"][0]["description"], "conflicts with payload");
        assert_eq!(err["errors"][0]["location"], "path");
        assert_eq!(err["errors"][0]["name"], "uid");
    }
}
