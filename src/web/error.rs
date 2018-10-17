use std::fmt::{self, Display, Formatter};

use actix_web::{error::ResponseError, http::header::ToStrError};
use base64::DecodeError;
use failure::{Backtrace, Context, Fail};
use hawk::Error as ParseError;
use hmac::crypto_mac::{InvalidKeyLength, MacError};
use serde_json::Error as JsonError;

use db::error::DbError;

#[derive(Debug)]
pub struct ApiError {
    inner: Context<ApiErrorKind>,
}

#[derive(Debug, Fail)]
pub enum ApiErrorKind {
    #[fail(display = "{}", _0)]
    Db(#[cause] DbError),

    #[fail(display = "HAWK authentication error: {}", _0)]
    Hawk(#[cause] HawkError),
}

#[derive(Debug)]
pub struct HawkError {
    inner: Context<HawkErrorKind>,
}

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

macro_rules! failure_boilerplate {
    ($error:ty, $kind:ty) => {
        impl Fail for $error {
            fn cause(&self) -> Option<&Fail> {
                self.inner.cause()
            }

            fn backtrace(&self) -> Option<&Backtrace> {
                self.inner.backtrace()
            }
        }

        impl Display for $error {
            fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
                Display::fmt(&self.inner, formatter)
            }
        }

        impl From<$kind> for $error {
            fn from(kind: $kind) -> Self {
                Context::new(kind).into()
            }
        }

        impl From<Context<$kind>> for $error {
            fn from(inner: Context<$kind>) -> Self {
                Self { inner }
            }
        }
    };
}

failure_boilerplate!(ApiError, ApiErrorKind);
failure_boilerplate!(HawkError, HawkErrorKind);

macro_rules! from_error {
    ($from:ty, $to:ty, $to_kind:expr) => {
        impl From<$from> for $to {
            fn from(inner: $from) -> $to {
                $to_kind(inner).into()
            }
        }
    };
}

from_error!(DbError, ApiError, ApiErrorKind::Db);
from_error!(HawkError, ApiError, ApiErrorKind::Hawk);

from_error!(DecodeError, ApiError, HawkErrorKind::Base64);
from_error!(InvalidKeyLength, ApiError, HawkErrorKind::InvalidKeyLength);
from_error!(JsonError, ApiError, HawkErrorKind::Json);
from_error!(MacError, ApiError, HawkErrorKind::Hmac);
from_error!(ToStrError, ApiError, HawkErrorKind::Header);
from_error!(ParseError, ApiError, HawkErrorKind::Parse);

impl ResponseError for ApiError {}
unsafe impl Send for ApiError {}
unsafe impl Sync for ApiError {}
unsafe impl Send for ApiErrorKind {}
unsafe impl Sync for ApiErrorKind {}
unsafe impl Send for HawkError {}
unsafe impl Sync for HawkError {}
unsafe impl Send for HawkErrorKind {}
unsafe impl Sync for HawkErrorKind {}

impl From<HawkErrorKind> for ApiError {
    fn from(kind: HawkErrorKind) -> Self {
        let hawk_error: HawkError = Context::new(kind).into();
        hawk_error.into()
    }
}
