//! Error types and macros.
// TODO: Currently `Validation(#[cause] ValidationError)` may trigger some
// performance issues. The suggested fix is to Box ValidationError, however
// this cascades into Failure requiring std::error::Error being implemented
// which is out of scope.
#![allow(clippy::single_match, clippy::large_enum_variant)]
use backtrace::Backtrace;
use std::convert::From;
use std::fmt;

use actix_web::{
    dev::ServiceResponse, error::ResponseError, http::StatusCode, middleware::ErrorHandlerResponse,
    HttpResponse, HttpResponseBuilder, Result,
};

use serde::{
    ser::{SerializeMap, SerializeSeq, Serializer},
    Serialize,
};

use syncserver_common::{from_error, impl_fmt_display, MetricError, ReportableError};
use syncstorage_db::{DbError, DbErrorIntrospect};

use thiserror::Error;

use crate::web::error::{HawkError, ValidationError};
use std::error::Error;

/// Legacy Sync 1.1 error codes, which Sync 1.5 also returns by replacing the descriptive JSON
/// information and replacing it with one of these error codes.
#[allow(dead_code)]
#[derive(Serialize)]
pub enum WeaveError {
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
    kind: ApiErrorKind,
    pub(crate) backtrace: Box<Backtrace>,
    status: StatusCode,
}

/// Top-level ErrorKind.
#[derive(Error, Debug)]
pub enum ApiErrorKind {
    // Note, `#[from]` applies some derivation to the target error, but can fail
    // if the target has any complexity associated with it. It's best to add
    // #[derive(thiserror::Error,...)] to the target error to ensure that various
    // traits are defined.
    #[error("{}", _0)]
    Db(DbError),

    #[error("HAWK authentication error: {}", _0)]
    Hawk(HawkError),

    #[error("No app_data ServerState")]
    NoServerState,

    #[error("{}", _0)]
    Internal(String),

    #[error("{}", _0)]
    Validation(ValidationError),
}

impl ApiErrorKind {
    pub fn metric_label(&self) -> Option<String> {
        match self {
            ApiErrorKind::Hawk(err) => err.metric_label(),
            ApiErrorKind::Db(err) => err.metric_label(),
            ApiErrorKind::Validation(err) => err.metric_label(),
            _ => None,
        }
    }
}

impl ApiError {
    pub fn is_sentry_event(&self) -> bool {
        // Should we report this error to sentry?
        self.status.is_server_error()
            && match &self.kind {
                ApiErrorKind::Db(dbe) => dbe.is_sentry_event(),
                _ => self.kind.metric_label().is_none(),
            }
    }

    fn weave_error_code(&self) -> WeaveError {
        match &self.kind {
            ApiErrorKind::Validation(ver) => ver.weave_error_code(),
            ApiErrorKind::Db(dber) if dber.is_quota() => WeaveError::OverQuota,
            _ => WeaveError::UnknownError,
        }
    }

    pub fn render_404<B>(res: ServiceResponse<B>) -> Result<ErrorHandlerResponse<B>> {
        if res.request().path().starts_with("/1.0/") {
            // Do not use a custom response for Tokenserver requests.
            Ok(ErrorHandlerResponse::Response(res.map_into_left_body()))
        } else {
            // Replace the outbound error message with our own for Sync requests.
            let resp = HttpResponseBuilder::new(StatusCode::NOT_FOUND)
                .json(WeaveError::UnknownError as u32);
            Ok(ErrorHandlerResponse::Response(ServiceResponse::new(
                res.request().clone(),
                resp.map_into_right_body(),
            )))
        }
    }

    pub fn is_collection_not_found(&self) -> bool {
        matches!(&self.kind, ApiErrorKind::Db(dbe) if dbe.is_collection_not_found())
    }

    pub fn is_conflict(&self) -> bool {
        matches!(&self.kind, ApiErrorKind::Db(dbe) if dbe.is_conflict())
    }

    pub fn is_quota(&self) -> bool {
        matches!(&self.kind, ApiErrorKind::Db(dbe) if dbe.is_quota())
    }

    pub fn is_bso_not_found(&self) -> bool {
        matches!(&self.kind, ApiErrorKind::Db(dbe) if dbe.is_bso_not_found())
    }
}

impl Error for ApiError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.kind.source()
    }
}

impl From<ApiError> for HttpResponse {
    fn from(inner: ApiError) -> Self {
        ResponseError::error_response(&inner)
    }
}

impl From<MetricError> for ApiError {
    fn from(inner: MetricError) -> Self {
        ApiErrorKind::Internal(inner.to_string()).into()
    }
}

impl From<std::io::Error> for ApiError {
    fn from(inner: std::io::Error) -> Self {
        ApiErrorKind::Internal(inner.to_string()).into()
    }
}

impl From<tracing::subscriber::SetGlobalDefaultError> for ApiError {
    fn from(_: tracing::subscriber::SetGlobalDefaultError) -> Self {
        ApiErrorKind::Internal("Logging failed to initialize".to_string()).into()
    }
}

impl From<ApiErrorKind> for ApiError {
    fn from(kind: ApiErrorKind) -> Self {
        let status = match &kind {
            ApiErrorKind::Db(error) => error.status,
            ApiErrorKind::Hawk(_) => StatusCode::UNAUTHORIZED,
            ApiErrorKind::NoServerState | ApiErrorKind::Internal(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            ApiErrorKind::Validation(error) => error.status,
        };

        Self {
            kind,
            backtrace: Box::new(Backtrace::new()),
            status,
        }
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
        let mut resp = HttpResponse::build(self.status);
        if self.is_conflict() {
            resp.insert_header(("Retry-After", RETRY_AFTER.to_string()));
        };
        resp.json(self.weave_error_code() as i32)
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
            map.serialize_entry("errors", &self.kind)?;
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
            ApiErrorKind::NoServerState => {
                Serialize::serialize("No State information found", serializer)
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

impl_fmt_display!(ApiError, ApiErrorKind);

impl From<DbError> for ApiError {
    fn from(db_error: DbError) -> Self {
        Self {
            status: db_error.status,
            backtrace: db_error.backtrace.clone(),
            kind: ApiErrorKind::Db(db_error),
        }
    }
}

from_error!(HawkError, ApiError, ApiErrorKind::Hawk);
from_error!(ValidationError, ApiError, ApiErrorKind::Validation);

impl ReportableError for ApiError {
    fn error_backtrace(&self) -> String {
        format!("{:#?}", self.backtrace)
    }

    fn is_sentry_event(&self) -> bool {
        // Should we report this error to sentry?
        self.status.is_server_error()
            && match &self.kind {
                ApiErrorKind::Db(dbe) => dbe.is_sentry_event(),
                _ => self.kind.metric_label().is_none(),
            }
    }

    fn metric_label(&self) -> Option<String> {
        self.kind.metric_label()
    }
}
