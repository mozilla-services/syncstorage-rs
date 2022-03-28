use std::fmt;

use actix_web::http::StatusCode;
use thiserror::Error;

use crate::error::{ApiError, ApiErrorKind};

#[derive(Error, Debug)]
pub struct DbError {
    kind: DbErrorKind,
    pub status: StatusCode,
}

#[derive(Debug, Error)]
pub enum DbErrorKind {
    #[error("A database error occurred: {}", _0)]
    DieselQuery(#[from] diesel::result::Error),

    #[error("An error occurred while establishing a db connection: {}", _0)]
    DieselConnection(#[from] diesel::result::ConnectionError),

    #[error("A database error occurred: {}", _0)]
    SpannerGrpc(#[from] grpcio::Error),

    #[error("Spanner data load too large: {}", _0)]
    SpannerTooLarge(String),

    #[error("A database pool error occurred: {}", _0)]
    Pool(diesel::r2d2::PoolError),

    #[error("Error migrating the database: {}", _0)]
    Migration(diesel_migrations::RunMigrationsError),

    #[error("Specified collection does not exist")]
    CollectionNotFound,

    #[error("Specified bso does not exist")]
    BsoNotFound,

    #[error("Specified batch does not exist")]
    BatchNotFound,

    #[error("Tokenserver user retired")]
    TokenserverUserRetired,

    #[error("An attempt at a conflicting write")]
    Conflict,

    #[error("Database integrity error: {}", _0)]
    Integrity(String),

    #[error("Invalid SYNC_DATABASE_URL: {}", _0)]
    InvalidUrl(String),

    #[error("Unexpected error: {}", _0)]
    Internal(String),

    #[error("User over quota")]
    Quota,

    #[error("Connection expired")]
    Expired,
}

impl DbError {
    pub fn kind(&self) -> &DbErrorKind {
        &self.kind
    }

    pub fn internal(msg: &str) -> Self {
        DbErrorKind::Internal(msg.to_owned()).into()
    }

    pub fn is_reportable(&self) -> bool {
        !matches!(&self.kind, DbErrorKind::Conflict)
    }

    pub fn metric_label(&self) -> Option<String> {
        match &self.kind {
            DbErrorKind::Conflict => Some("storage.conflict".to_owned()),
            _ => None,
        }
    }
}

impl From<DbErrorKind> for DbError {
    fn from(kind: DbErrorKind) -> Self {
        let status = match kind {
            DbErrorKind::CollectionNotFound | DbErrorKind::BsoNotFound => StatusCode::NOT_FOUND,
            // Matching the Python code here (a 400 vs 404)
            DbErrorKind::BatchNotFound | DbErrorKind::SpannerTooLarge(_) => StatusCode::BAD_REQUEST,
            // NOTE: the protocol specification states that we should return a
            // "409 Conflict" response here, but clients currently do not
            // handle these respones very well:
            //  * desktop bug: https://bugzilla.mozilla.org/show_bug.cgi?id=959034
            //  * android bug: https://bugzilla.mozilla.org/show_bug.cgi?id=959032
            DbErrorKind::Conflict => StatusCode::SERVICE_UNAVAILABLE,
            DbErrorKind::Quota => StatusCode::FORBIDDEN,
            // NOTE: TokenserverUserRetired is an internal service error for compatibility reasons
            // (the legacy Tokenserver returned an internal service error in this situation)
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        Self { kind, status }
    }
}

impl_fmt_display!(DbError, DbErrorKind);

impl From<actix_web::error::BlockingError<DbError>> for DbError {
    fn from(inner: actix_web::error::BlockingError<DbError>) -> Self {
        match inner {
            actix_web::error::BlockingError::Error(e) => e,
            actix_web::error::BlockingError::Canceled => {
                DbErrorKind::Internal("Db threadpool operation canceled".to_owned()).into()
            }
        }
    }
}

from_error!(DbErrorKind, ApiError, |inner: DbErrorKind| {
    ApiErrorKind::Db(DbError::from(inner))
});
from_error!(diesel::result::Error, DbError, DbErrorKind::DieselQuery);
from_error!(
    diesel::result::Error,
    ApiError,
    |inner: diesel::result::Error| { ApiErrorKind::Db(DbError::from(inner)) }
);
from_error!(
    diesel::result::ConnectionError,
    DbError,
    DbErrorKind::DieselConnection
);
from_error!(
    diesel::result::ConnectionError,
    ApiError,
    |inner: diesel::result::ConnectionError| { ApiErrorKind::Db(DbError::from(inner)) }
);
from_error!(grpcio::Error, DbError, |inner: grpcio::Error| {
    // Convert ABORTED (typically due to a transaction abort) into 503s
    match inner {
        grpcio::Error::RpcFailure(ref status) | grpcio::Error::RpcFinished(Some(ref status))
            if status.code() == grpcio::RpcStatusCode::ABORTED =>
        {
            DbErrorKind::Conflict
        }
        _ => DbErrorKind::SpannerGrpc(inner),
    }
});
from_error!(grpcio::Error, ApiError, |inner: grpcio::Error| {
    ApiErrorKind::Db(DbError::from(inner))
});
from_error!(diesel::r2d2::PoolError, DbError, DbErrorKind::Pool);
from_error!(
    diesel::r2d2::PoolError,
    ApiError,
    |inner: diesel::r2d2::PoolError| { ApiErrorKind::Db(DbError::from(inner)) }
);
from_error!(
    diesel_migrations::RunMigrationsError,
    DbError,
    DbErrorKind::Migration
);
from_error!(
    diesel_migrations::RunMigrationsError,
    ApiError,
    |inner: diesel_migrations::RunMigrationsError| { ApiErrorKind::Db(DbError::from(inner)) }
);
