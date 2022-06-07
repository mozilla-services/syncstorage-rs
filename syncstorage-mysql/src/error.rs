use std::fmt;

use backtrace::Backtrace;
use http::StatusCode;
use syncserver_common::{from_error, impl_fmt_display};
use syncserver_db_common::error::{
    CommonDbError, CommonDbErrorKind, DbErrorIntrospect, MysqlError,
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
        DbErrorKind::Common(CommonDbErrorKind::BatchNotFound.into()).into()
    }

    pub fn bso_not_found() -> Self {
        DbErrorKind::Common(CommonDbErrorKind::BsoNotFound.into()).into()
    }

    pub fn collection_not_found() -> Self {
        DbErrorKind::Common(CommonDbErrorKind::CollectionNotFound.into()).into()
    }

    pub fn conflict() -> Self {
        DbErrorKind::Common(CommonDbErrorKind::Conflict.into()).into()
    }

    pub fn internal(msg: &str) -> Self {
        DbErrorKind::Common(CommonDbError::internal(msg)).into()
    }

    pub fn quota() -> Self {
        DbErrorKind::Common(CommonDbErrorKind::Quota.into()).into()
    }
}

#[derive(Debug, Error)]
pub enum DbErrorKind {
    #[error("{}", _0)]
    Common(CommonDbError),

    #[error("{}", _0)]
    Mysql(MysqlError),
}

impl From<DbErrorKind> for DbError {
    fn from(kind: DbErrorKind) -> Self {
        Self {
            status: match &kind {
                DbErrorKind::Common(dbe) => dbe.status,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            },
            kind,
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

from_error!(CommonDbError, DbError, DbErrorKind::Common);
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
