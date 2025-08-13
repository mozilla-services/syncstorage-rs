use std::fmt;

use backtrace::Backtrace;
use grpcio::RpcStatusCode;
use http::StatusCode;
use syncserver_common::{from_error, impl_fmt_display, InternalError, ReportableError};
use syncstorage_db_common::error::{DbErrorIntrospect, SyncstorageDbError};
use thiserror::Error;

/// An error type that represents any Spanner-related errors that may occur while processing a
/// syncstorage request. These errors may be application-specific or lower-level errors that arise
/// from the database backend.
#[derive(Debug)]
pub struct DbError {
    pub(crate) kind: DbErrorKind,
    pub status: StatusCode,
    pub backtrace: Box<Backtrace>,
}

impl DbError {
    pub fn batch_not_found() -> Self {
        DbErrorKind::Common(SyncstorageDbError::batch_not_found()).into()
    }

    pub fn bso_not_found() -> Self {
        DbErrorKind::Common(SyncstorageDbError::bso_not_found()).into()
    }

    pub fn collection_not_found() -> Self {
        DbErrorKind::Common(SyncstorageDbError::collection_not_found()).into()
    }

    pub fn conflict() -> Self {
        DbErrorKind::Common(SyncstorageDbError::conflict()).into()
    }

    pub fn expired() -> Self {
        DbErrorKind::Expired.into()
    }

    pub fn integrity(msg: String) -> Self {
        DbErrorKind::Integrity(msg).into()
    }

    pub fn internal(msg: String) -> Self {
        DbErrorKind::Common(SyncstorageDbError::internal(msg)).into()
    }

    pub fn quota() -> Self {
        DbErrorKind::Common(SyncstorageDbError::quota()).into()
    }

    pub fn too_large(msg: String) -> Self {
        DbErrorKind::TooLarge(msg).into()
    }

    pub fn pool_timeout(timeout_type: deadpool::managed::TimeoutType) -> Self {
        DbErrorKind::PoolTimeout(timeout_type).into()
    }
}

#[derive(Debug, Error)]
pub(crate) enum DbErrorKind {
    #[error("{}", _0)]
    Common(SyncstorageDbError),

    #[error("Connection expired")]
    Expired,

    #[error("A database error occurred: {}", _0)]
    Grpc(#[from] grpcio::Error),

    #[error("A database pool timeout occurred, type: {:?}", _0)]
    PoolTimeout(deadpool::managed::TimeoutType),

    #[error("Database integrity error: {}", _0)]
    Integrity(String),

    #[error("Spanner data load too large: {}", _0)]
    TooLarge(String),
}

impl From<DbErrorKind> for DbError {
    fn from(kind: DbErrorKind) -> Self {
        let status = match &kind {
            DbErrorKind::Common(e) => e.status,
            // Matching the Python code here (a 400 vs 404)
            DbErrorKind::TooLarge(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        Self {
            kind,
            status,
            backtrace: Box::new(Backtrace::new()),
        }
    }
}

impl DbErrorIntrospect for DbError {
    fn is_batch_not_found(&self) -> bool {
        matches!(&self.kind, DbErrorKind::Common(e) if e.is_batch_not_found())
    }

    fn is_bso_not_found(&self) -> bool {
        matches!(&self.kind, DbErrorKind::Common(e) if e.is_bso_not_found())
    }

    fn is_collection_not_found(&self) -> bool {
        matches!(&self.kind, DbErrorKind::Common(e) if e.is_collection_not_found())
    }

    fn is_conflict(&self) -> bool {
        matches!(&self.kind, DbErrorKind::Common(e) if e.is_conflict())
    }

    fn is_quota(&self) -> bool {
        matches!(&self.kind, DbErrorKind::Common(e) if e.is_quota())
    }
}

impl ReportableError for DbError {
    fn reportable_source(&self) -> Option<&(dyn ReportableError + 'static)> {
        match &self.kind {
            DbErrorKind::Common(e) => Some(e),
            _ => None,
        }
    }

    fn is_sentry_event(&self) -> bool {
        match &self.kind {
            DbErrorKind::Common(e) => e.is_sentry_event(),
            // Match against server/connection errors that we don't want reported to Sentry.
            DbErrorKind::Grpc(grpcio::Error::RpcFailure(status)) => {
                match status.code() {
                    // Code 14 - UNAVAILABLE
                    RpcStatusCode::UNAVAILABLE => false,
                    // Code 13 - INTERNAL
                    RpcStatusCode::INTERNAL => !is_ignored_internal(status),
                    _ => true,
                }
            }
            DbErrorKind::PoolTimeout(_) => false,
            _ => true,
        }
    }

    fn metric_label(&self) -> Option<&str> {
        match &self.kind {
            DbErrorKind::Common(e) => e.metric_label(),

            DbErrorKind::Grpc(grpcio::Error::RpcFailure(status)) => {
                match status.code() {
                    // Code 14 - UNAVAILABLE
                    RpcStatusCode::UNAVAILABLE => Some("storage.spanner.grpc.unavailable"),
                    // Code 13 - INTERNAL
                    RpcStatusCode::INTERNAL => Some("storage.spanner.grpc.internal"),

                    _ => None,
                }
            }
            DbErrorKind::PoolTimeout(_) => Some("storage.spanner.pool.timeout"),
            _ => None,
        }
    }

    fn tags(&self) -> Vec<(&str, String)> {
        match &self.kind {
            DbErrorKind::PoolTimeout(timeout_type) => {
                vec![("type", format!("{timeout_type:?}").to_ascii_lowercase())]
            }
            _ => vec![],
        }
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        Some(&self.backtrace)
    }
}

/// Whether to ignore a 13 - INTERNAL error based on its status
fn is_ignored_internal(status: &grpcio::RpcStatus) -> bool {
    [
        "rst_stream",
        "rst stream",
        "received unexpected eos on data frame from server",
    ]
    .contains(&status.message().to_lowercase().as_str())
}

impl InternalError for DbError {
    fn internal_error(message: String) -> Self {
        DbErrorKind::Common(SyncstorageDbError::internal(message)).into()
    }
}

impl_fmt_display!(DbError, DbErrorKind);

from_error!(grpcio::Error, DbError, |inner: grpcio::Error| {
    // Convert ABORTED (typically due to a transaction abort) into 503s
    match inner {
        grpcio::Error::RpcFailure(ref status) | grpcio::Error::RpcFinished(Some(ref status))
            if status.code() == grpcio::RpcStatusCode::ABORTED =>
        {
            DbErrorKind::Common(SyncstorageDbError::conflict())
        }
        _ => DbErrorKind::Grpc(inner),
    }
});
from_error!(SyncstorageDbError, DbError, DbErrorKind::Common);
