//! Error types and macros.
// TODO: Currently `Validation(#[cause] ValidationError)` may trigger some
// performance issues. The suggested fix is to Box ValidationError, however
// this cascades into Failure requiring std::error::Error being implemented
// which is out of scope.
#![allow(clippy::single_match, clippy::large_enum_variant)]

use std::convert::From;
use std::fmt;

use failure::{Backtrace, Context, Fail};
use serde::{
    ser::{SerializeMap, SerializeSeq, Serializer},
    Serialize,
};

/// Common `Result` type.
pub type ApiResult<T> = Result<T, ApiError>;

/// How long the client should wait before retrying a conflicting write.
pub const RETRY_AFTER: u8 = 10;

/// Top-level error type.
#[derive(Debug)]
pub struct ApiError {
    inner: Context<ApiErrorKind>,
}

/// Top-level ErrorKind.
#[derive(Debug, Fail)]
pub enum ApiErrorKind {
    #[fail(display = "{}", _0)]
    Internal(String),
}

impl ApiError {
    pub fn kind(&self) -> &ApiErrorKind {
        self.inner.get_context()
    }
}

impl From<std::io::Error> for ApiError {
    fn from(inner: std::io::Error) -> Self {
        ApiErrorKind::Internal(inner.to_string()).into()
    }
}

impl Serialize for ApiError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(3))?;
        map.end()
    }
}

impl From<Context<ApiErrorKind>> for ApiError {
    fn from(inner: Context<ApiErrorKind>) -> Self {
        Self { inner }
    }
}

impl Serialize for ApiErrorKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            ApiErrorKind::Internal(ref description) => {
                serialize_string_to_array(serializer, description)
            }
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
            fn cause(&self) -> Option<&dyn Fail> {
                self.inner.cause()
            }

            fn backtrace(&self) -> Option<&Backtrace> {
                self.inner.backtrace()
            }
        }

        impl fmt::Display for $error {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
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

/*
macro_rules! from_error {
    ($from:ty, $to:ty, $to_kind:expr) => {
        impl From<$from> for $to {
            fn from(inner: $from) -> $to {
                $to_kind(inner).into()
            }
        }
    };
}
*/
