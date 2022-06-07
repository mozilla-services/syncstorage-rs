use std::fmt;

use backtrace::Backtrace;
use http::StatusCode;
use syncserver_common::{from_error, impl_fmt_display};
use thiserror::Error;

// TODO: pull all these errors into a general SyncserverError
#[derive(Debug)]
pub struct CommonDbError {
    kind: CommonDbErrorKind,
    pub status: StatusCode,
    pub backtrace: Backtrace,
}

#[derive(Debug, Error)]
pub enum CommonDbErrorKind {
    #[error("Specified collection does not exist")]
    CollectionNotFound,

    #[error("Specified bso does not exist")]
    BsoNotFound,

    #[error("Specified batch does not exist")]
    BatchNotFound,

    #[error("An attempt at a conflicting write")]
    Conflict,

    #[error("Invalid database URL: {}", _0)]
    InvalidUrl(String),

    #[error("Unexpected error: {}", _0)]
    Internal(String),

    #[error("User over quota")]
    Quota,
}

impl CommonDbError {
    pub fn internal(msg: &str) -> Self {
        CommonDbErrorKind::Internal(msg.to_owned()).into()
    }
}

pub trait DbErrorIntrospect {
    fn is_sentry_event(&self) -> bool;
    fn metric_label(&self) -> Option<String>;
    fn is_collection_not_found(&self) -> bool;
    fn is_conflict(&self) -> bool;
    fn is_quota(&self) -> bool;
    fn is_bso_not_found(&self) -> bool;
    fn is_batch_not_found(&self) -> bool;
}

impl DbErrorIntrospect for CommonDbError {
    fn is_sentry_event(&self) -> bool {
        !matches!(&self.kind, CommonDbErrorKind::Conflict)
    }

    fn metric_label(&self) -> Option<String> {
        match &self.kind {
            CommonDbErrorKind::Conflict => Some("storage.conflict".to_owned()),
            _ => None,
        }
    }

    fn is_collection_not_found(&self) -> bool {
        matches!(self.kind, CommonDbErrorKind::CollectionNotFound)
    }

    fn is_conflict(&self) -> bool {
        matches!(self.kind, CommonDbErrorKind::Conflict)
    }

    fn is_quota(&self) -> bool {
        matches!(self.kind, CommonDbErrorKind::Quota)
    }

    fn is_bso_not_found(&self) -> bool {
        matches!(self.kind, CommonDbErrorKind::BsoNotFound)
    }

    fn is_batch_not_found(&self) -> bool {
        matches!(self.kind, CommonDbErrorKind::BatchNotFound)
    }
}

impl From<CommonDbErrorKind> for CommonDbError {
    fn from(kind: CommonDbErrorKind) -> Self {
        let status = match kind {
            CommonDbErrorKind::CollectionNotFound | CommonDbErrorKind::BsoNotFound => {
                StatusCode::NOT_FOUND
            }
            // Matching the Python code here (a 400 vs 404)
            CommonDbErrorKind::BatchNotFound => StatusCode::BAD_REQUEST,
            // NOTE: the protocol specification states that we should return a
            // "409 Conflict" response here, but clients currently do not
            // handle these respones very well:
            //  * desktop bug: https://bugzilla.mozilla.org/show_bug.cgi?id=959034
            //  * android bug: https://bugzilla.mozilla.org/show_bug.cgi?id=959032
            CommonDbErrorKind::Conflict => StatusCode::SERVICE_UNAVAILABLE,
            CommonDbErrorKind::Quota => StatusCode::FORBIDDEN,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        Self {
            kind,
            status,
            backtrace: Backtrace::new(),
        }
    }
}

impl_fmt_display!(CommonDbError, CommonDbErrorKind);

#[derive(Debug)]
pub struct MysqlError {
    kind: MysqlErrorKind,
    pub status: StatusCode,
    pub backtrace: Backtrace,
}

#[derive(Debug, Error)]
enum MysqlErrorKind {
    #[error("A database error occurred: {}", _0)]
    DieselQuery(#[from] diesel::result::Error),

    #[error("An error occurred while establishing a db connection: {}", _0)]
    DieselConnection(#[from] diesel::result::ConnectionError),

    #[error("A database pool error occurred: {}", _0)]
    Pool(diesel::r2d2::PoolError),

    #[error("Error migrating the database: {}", _0)]
    Migration(diesel_migrations::RunMigrationsError),
}

impl From<MysqlErrorKind> for MysqlError {
    fn from(kind: MysqlErrorKind) -> Self {
        Self {
            kind,
            status: StatusCode::INTERNAL_SERVER_ERROR,
            backtrace: Backtrace::new(),
        }
    }
}

impl_fmt_display!(MysqlError, MysqlErrorKind);

from_error!(
    diesel::result::Error,
    MysqlError,
    MysqlErrorKind::DieselQuery
);
from_error!(
    diesel::result::ConnectionError,
    MysqlError,
    MysqlErrorKind::DieselConnection
);
from_error!(diesel::r2d2::PoolError, MysqlError, MysqlErrorKind::Pool);
from_error!(
    diesel_migrations::RunMigrationsError,
    MysqlError,
    MysqlErrorKind::Migration
);
