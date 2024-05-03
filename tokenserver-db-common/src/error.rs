use std::fmt;

use backtrace::Backtrace;
use http::StatusCode;
use syncserver_common::{from_error, impl_fmt_display, InternalError};
use syncserver_db_common::error::SqlError;
use thiserror::Error;
use tokenserver_common::TokenserverError;

pub type DbFuture<'a, T> = syncserver_db_common::DbFuture<'a, T, DbError>;
pub type DbResult<T> = Result<T, DbError>;

/// An error type that represents any database-related errors that may occur while processing a
/// tokenserver request.
#[derive(Debug)]
pub struct DbError {
    kind: DbErrorKind,
    pub status: StatusCode,
    pub backtrace: Box<Backtrace>,
}

impl DbError {
    pub fn internal(msg: String) -> Self {
        DbErrorKind::Internal(msg).into()
    }
}

#[derive(Debug, Error)]
enum DbErrorKind {
    #[error("{}", _0)]
    SqlError(SqlError),

    #[error("Unexpected error: {}", _0)]
    Internal(String),
}

impl From<DbErrorKind> for DbError {
    fn from(kind: DbErrorKind) -> Self {
        match kind {
            DbErrorKind::SqlError(ref sql_error) => Self {
                status: sql_error.status,
                backtrace: Box::new(sql_error.backtrace.clone()),
                kind,
            },
            DbErrorKind::Internal(_) => Self {
                kind,
                status: StatusCode::INTERNAL_SERVER_ERROR,
                backtrace: Box::new(Backtrace::new()),
            },
        }
    }
}

impl From<DbError> for TokenserverError {
    fn from(db_error: DbError) -> Self {
        TokenserverError {
            description: db_error.to_string(),
            context: db_error.to_string(),
            backtrace: db_error.backtrace,
            http_status: if db_error.status.is_server_error() {
                // Use the status code from the DbError if it already suggests an internal error;
                // it might be more specific than `StatusCode::SERVICE_UNAVAILABLE`
                db_error.status
            } else {
                StatusCode::SERVICE_UNAVAILABLE
            },
            // An unhandled DbError in the Tokenserver code is an internal error
            ..TokenserverError::internal_error()
        }
    }
}

impl InternalError for DbError {
    fn internal_error(message: String) -> Self {
        DbErrorKind::Internal(message).into()
    }
}

impl_fmt_display!(DbError, DbErrorKind);

from_error!(
    diesel::result::Error,
    DbError,
    |error: diesel::result::Error| DbError::from(DbErrorKind::SqlError(SqlError::from(error)))
);
from_error!(
    diesel::result::ConnectionError,
    DbError,
    |error: diesel::result::ConnectionError| DbError::from(DbErrorKind::SqlError(SqlError::from(
        error
    )))
);
from_error!(
    diesel::r2d2::PoolError,
    DbError,
    |error: diesel::r2d2::PoolError| DbError::from(DbErrorKind::SqlError(SqlError::from(error)))
);
from_error!(
    diesel_migrations::RunMigrationsError,
    DbError,
    |error: diesel_migrations::RunMigrationsError| DbError::from(DbErrorKind::SqlError(
        SqlError::from(error)
    ))
);
