//! Error types and macros.

use std::fmt;

use actix_web::error::ResponseError;
use failure::{Backtrace, Context, Fail};

use db::error::DbError;
use web::error::HawkError;

/// Common `Result` type.
pub type ApiResult<T> = Result<T, ApiError>;

/// Top-level error type.
#[derive(Debug)]
pub struct ApiError {
    inner: Context<ApiErrorKind>,
}

/// Top-level ErrorKind.
#[derive(Debug, Fail)]
pub enum ApiErrorKind {
    #[fail(display = "{}", _0)]
    Db(#[cause] DbError),

    #[fail(display = "HAWK authentication error: {}", _0)]
    Hawk(#[cause] HawkError),
}

impl ResponseError for ApiError {}

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

        impl From<Context<$kind>> for $error {
            fn from(inner: Context<$kind>) -> Self {
                Self { inner }
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
