//! Error types for `web` modules.
#![allow(clippy::single_match)]
use std::fmt;

use actix_web::http::{header::ToStrError, StatusCode};
use actix_web::Error as ActixError;
use base64::DecodeError;

use hawk::Error as ParseError;
use hmac::crypto_mac::{InvalidKeyLength, MacError};
use serde::{
    ser::{SerializeSeq, Serializer},
    Serialize,
};
use serde_json::{Error as JsonError, Value};
use syncstorage_common::{from_error, impl_fmt_display};

use super::extractors::RequestErrorLocation;
use crate::error::{ApiError, WeaveError};

use thiserror::Error;

/// An error occurred during HAWK authentication.
#[derive(Debug)]
pub struct HawkError {
    kind: HawkErrorKind,
}

impl HawkError {
    pub fn kind(&self) -> &HawkErrorKind {
        &self.kind
    }

    pub fn metric_label(&self) -> Option<String> {
        match self.kind() {
            HawkErrorKind::Base64(_) => Some("request.error.hawk.decode_error".to_owned()),
            HawkErrorKind::Expired => Some("request.error.hawk.expired".to_owned()),
            HawkErrorKind::Header(_) => Some("request.error.hawk.header".to_owned()),
            HawkErrorKind::Hmac(_) => Some("request.error.hawk.hmac".to_owned()),
            HawkErrorKind::InvalidHeader => Some("request.error.hawk.invalid_header".to_owned()),
            HawkErrorKind::InvalidKeyLength(_) => Some("request.error.hawk.expired".to_owned()),
            HawkErrorKind::Json(_) => Some("request.error.hawk.invalid_json".to_owned()),
            HawkErrorKind::MissingHeader => Some("request.error.hawk.missing_header".to_owned()),
            HawkErrorKind::MissingId => Some("request.error.hawk.missing_id".to_owned()),
            HawkErrorKind::MissingPrefix => Some("request.error.hawk.missing_prefix".to_owned()),
            HawkErrorKind::Parse(_) => Some("request.error.hawk.parse_error".to_owned()),
            HawkErrorKind::TruncatedId => Some("request.error.hawk.id_too_short".to_owned()),
            _ => None,
        }
    }
}

/// Causes of HAWK errors.
#[derive(Debug, Error)]
pub enum HawkErrorKind {
    #[error("{}", _0)]
    Base64(DecodeError),

    #[error("expired payload")]
    Expired,

    #[error("{}", _0)]
    Header(ToStrError),

    #[error("{}", _0)]
    Hmac(MacError),

    #[error("validation failed")]
    InvalidHeader,

    #[error("{}", _0)]
    InvalidKeyLength(InvalidKeyLength),

    #[error("{}", _0)]
    Json(JsonError),

    #[error("missing header")]
    MissingHeader,

    #[error("missing id property")]
    MissingId,

    #[error("missing path")]
    MissingPath,

    #[error("missing \"Hawk \" prefix")]
    MissingPrefix,

    #[error("{}", _0)]
    Parse(ParseError),

    #[error("id property is too short")]
    TruncatedId,
}

/// An error occurred in an Actix extractor.
#[derive(Error, Debug)]
pub struct ValidationError {
    pub status: StatusCode,
    kind: ValidationErrorKind,
}

impl ValidationError {
    pub fn metric_label(&self) -> Option<String> {
        match &self.kind {
            ValidationErrorKind::FromDetails(
                _description,
                ref _location,
                Some(ref _name),
                metric_label,
            ) => metric_label.clone(),
            ValidationErrorKind::FromValidationErrors(_errors, _location, metric_label) => {
                metric_label.clone()
            }
            _ => None,
        }
    }

    pub fn weave_error_code(&self) -> WeaveError {
        match &self.kind {
            ValidationErrorKind::FromDetails(
                ref description,
                ref location,
                name,
                ref _metric_label,
            ) => {
                match description.as_ref() {
                    "over-quota" => return WeaveError::OverQuota,
                    "size-limit-exceeded" => return WeaveError::SizeLimitExceeded,
                    _ => {}
                }
                let name = name.clone().unwrap_or_else(|| "".to_owned());
                if *location == RequestErrorLocation::Body
                    && ["bso", "bsos"].contains(&name.as_str())
                {
                    return WeaveError::InvalidWbo;
                }
                WeaveError::UnknownError
            }
            ValidationErrorKind::FromValidationErrors(ref _err, ref location, _metric_label) => {
                if *location == RequestErrorLocation::Body {
                    WeaveError::InvalidWbo
                } else {
                    WeaveError::UnknownError
                }
            }
        }
    }
}

/// Causes of extractor errors.
#[derive(Debug, Error)]
pub enum ValidationErrorKind {
    #[error("{}", _0)]
    FromDetails(String, RequestErrorLocation, Option<String>, Option<String>),

    #[error("{}", _0)]
    FromValidationErrors(
        validator::ValidationErrors,
        RequestErrorLocation,
        Option<String>,
    ),
}

impl_fmt_display!(HawkError, HawkErrorKind);
impl_fmt_display!(ValidationError, ValidationErrorKind);

from_error!(DecodeError, ApiError, HawkErrorKind::Base64);
from_error!(InvalidKeyLength, ApiError, HawkErrorKind::InvalidKeyLength);
from_error!(JsonError, ApiError, HawkErrorKind::Json);
from_error!(MacError, ApiError, HawkErrorKind::Hmac);
from_error!(ToStrError, ApiError, HawkErrorKind::Header);

impl From<HawkErrorKind> for HawkError {
    fn from(kind: HawkErrorKind) -> Self {
        Self { kind }
    }
}

impl From<ValidationErrorKind> for ValidationError {
    fn from(kind: ValidationErrorKind) -> Self {
        trace!("Validation Error: {:?}", kind);
        let status = match kind {
            ValidationErrorKind::FromDetails(ref _description, ref location, Some(ref name), _)
                if *location == RequestErrorLocation::Header =>
            {
                match name.to_ascii_lowercase().as_str() {
                    "accept" => StatusCode::NOT_ACCEPTABLE,
                    "content-type" => StatusCode::UNSUPPORTED_MEDIA_TYPE,
                    _ => StatusCode::BAD_REQUEST,
                }
            }
            ValidationErrorKind::FromDetails(ref _description, ref location, Some(ref name), _)
                if *location == RequestErrorLocation::Path
                    && ["bso", "collection"].contains(&name.as_ref()) =>
            {
                StatusCode::NOT_FOUND
            }
            _ => StatusCode::BAD_REQUEST,
        };

        Self { status, kind }
    }
}

impl From<HawkErrorKind> for ApiError {
    fn from(kind: HawkErrorKind) -> Self {
        let hawk_error: HawkError = kind.into();
        hawk_error.into()
    }
}

impl From<ParseError> for ApiError {
    fn from(inner: ParseError) -> Self {
        HawkErrorKind::Parse(inner).into()
    }
}

impl From<ValidationErrorKind> for ApiError {
    fn from(kind: ValidationErrorKind) -> Self {
        let validation_error: ValidationError = kind.into();
        validation_error.into()
    }
}

impl From<ValidationErrorKind> for ActixError {
    fn from(kind: ValidationErrorKind) -> Self {
        let api_error: ApiError = kind.into();
        api_error.into()
    }
}

impl Serialize for ValidationError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Serialize::serialize(&self.kind, serializer)
    }
}

impl Serialize for ValidationErrorKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(None)?;
        match *self {
            ValidationErrorKind::FromDetails(
                ref description,
                ref location,
                ref name,
                ref _metric_label,
            ) => {
                seq.serialize_element(&SerializedValidationError {
                    description,
                    location,
                    name: name.as_ref().map(|name| &**name),
                    value: None,
                })?;
            }

            ValidationErrorKind::FromValidationErrors(
                ref errors,
                ref location,
                ref _metric_label,
            ) => {
                for (field, field_errors) in errors.clone().field_errors().iter() {
                    for field_error in field_errors.iter() {
                        seq.serialize_element(&SerializedValidationError {
                            description: &field_error.code,
                            location,
                            name: Some(field),
                            value: field_error.params.get("value"),
                        })?;
                    }
                }
            }
        }

        seq.end()
    }
}

#[derive(Debug, Serialize)]
struct SerializedValidationError<'e> {
    pub description: &'e str,
    pub location: &'e RequestErrorLocation,
    pub name: Option<&'e str>,
    pub value: Option<&'e Value>,
}
