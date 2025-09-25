use std::fmt;

use backtrace::Backtrace;
use deadpool::managed::PoolError;
use http::StatusCode;
use syncserver_common::{from_error, impl_fmt_display, InternalError, ReportableError};
use syncserver_db_common::error::SqlError;
use thiserror::Error;
use tokenserver_common::TokenserverError;

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

    pub fn pool_timeout(timeout_type: deadpool::managed::TimeoutType) -> Self {
        DbErrorKind::PoolTimeout(timeout_type).into()
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
            DbErrorKind::PoolTimeout(_) => false,
            _ => true,
        }
    }

    fn metric_label(&self) -> Option<&str> {
        match &self.kind {
            DbErrorKind::Sql(e) => e.metric_label(),
            DbErrorKind::PoolTimeout(_) => Some("storage.pool.timeout"),
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

    #[error("A database pool timeout occurred, type: {:?}", _0)]
    PoolTimeout(deadpool::managed::TimeoutType),
}

impl From<DbErrorKind> for DbError {
    fn from(kind: DbErrorKind) -> Self {
        match kind {
            DbErrorKind::Sql(ref sqle) => Self {
                status: sqle.status,
                backtrace: Box::new(sqle.backtrace.clone()),
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
    diesel_migrations::MigrationError,
    DbError,
    |error: diesel_migrations::MigrationError| DbError::from(DbErrorKind::Sql(SqlError::from(
        error
    )))
);
from_error!(
    std::boxed::Box<dyn std::error::Error + std::marker::Send + Sync>,
    DbError,
    |error: std::boxed::Box<dyn std::error::Error>| DbError::internal_error(error.to_string())
);

impl From<PoolError<diesel_async::pooled_connection::PoolError>> for DbError {
    fn from(pe: PoolError<diesel_async::pooled_connection::PoolError>) -> DbError {
        match pe {
            PoolError::Backend(be) => match be {
                diesel_async::pooled_connection::PoolError::ConnectionError(ce) => ce.into(),
                diesel_async::pooled_connection::PoolError::QueryError(dbe) => dbe.into(),
            },
            PoolError::Timeout(timeout_type) => DbError::pool_timeout(timeout_type),
            _ => DbError::internal(format!("deadpool PoolError: {pe}")),
        }
    }
}
