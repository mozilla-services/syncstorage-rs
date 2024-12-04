use std::fmt;

use backtrace::Backtrace;
use http::StatusCode;
use syncserver_common::{from_error, impl_fmt_display, InternalError, ReportableError};
use syncserver_db_common::error::SqlError;
use thiserror::Error;
use tokenserver_common::TokenserverError;

pub(crate) type DbFuture<'a, T> = syncserver_db_common::DbFuture<'a, T, DbError>;
pub(crate) type DbResult<T> = Result<T, DbError>;

/// An error type that represents any database-related errors that may occur while processing a
/// tokenserver request.
#[derive(Debug)]
pub struct DbError {
    kind: DbErrorKind,
    pub status: StatusCode,
    pub backtrace: Box<Backtrace>,
}

impl DbError {
    pub(crate) fn internal(msg: String) -> Self {
        DbErrorKind::Internal(msg).into()
    }
}

impl ReportableError for DbError {
    fn backtrace(&self) -> Option<&Backtrace> {
        match &self.kind {
            DbErrorKind::Sql(e) => e.backtrace(),
            _ => Some(&self.backtrace),
        }
    }

    fn is_sentry_event(&self) -> bool {
        match &self.kind {
            DbErrorKind::Sql(e) => e.is_sentry_event(),
            _ => true,
        }
    }

    fn metric_label(&self) -> Option<String> {
        match &self.kind {
            DbErrorKind::Sql(e) => e.metric_label(),
            _ => None,
        }
    }
}

#[derive(Debug, Error)]
enum DbErrorKind {
    #[error("{}", _0)]
    Sql(SqlError),

    #[error("Unexpected error: {}", _0)]
    Internal(String),
}

impl From<DbErrorKind> for DbError {
    fn from(kind: DbErrorKind) -> Self {
        match kind {
            DbErrorKind::Sql(ref mysql_error) => Self {
                status: mysql_error.status,
                backtrace: Box::new(mysql_error.backtrace.clone()),
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
            backtrace: db_error.backtrace.clone(),
            http_status: if db_error.status.is_server_error() {
                // Use the status code from the DbError if it already suggests an internal error;
                // it might be more specific than `StatusCode::SERVICE_UNAVAILABLE`
                db_error.status
            } else {
                StatusCode::SERVICE_UNAVAILABLE
            },
            source: Some(Box::new(db_error)),
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
    |error: diesel::result::Error| DbError::from(DbErrorKind::Sql(SqlError::from(error)))
);
from_error!(
    diesel::result::ConnectionError,
    DbError,
    |error: diesel::result::ConnectionError| DbError::from(DbErrorKind::Sql(SqlError::from(error)))
);
from_error!(
    diesel::r2d2::PoolError,
    DbError,
    |error: diesel::r2d2::PoolError| DbError::from(DbErrorKind::Sql(SqlError::from(error)))
);
from_error!(
    diesel_migrations::RunMigrationsError,
    DbError,
    |error: diesel_migrations::RunMigrationsError| DbError::from(DbErrorKind::Sql(SqlError::from(
        error
    )))
);
