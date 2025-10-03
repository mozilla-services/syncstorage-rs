use std::{num::ParseIntError, str::FromStr};

use actix_web::{dev::Payload, web::Query, Error, FromRequest, HttpRequest};
use futures::future::{LocalBoxFuture, TryFutureExt};
use serde::{
    de::{Deserializer, Error as SerdeError},
    Deserialize,
};
use validator::{Validate, ValidationError};

use syncstorage_db::{params, Sorting, SyncTimestamp};

use super::{request_error, RequestErrorLocation, BATCH_MAX_IDS, VALID_ID_REGEX};
use crate::web::error::ValidationErrorKind;

#[derive(Debug, Default, Clone, Copy, Deserialize, Eq, PartialEq, Validate)]
#[serde(default)]
pub struct Offset {
    pub timestamp: Option<SyncTimestamp>,
    pub offset: u64,
}

impl From<Offset> for params::Offset {
    fn from(offset: Offset) -> Self {
        Self {
            timestamp: offset.timestamp,
            offset: offset.offset,
        }
    }
}

impl FromStr for Offset {
    type Err = ParseIntError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // issue559: Disable ':' support for now: simply parse as i64 as
        // previously (it was u64 previously but i64's close enough)
        let result = Offset {
            timestamp: None,
            offset: s.parse::<u64>()?,
        };
        /*
        let result = match s.chars().position(|c| c == ':') {
            None => Offset {
                timestamp: None,
                offset: s.parse::<u64>()?,
            },
            Some(_colon_position) => {
                let mut parts = s.split(':');
                let timestamp_string = parts.next().unwrap_or("0");
                let timestamp = SyncTimestamp::from_milliseconds(timestamp_string.parse::<u64>()?);
                let offset = parts.next().unwrap_or("0").parse::<u64>()?;
                Offset {
                    timestamp: Some(timestamp),
                    offset,
                }
            }
        };
        */
        Ok(result)
    }
}

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

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use actix_web::{dev::ServiceResponse, test::TestRequest, FromRequest, HttpResponse};
    use futures::executor::block_on;

    use syncstorage_db::{params, Sorting, SyncTimestamp};

    use super::{BsoQueryParams, Offset};
    use crate::web::extractors::test_utils::{extract_body_as_str, make_state};

    #[test]
    fn test_invalid_query_args() {
        let state = make_state();
        let req = TestRequest::with_uri("/?lower=-1.23&sort=whatever")
            .data(state)
            .to_http_request();
        let result = block_on(BsoQueryParams::extract(&req));
        assert!(result.is_err());
        let response: HttpResponse = result.err().unwrap().into();
        assert_eq!(response.status(), 400);
        let body = extract_body_as_str(ServiceResponse::new(req, response));
        assert_eq!(body, "0");

        /* New tests for when we can use descriptive errors
        let err: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(err["status"], 400);
        assert_eq!(err["reason"], "Bad Request");

        let (_lower_error, sort_error) = if err["errors"][0]["name"] == "lower" {
        (&err["errors"][0], &err["errors"][1])
        } else {
        (&err["errors"][1], &err["errors"][0])
        };

        assert_eq!(sort_error["location"], "querystring");
        */
    }

    #[test]
    fn test_valid_query_args() {
        let req = TestRequest::with_uri("/?ids=1,2&full=&sort=index&older=2.43")
            .data(make_state())
            .to_http_request();
        let result = block_on(BsoQueryParams::extract(&req)).unwrap();
        assert_eq!(result.ids, vec!["1", "2"]);
        assert_eq!(result.sort, Sorting::Index);
        assert_eq!(result.older.unwrap(), SyncTimestamp::from_seconds(2.43));
        assert!(result.full);
    }

    #[actix_rt::test]
    async fn test_offset() {
        let sample_offset = params::Offset {
            timestamp: Some(SyncTimestamp::default()),
            offset: 1234,
        };

        let test_offset = Offset {
            timestamp: None,
            offset: sample_offset.offset,
        };

        let offset_str = sample_offset.to_string();
        assert!(test_offset == Offset::from_str(&offset_str).unwrap())
    }
}
