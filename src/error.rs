//! Error types and macros.

use std::fmt;

use actix_web::{error::ResponseError, http::StatusCode, HttpResponse};
use failure::{Backtrace, Context, Fail};
use serde::ser::{Serialize, SerializeMap, SerializeSeq, Serializer};

use db::error::DbError;
use web::error::{HawkError, ValidationError};

/// Common `Result` type.
pub type ApiResult<T> = Result<T, ApiError>;

/// Top-level error type.
#[derive(Debug)]
pub struct ApiError {
    inner: Context<ApiErrorKind>,
    status: StatusCode,
}

/// Top-level ErrorKind.
#[derive(Debug, Fail)]
pub enum ApiErrorKind {
    #[fail(display = "{}", _0)]
    Db(#[cause] DbError),

    #[fail(display = "HAWK authentication error: {}", _0)]
    Hawk(#[cause] HawkError),

    #[fail(display = "{}", _0)]
    Validation(#[cause] ValidationError),
}

impl From<ApiError> for HttpResponse {
    fn from(inner: ApiError) -> Self {
        ResponseError::error_response(&inner)
    }
}

impl From<Context<ApiErrorKind>> for ApiError {
    fn from(inner: Context<ApiErrorKind>) -> Self {
        let status = match inner.get_context() {
            ApiErrorKind::Db(error) => error.status,
            ApiErrorKind::Hawk(_) => StatusCode::UNAUTHORIZED,
            ApiErrorKind::Validation(_) => StatusCode::BAD_REQUEST,
        };

        Self { inner, status }
    }
}

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status).json(self)
    }
}

impl Serialize for ApiError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let size = if self.status == StatusCode::UNAUTHORIZED {
            2
        } else {
            3
        };

        let mut map = serializer.serialize_map(Some(size))?;
        map.serialize_entry("status", &self.status.as_u16())?;
        map.serialize_entry("reason", self.status.canonical_reason().unwrap_or(""))?;

        if self.status != StatusCode::UNAUTHORIZED {
            map.serialize_entry("errors", &self.inner.get_context())?;
        }

        map.end()
    }
}

impl Serialize for ApiErrorKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            ApiErrorKind::Db(ref error) => serialize_string_to_array(serializer, error),
            ApiErrorKind::Hawk(ref error) => serialize_string_to_array(serializer, error),
            ApiErrorKind::Validation(ref error) => Serialize::serialize(error, serializer),
        }
    }
}

fn serialize_string_to_array<S, V>(serializer: S, value: V) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    V: fmt::Display,
{
    let mut seq = serializer.serialize_seq(Some(1))?;
    seq.serialize_element(&value.to_string())?;
    seq.end()
}

// XXX: We can remove this if/when db methods return ApiError directly
impl ResponseError for DbError {}

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

        impl fmt::Display for $error {
            fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                fmt::Display::fmt(&self.inner, formatter)
            }
        }

        impl From<$kind> for $error {
            fn from(kind: $kind) -> Self {
                Context::new(kind).into()
            }
        }
    };
}

failure_boilerplate!(ApiError, ApiErrorKind);

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
from_error!(ValidationError, ApiError, ApiErrorKind::Validation);
