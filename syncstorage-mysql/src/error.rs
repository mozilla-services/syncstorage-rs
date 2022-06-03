use std::fmt;

use backtrace::Backtrace;
use http::StatusCode;
use syncstorage_common::{from_error, impl_fmt_display};
use syncstorage_db_common::error::{DbError as DbErrorCommon, DbErrorKind as DbErrorKindCommon};
use thiserror::Error;

#[derive(Debug)]
pub struct DbError {
    kind: DbErrorKind,
    pub status: StatusCode,
    pub backtrace: Backtrace,
}

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

    pub fn is_collection_not_found(&self) -> bool {
        matches!(&self.kind, DbErrorKind::Common(e) if e.is_collection_not_found())
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
