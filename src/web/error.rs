//! Error types for `web` modules.
#![allow(clippy::single_match)]
use std::fmt;

use actix_web::http::{header::ToStrError, StatusCode};
use actix_web::Error as ActixError;
use base64::DecodeError;
use failure::{Backtrace, Context, Fail};
use hawk::Error as ParseError;
use hmac::crypto_mac::{InvalidKeyLength, MacError};
use serde::{
    ser::{SerializeSeq, Serializer},
    Serialize,
};
use serde_json::{Error as JsonError, Value};

use super::extractors::RequestErrorLocation;
use super::tags::Tags;
use crate::error::ApiError;

/// An error occurred during HAWK authentication.
#[derive(Debug)]
pub struct HawkError {
    inner: Context<HawkErrorKind>,
}

impl HawkError {
    pub fn kind(&self) -> &HawkErrorKind {
        self.inner.get_context()
    }

    pub fn is_reportable(&self) -> bool {
        match &self.kind() {
            HawkErrorKind::TruncatedId => true,
            HawkErrorKind::Parse(_) => true,
            _ => false,
        }
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
    Parse(ParseError),

    #[fail(display = "id property is too short")]
    TruncatedId,
}

/// An error occurred in an Actix extractor.
#[derive(Debug)]
pub struct ValidationError {
    pub status: StatusCode,
    inner: Context<ValidationErrorKind>,
}

impl ValidationError {
    pub fn kind(&self) -> &ValidationErrorKind {
        self.inner.get_context()
    }
}

/// Causes of extractor errors.
#[derive(Debug, Fail)]
pub enum ValidationErrorKind {
    #[fail(display = "{}", _0)]
    FromDetails(
        String,
        RequestErrorLocation,
        Option<String>,
        Option<Tags>,
        Option<String>,
    ),

    #[fail(display = "{}", _0)]
    FromValidationErrors(
        #[cause] validator::ValidationErrors,
        RequestErrorLocation,
        Option<Tags>,
        Option<String>,
    ),
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
        debug!("Validation Error: {:?}", inner.get_context());
        let status = match inner.get_context() {
            ValidationErrorKind::FromDetails(
                ref _description,
                ref location,
                Some(ref name),
                _,
                _,
            ) if *location == RequestErrorLocation::Header => {
                match name.to_ascii_lowercase().as_str() {
                    "accept" => StatusCode::NOT_ACCEPTABLE,
                    "content-type" => StatusCode::UNSUPPORTED_MEDIA_TYPE,
                    _ => StatusCode::BAD_REQUEST,
                }
            }
            ValidationErrorKind::FromDetails(
                ref _description,
                ref location,
                Some(ref name),
                _,
                _,
            ) if *location == RequestErrorLocation::Path
                && ["bso", "collection"].contains(&name.as_ref()) =>
            {
                StatusCode::NOT_FOUND
            }
            _ => StatusCode::BAD_REQUEST,
        };

        Self { inner, status }
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
        HawkErrorKind::Parse(inner).into()
    }
}

impl From<ValidationErrorKind> for ApiError {
    fn from(kind: ValidationErrorKind) -> Self {
        let validation_error: ValidationError = Context::new(kind).into();
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
        Serialize::serialize(&self.inner.get_context(), serializer)
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
                ref tags,
                ref _metric_label,
            ) => {
                seq.serialize_element(&SerializedValidationError {
                    description,
                    location,
                    name: name.as_ref().map(|name| &**name),
                    value: None,
                    tags: tags.as_ref(),
                })?;
            }

            ValidationErrorKind::FromValidationErrors(
                ref errors,
                ref location,
                ref tags,
                ref _metric_label,
            ) => {
                for (field, field_errors) in errors.clone().field_errors().iter() {
                    for field_error in field_errors.iter() {
                        seq.serialize_element(&SerializedValidationError {
                            description: &field_error.code,
                            location,
                            name: Some(field),
                            value: field_error.params.get("value"),
                            tags: tags.clone().as_ref(),
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
    pub tags: Option<&'e Tags>,
}
