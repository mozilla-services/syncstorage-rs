use std::fmt;

use backtrace::Backtrace;
use http::StatusCode;
use syncserver_common::{from_error, impl_fmt_display, ReportableError};
use thiserror::Error;

/// Error specific to any MySQL database backend. These errors are not related to the syncstorage
/// or tokenserver application logic; rather, they are lower-level errors arising from diesel.
#[derive(Debug)]
pub struct SqlError {
    kind: SqlErrorKind,
    pub status: StatusCode,
    pub backtrace: Backtrace,
}

#[derive(Debug, Error)]
enum SqlErrorKind {
    #[error("A database error occurred: {}", _0)]
    DieselQuery(#[from] diesel::result::Error),

    #[error("An error occurred while establishing a db connection: {}", _0)]
    DieselConnection(#[from] diesel::result::ConnectionError),

    #[error("A database pool error occurred: {}", _0)]
    Pool(diesel::r2d2::PoolError),

    #[error("Error migrating the database: {}", _0)]
    Migration(diesel_migrations::RunMigrationsError),
}

impl From<SqlErrorKind> for SqlError {
    fn from(kind: SqlErrorKind) -> Self {
        Self {
            kind,
            status: StatusCode::INTERNAL_SERVER_ERROR,
            backtrace: Backtrace::new(),
        }
    }
}

impl ReportableError for SqlError {
    fn is_sentry_event(&self) -> bool {
        true
    }

    fn metric_label(&self) -> Option<String> {
        Some(
            match self.kind {
                SqlErrorKind::DieselQuery(_) => "diesel_query",
                SqlErrorKind::DieselConnection(_) => "diesel_connection",
                SqlErrorKind::Pool(_) => "pool",
                SqlErrorKind::Migration(_) => "migration",
            }
            .to_string(),
        )
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        Some(&self.backtrace)
    }
}

impl_fmt_display!(SqlError, SqlErrorKind);

from_error!(diesel::result::Error, SqlError, SqlErrorKind::DieselQuery);
from_error!(
    diesel::result::ConnectionError,
    SqlError,
    SqlErrorKind::DieselConnection
);
from_error!(diesel::r2d2::PoolError, SqlError, SqlErrorKind::Pool);
from_error!(
    diesel_migrations::RunMigrationsError,
    SqlError,
    SqlErrorKind::Migration
);
