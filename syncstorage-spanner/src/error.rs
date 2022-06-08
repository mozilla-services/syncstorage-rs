use std::fmt;

use backtrace::Backtrace;
use http::StatusCode;
use syncserver_common::{from_error, impl_fmt_display};
use syncserver_db_common::error::{CommonDbError, DbErrorIntrospect};
use thiserror::Error;

#[derive(Debug)]
pub struct DbError {
    kind: DbErrorKind,
    pub status: StatusCode,
    pub backtrace: Backtrace,
}

impl DbError {
    pub fn batch_not_found() -> Self {
        DbErrorKind::Common(CommonDbError::batch_not_found()).into()
    }

    pub fn bso_not_found() -> Self {
        DbErrorKind::Common(CommonDbError::bso_not_found()).into()
    }

    pub fn collection_not_found() -> Self {
        DbErrorKind::Common(CommonDbError::collection_not_found()).into()
    }

    pub fn conflict() -> Self {
        DbErrorKind::Common(CommonDbError::conflict()).into()
    }

    pub fn expired() -> Self {
        DbErrorKind::Expired.into()
    }

    pub fn integrity(msg: String) -> Self {
        DbErrorKind::Integrity(msg).into()
    }

    pub fn internal(msg: String) -> Self {
        DbErrorKind::Common(CommonDbError::internal(msg)).into()
    }

    pub fn quota() -> Self {
        DbErrorKind::Common(CommonDbError::quota()).into()
    }

    pub fn too_large(msg: String) -> Self {
        DbErrorKind::TooLarge(msg).into()
    }
}

#[derive(Debug, Error)]
enum DbErrorKind {
    #[error("{}", _0)]
    Common(CommonDbError),

    #[error("Connection expired")]
    Expired,

    #[error("A database error occurred: {}", _0)]
    Grpc(#[from] grpcio::Error),

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
            backtrace: Backtrace::new(),
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

    fn is_sentry_event(&self) -> bool {
        matches!(&self.kind, DbErrorKind::Common(e) if e.is_sentry_event())
    }

    fn metric_label(&self) -> Option<String> {
        if let DbErrorKind::Common(e) = &self.kind {
            e.metric_label()
        } else {
            None
        }
    }
}

impl_fmt_display!(DbError, DbErrorKind);

from_error!(grpcio::Error, DbError, |inner: grpcio::Error| {
    // Convert ABORTED (typically due to a transaction abort) into 503s
    match inner {
        grpcio::Error::RpcFailure(ref status) | grpcio::Error::RpcFinished(Some(ref status))
            if status.code() == grpcio::RpcStatusCode::ABORTED =>
        {
            DbErrorKind::Common(CommonDbError::conflict())
        }
        _ => DbErrorKind::Grpc(inner),
    }
});
from_error!(CommonDbError, DbError, DbErrorKind::Common);
