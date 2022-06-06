use std::fmt;

use backtrace::Backtrace;
use http::StatusCode;
use syncserver_common::{from_error, impl_fmt_display};
use syncserver_db_common::error::{
    DbError as DbErrorCommon, DbErrorIntrospect, DbErrorKind as DbErrorKindCommon,
};
use thiserror::Error;

#[derive(Debug)]
pub struct DbError {
    kind: DbErrorKind,
    pub status: StatusCode,
    pub backtrace: Backtrace,
}

// TODO: these helpers shouldn't be duplicated across error types -- how can we share them?
impl DbError {
    pub fn batch_not_found() -> Self {
        DbErrorKind::Common(DbErrorKindCommon::BatchNotFound.into()).into()
    }

    pub fn bso_not_found() -> Self {
        DbErrorKind::Common(DbErrorKindCommon::BsoNotFound.into()).into()
    }

    pub fn collection_not_found() -> Self {
        DbErrorKind::Common(DbErrorKindCommon::CollectionNotFound.into()).into()
    }

    pub fn conflict() -> Self {
        DbErrorKind::Common(DbErrorKindCommon::Conflict.into()).into()
    }

    pub fn internal(msg: &str) -> Self {
        DbErrorKind::Common(DbErrorCommon::internal(msg)).into()
    }

    pub fn quota() -> Self {
        DbErrorKind::Common(DbErrorKindCommon::Quota.into()).into()
    }
}

#[derive(Debug, Error)]
pub enum DbErrorKind {
    #[error("{}", _0)]
    Common(DbErrorCommon),

    #[error("A database error occurred: {}", _0)]
    DieselQuery(#[from] diesel::result::Error),

    #[error("An error occurred while establishing a db connection: {}", _0)]
    DieselConnection(#[from] diesel::result::ConnectionError),

    #[error("A database pool error occurred: {}", _0)]
    Pool(diesel::r2d2::PoolError),

    #[error("Error migrating the database: {}", _0)]
    Migration(diesel_migrations::RunMigrationsError),
}

impl From<DbErrorKind> for DbError {
    fn from(kind: DbErrorKind) -> Self {
        Self {
            kind,
            status: StatusCode::INTERNAL_SERVER_ERROR,
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

from_error!(diesel::result::Error, DbError, DbErrorKind::DieselQuery);
from_error!(
    diesel::result::ConnectionError,
    DbError,
    DbErrorKind::DieselConnection
);
from_error!(diesel::r2d2::PoolError, DbError, DbErrorKind::Pool);
from_error!(
    diesel_migrations::RunMigrationsError,
    DbError,
    DbErrorKind::Migration
);
from_error!(DbErrorCommon, DbError, DbErrorKind::Common);
