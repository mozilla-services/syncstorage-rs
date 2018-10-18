//! Error types for `web` modules.

use std::fmt;

use actix_web::{self, http::header::ToStrError};
use base64::DecodeError;
use failure::{Backtrace, Context, Fail, SyncFailure};
use hawk::Error as ParseError;
use hmac::crypto_mac::{InvalidKeyLength, MacError};
use serde::ser::{Serialize, SerializeMap, Serializer};
use serde_json::Error as JsonError;
use validator;

use super::extractors::RequestErrorLocation;
use error::ApiError;

/// An error occurred during HAWK authentication.
#[derive(Debug)]
pub struct HawkError {
    inner: Context<HawkErrorKind>,
}

/// Causes of HAWK errors.
#[derive(Debug, Fail)]
pub enum HawkErrorKind {
    #[fail(display = "{}", _0)]
    Base64(#[cause] DecodeError),

    #[fail(display = "expired payload")]
    Expired,

    #[fail(display = "{}", _0)]
    Header(#[cause] ToStrError),

    #[fail(display = "{}", _0)]
    Hmac(MacError),

    #[fail(display = "validation failed")]
    InvalidHeader,

    #[fail(display = "{}", _0)]
    InvalidKeyLength(InvalidKeyLength),

    #[fail(display = "{}", _0)]
    Json(#[cause] JsonError),

    #[fail(display = "missing header")]
    MissingHeader,

    #[fail(display = "missing id property")]
    MissingId,

    #[fail(display = "missing path")]
    MissingPath,

    #[fail(display = "missing \"Hawk \" prefix")]
    MissingPrefix,

    #[fail(display = "{}", _0)]
    Parse(SyncFailure<ParseError>),

    #[fail(display = "id property is too short")]
    TruncatedId,
}

/// An error occurred in an Actix extractor.
#[derive(Debug)]
pub struct ValidationError {
    inner: Context<ValidationErrorKind>,
}

/// Causes of extractor errors.
#[derive(Debug, Fail)]
pub enum ValidationErrorKind {
    #[fail(display = "{}", _0)]
    WithLocation(#[cause] validator::ValidationErrors, RequestErrorLocation),

    #[fail(display = "Conflicting headers: {}, {}", _0, _1)]
    HeaderConflict(String, String),

    #[fail(display = "Invalid {} header: {}", _0, _1)]
    InvalidHeader(String, String),

    #[fail(display = "Invalid {} path component: {}", _0, _1)]
    InvalidPathComponent(String, String),

    #[fail(display = "Invalid query string: {}", _0)]
    InvalidQueryString(String),

    #[fail(display = "Invalid request: {}", _0)]
    InvalidRequest(actix_web::Error),

    #[fail(display = "User id in path does not match payload")]
    MismatchedUserId,
}

failure_boilerplate!(HawkError, HawkErrorKind);
failure_boilerplate!(ValidationError, ValidationErrorKind);

from_error!(DecodeError, ApiError, HawkErrorKind::Base64);
from_error!(InvalidKeyLength, ApiError, HawkErrorKind::InvalidKeyLength);
from_error!(JsonError, ApiError, HawkErrorKind::Json);
from_error!(MacError, ApiError, HawkErrorKind::Hmac);
from_error!(ToStrError, ApiError, HawkErrorKind::Header);

impl From<Context<HawkErrorKind>> for HawkError {
    fn from(inner: Context<HawkErrorKind>) -> Self {
        Self { inner }
    }
}

impl From<Context<ValidationErrorKind>> for ValidationError {
    fn from(inner: Context<ValidationErrorKind>) -> Self {
        Self { inner }
    }
}

impl From<HawkErrorKind> for ApiError {
    fn from(kind: HawkErrorKind) -> Self {
        let hawk_error: HawkError = Context::new(kind).into();
        hawk_error.into()
    }
}

impl From<ParseError> for ApiError {
    fn from(inner: ParseError) -> Self {
        HawkErrorKind::Parse(SyncFailure::new(inner)).into()
    }
}

impl From<ValidationErrorKind> for ApiError {
    fn from(kind: ValidationErrorKind) -> Self {
        let validation_error: ValidationError = Context::new(kind).into();
        validation_error.into()
    }
}

impl Serialize for ValidationError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Serialize::serialize(&self.inner.get_context(), serializer)
    }
}

impl Serialize for ValidationErrorKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            ValidationErrorKind::WithLocation(ref errors, ref location) => {
                let mut map = serializer.serialize_map(Some(2))?;
                map.serialize_entry("location", &location)?;
                map.serialize_entry("errors", &errors)?;
                map.end()
            }
            _ => serializer.serialize_str(&self.to_string()),
        }
    }
}
