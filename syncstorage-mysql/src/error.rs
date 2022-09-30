use std::fmt;

use backtrace::Backtrace;
use http::StatusCode;
use syncserver_common::{from_error, impl_fmt_display, ReportableError};
use syncserver_db_common::error::MysqlError;
use syncstorage_db_common::error::{DbErrorIntrospect, SyncstorageDbError};
use thiserror::Error;

/// An error type that represents any MySQL-related errors that may occur while processing a
/// syncstorage request. These errors may be application-specific or lower-level errors that arise
/// from the database backend.
#[derive(Debug)]
pub struct DbError {
    kind: DbErrorKind,
    pub status: StatusCode,
    pub backtrace: Backtrace,
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
}

#[derive(Debug, Error)]
enum DbErrorKind {
    #[error("{}", _0)]
    Common(SyncstorageDbError),

    #[error("{}", _0)]
    Mysql(MysqlError),
}

impl From<DbErrorKind> for DbError {
    fn from(kind: DbErrorKind) -> Self {
        match &kind {
            DbErrorKind::Common(dbe) => Self {
                status: dbe.status,
                backtrace: dbe.backtrace.clone(),
                kind,
            },
            _ => Self {
                kind,
                status: StatusCode::INTERNAL_SERVER_ERROR,
                backtrace: Backtrace::new(),
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

    fn error_backtrace(&self) -> String {
        format!("{:#?}", self.backtrace)
    }
}

impl_fmt_display!(DbError, DbErrorKind);

from_error!(SyncstorageDbError, DbError, DbErrorKind::Common);
from_error!(
    diesel::result::Error,
    DbError,
    |error: diesel::result::Error| DbError::from(DbErrorKind::Mysql(MysqlError::from(error)))
);
from_error!(
    diesel::result::ConnectionError,
    DbError,
    |error: diesel::result::ConnectionError| DbError::from(DbErrorKind::Mysql(MysqlError::from(
        error
    )))
);
from_error!(
    diesel::r2d2::PoolError,
    DbError,
    |error: diesel::r2d2::PoolError| DbError::from(DbErrorKind::Mysql(MysqlError::from(error)))
);
from_error!(
    diesel_migrations::RunMigrationsError,
    DbError,
    |error: diesel_migrations::RunMigrationsError| DbError::from(DbErrorKind::Mysql(
        MysqlError::from(error)
    ))
);
