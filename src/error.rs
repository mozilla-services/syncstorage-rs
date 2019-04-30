//! Error types and macros.
#![allow(clippy::single_match)]
use std::fmt;

use actix_web::{error::ResponseError, http::StatusCode, HttpResponse};
use failure::{Backtrace, Context, Fail};
use serde::ser::{Serialize, SerializeMap, SerializeSeq, Serializer};

use db::error::{DbError, DbErrorKind};
use web::error::{HawkError, ValidationError, ValidationErrorKind};
use web::extractors::RequestErrorLocation;

/// Legacy Sync 1.1 error codes, which Sync 1.5 also returns by replacing the descriptive JSON
/// information and replacing it with one of these error codes.
#[allow(dead_code)]
#[derive(Serialize)]
enum WeaveError {
    /// Unknown error
    UnknownError = 0,
    /// Illegal method/protocol
    IllegalMethod = 1,
    /// Json parse failure
    MalformedJson = 6,
    /// Invalid Weave Basic Object
    InvalidWbo = 8,
    /// User over quota
    OverQuota = 14,
    /// Size limit exceeded
    SizeLimitExceeded = 17,
}

/// Common `Result` type.
pub type ApiResult<T> = Result<T, ApiError>;

/// How long the client should wait before retrying a conflicting write.
pub const RETRY_AFTER: u8 = 10;

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
    Internal(String),

    #[fail(display = "{}", _0)]
    Validation(#[cause] ValidationError),
}

impl ApiError {
    pub fn kind(&self) -> &ApiErrorKind {
        self.inner.get_context()
    }

    pub fn is_colllection_not_found(&self) -> bool {
        match self.kind() {
            ApiErrorKind::Db(dbe) => match dbe.kind() {
                DbErrorKind::CollectionNotFound => return true,
                _ => (),
            },
            _ => (),
        }
        false
    }

    pub fn is_bso_not_found(&self) -> bool {
        match self.kind() {
            ApiErrorKind::Db(dbe) => match dbe.kind() {
                DbErrorKind::BsoNotFound => return true,
                _ => (),
            },
            _ => (),
        }
        false
    }

    pub fn is_conflict(&self) -> bool {
        match self.kind() {
            ApiErrorKind::Db(dbe) => match dbe.kind() {
                DbErrorKind::Conflict => return true,
                _ => (),
            },
            _ => (),
        }
        false
    }

    fn weave_error_code(&self) -> WeaveError {
        match self.kind() {
            ApiErrorKind::Validation(ver) => match ver.kind() {
                ValidationErrorKind::FromDetails(ref description, ref location, name) => {
                    if description == "size-limit-exceeded" {
                        return WeaveError::SizeLimitExceeded;
                    }
                    let name = name.clone().unwrap_or_else(|| "".to_owned());
                    if *location == RequestErrorLocation::Body
                        && ["bso", "bsos"].contains(&name.as_str())
                    {
                        return WeaveError::InvalidWbo;
                    }
                    WeaveError::UnknownError
                }
                _ => WeaveError::UnknownError,
            },
            _ => WeaveError::UnknownError,
        }
    }
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
            ApiErrorKind::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiErrorKind::Validation(error) => error.status,
        };

        Self { inner, status }
    }
}

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        // To return a descriptive error response, this would work. We do not
        // unfortunately do that so that we can retain Sync 1.1 backwards compatibility
        // as the Python one does.
        // HttpResponse::build(self.status).json(self)
        //
        // So instead we translate our error to a backwards compatible one
        HttpResponse::build(self.status)
            .if_true(self.is_conflict(), |resp| {
                resp.header("Retry-After", RETRY_AFTER.to_string());
            })
            .json(self.weave_error_code() as i32)
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
            ApiErrorKind::Internal(ref description) => {
                serialize_string_to_array(serializer, description)
            }
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
