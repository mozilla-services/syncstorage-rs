//! Error types for `web` modules.

use std::fmt;

use actix_web::http::header::ToStrError;
use base64::DecodeError;
use failure::{Backtrace, Context, Fail, SyncFailure};
use hawk::Error as ParseError;
use hmac::crypto_mac::{InvalidKeyLength, MacError};
use serde_json::Error as JsonError;

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

failure_boilerplate!(HawkError, HawkErrorKind);

from_error!(DecodeError, ApiError, HawkErrorKind::Base64);
from_error!(InvalidKeyLength, ApiError, HawkErrorKind::InvalidKeyLength);
from_error!(JsonError, ApiError, HawkErrorKind::Json);
from_error!(MacError, ApiError, HawkErrorKind::Hmac);
from_error!(ToStrError, ApiError, HawkErrorKind::Header);

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
