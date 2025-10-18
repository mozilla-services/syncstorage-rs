use std::fmt;

use backtrace::Backtrace;
use http::StatusCode;
use syncserver_common::{from_error, impl_fmt_display, InternalError, ReportableError};
use syncserver_db_common::error::SqlError;
use thiserror::Error;

use super::error::{DbErrorIntrospect, SyncstorageDbError};

/// An error type that represents any diesel-related errors that may occur while processing a
/// syncstorage request. These errors may be application-specific or lower-level errors that arise
/// from the database backend.
#[derive(Debug)]
pub struct DbError {
    kind: DbErrorKind,
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

    pub fn internal(msg: String) -> Self {
        DbErrorKind::Common(SyncstorageDbError::internal(msg)).into()
    }

    pub fn quota() -> Self {
        DbErrorKind::Common(SyncstorageDbError::quota()).into()
    }

    pub fn pool_timeout(timeout_type: deadpool::managed::TimeoutType) -> Self {
        DbErrorKind::PoolTimeout(timeout_type).into()
    }
}

#[derive(Debug, Error)]
enum DbErrorKind {
    #[error("{}", _0)]
    Common(SyncstorageDbError),

    #[error("{}", _0)]
    Diesel(SqlError),

    #[error("A database pool timeout occurred, type: {:?}", _0)]
    PoolTimeout(deadpool::managed::TimeoutType),
}

impl From<DbErrorKind> for DbError {
    fn from(kind: DbErrorKind) -> Self {
        match &kind {
            DbErrorKind::Common(dbe) => Self {
                status: dbe.status,
                backtrace: Box::new(dbe.backtrace.clone()),
                kind,
            },
            _ => Self {
                kind,
                status: StatusCode::INTERNAL_SERVER_ERROR,
                backtrace: Box::new(Backtrace::new()),
            },
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
        Some(match &self.kind {
            DbErrorKind::Common(e) => e,
            DbErrorKind::Diesel(e) => e,
            _ => return None,
        })
    }

    fn is_sentry_event(&self) -> bool {
        match &self.kind {
            DbErrorKind::Common(e) => e.is_sentry_event(),
            DbErrorKind::Diesel(e) => e.is_sentry_event(),
            DbErrorKind::PoolTimeout(_) => false,
        }
    }

    fn metric_label(&self) -> Option<&str> {
        match &self.kind {
            DbErrorKind::Common(e) => e.metric_label(),
            DbErrorKind::Diesel(e) => e.metric_label(),
            DbErrorKind::PoolTimeout(_) => Some("storage.diesel.pool.timeout"),
        }
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        match &self.kind {
            DbErrorKind::Common(e) => e.backtrace(),
            DbErrorKind::Diesel(e) => e.backtrace(),
            _ => None,
        }
    }

    fn tags(&self) -> Vec<(&str, String)> {
        match &self.kind {
            DbErrorKind::Common(e) => e.tags(),
            DbErrorKind::Diesel(e) => e.tags(),
            _ => vec![],
        }
    }
}

impl InternalError for DbError {
    fn internal_error(message: String) -> Self {
        DbErrorKind::Common(SyncstorageDbError::internal(message)).into()
    }
}

impl_fmt_display!(DbError, DbErrorKind);

from_error!(SyncstorageDbError, DbError, DbErrorKind::Common);
from_error!(
    diesel::result::Error,
    DbError,
    |error: diesel::result::Error| DbError::from(DbErrorKind::Diesel(SqlError::from(error)))
);
from_error!(
    diesel::result::ConnectionError,
    DbError,
    |error: diesel::result::ConnectionError| DbError::from(DbErrorKind::Diesel(SqlError::from(
        error
    )))
);
from_error!(
    diesel_migrations::MigrationError,
    DbError,
    |error: diesel_migrations::MigrationError| DbError::from(DbErrorKind::Diesel(SqlError::from(
        error
    )))
);
from_error!(
    std::boxed::Box<dyn std::error::Error + std::marker::Send + Sync>,
    DbError,
    |error: std::boxed::Box<dyn std::error::Error>| DbError::internal_error(error.to_string())
);
