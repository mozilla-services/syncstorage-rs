use actix_web::{dev::Payload, web::Query, Error, FromRequest, HttpRequest};
use futures::future::{LocalBoxFuture, TryFutureExt};
use serde::{
    de::{Deserializer, Error as SerdeError},
    Deserialize,
};
use std::str::FromStr;
use syncstorage_db::{Sorting, SyncTimestamp};
use validator::{Validate, ValidationError};

use crate::web::{
    error::ValidationErrorKind,
    extractors::{request_error, Offset, RequestErrorLocation, BATCH_MAX_IDS, VALID_ID_REGEX},
};

/// Verifies that the list of id's is not too long and that the ids are valid
pub fn validate_qs_ids(ids: &[String]) -> Result<(), ValidationError> {
    if ids.len() > BATCH_MAX_IDS {
        return Err(request_error(
            "Too many ids provided",
            RequestErrorLocation::QueryString,
        ));
    }
    for id in ids {
        if !VALID_ID_REGEX.is_match(id) {
            return Err(request_error(
                "Invalid id in ids",
                RequestErrorLocation::QueryString,
            ));
        }
    }
    Ok(())
}

/// Deserialize a header string value (epoch seconds with 2 decimal places) as SyncTimestamp
pub fn deserialize_sync_timestamp<'de, D>(
    deserializer: D,
) -> Result<Option<SyncTimestamp>, D::Error>
where
    D: Deserializer<'de>,
{
    let maybe_str: Option<String> = Deserialize::deserialize(deserializer)?;
    if let Some(val) = maybe_str {
        let result = SyncTimestamp::from_header(&val).map_err(SerdeError::custom);
        Ok(Some(result?))
    } else {
        Ok(None)
    }
}

pub fn deserialize_offset<'de, D>(deserializer: D) -> Result<Option<Offset>, D::Error>
where
    D: Deserializer<'de>,
{
    let maybe_str: Option<String> = Deserialize::deserialize(deserializer)?;
    if let Some(val) = maybe_str {
        return Ok(Some(Offset::from_str(&val).map_err(SerdeError::custom)?));
    }
    Ok(None)
}

/// Deserialize a comma separated string
pub fn deserialize_comma_sep_string<'de, D, E>(deserializer: D) -> Result<Vec<E>, D::Error>
where
    D: Deserializer<'de>,
    E: FromStr,
{
    let str: String = Deserialize::deserialize(deserializer)?;
    let lst: Vec<String> = str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let mut parsed_lst: Vec<E> = Vec::new();
    for item in lst {
        parsed_lst.push(
            item.parse::<E>()
                .map_err(|_| SerdeError::custom("Invalid value in list"))?,
        );
    }
    Ok(parsed_lst)
}

/// Deserialize a value as True if it exists, False otherwise
#[allow(clippy::unnecessary_wraps)] // serde::Deserialize requires Result<bool>
pub fn deserialize_present_value<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let maybe_str: Option<String> = Option::deserialize(deserializer).unwrap_or(None);
    Ok(maybe_str.is_some())
}

/// Validator to extract BSO search parameters from the query string.
///
/// This validator will extract and validate the following search params used in
/// multiple handler functions. Not all query params are used in each handler.
#[derive(Debug, Default, Clone, Deserialize, Validate)]
#[serde(default)]
pub struct BsoQueryParams {
    /// lower-bound on last-modified time
    #[serde(deserialize_with = "deserialize_sync_timestamp")]
    pub newer: Option<SyncTimestamp>,

    /// upper-bound on last-modified time
    #[serde(deserialize_with = "deserialize_sync_timestamp")]
    pub older: Option<SyncTimestamp>,

    /// order in which to return results (string)
    #[serde(default)]
    pub sort: Sorting,

    /// maximum number of items to return (integer)
    pub limit: Option<u32>,

    /// position at which to restart search (string)
    #[serde(deserialize_with = "deserialize_offset")]
    pub offset: Option<Offset>,

    /// a comma-separated list of BSO ids (list of strings)
    #[serde(deserialize_with = "deserialize_comma_sep_string", default)]
    #[validate(custom(function = "validate_qs_ids"))]
    pub ids: Vec<String>,

    // flag, whether to include full bodies (bool)
    #[serde(deserialize_with = "deserialize_present_value")]
    pub full: bool,
}

impl FromRequest for BsoQueryParams {
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    /// Extract and validate the query parameters
    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let req = req.clone();
        let mut payload = Payload::None;
        Box::pin(async move {
            let params = Query::<BsoQueryParams>::from_request(&req, &mut payload)
                .map_err(|e| {
                    ValidationErrorKind::FromDetails(
                        e.to_string(),
                        RequestErrorLocation::QueryString,
                        None,
                        None,
                    )
                })
                .await?
                .into_inner();
            params.validate().map_err(|e| {
                ValidationErrorKind::FromValidationErrors(
                    e,
                    RequestErrorLocation::QueryString,
                    None,
                )
            })?;
            // issue559: Dead code (timestamp always None)
            /*
            if params.sort != Sorting::Index {
                if let Some(timestamp) = params.offset.as_ref().and_then(|offset| offset.timestamp)
                {
                    let bound = timestamp.as_i64();
                    if let Some(newer) = params.newer {
                        if bound < newer.as_i64() {
                            return Err(ValidationErrorKind::FromDetails(
                                format!("Invalid Offset {} {}", bound, newer.as_i64()),
                                RequestErrorLocation::QueryString,
                                Some("newer".to_owned()),
                                None,
                            )
                            .into());
                        }
                    } else if let Some(older) = params.older {
                        if bound > older.as_i64() {
                            return Err(ValidationErrorKind::FromDetails(
                                "Invalid Offset".to_owned(),
                                RequestErrorLocation::QueryString,
                                Some("older".to_owned()),
                                None,
                            )
                            .into());
                        }
                    }
                }
            }
            */
            Ok(params)
        })
    }
}
