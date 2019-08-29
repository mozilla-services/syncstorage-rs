use std::fmt;

use actix_web::http::StatusCode;
use diesel;
use diesel_migrations;
use failure::{Backtrace, Context, Fail};

#[derive(Debug)]
pub struct DbError {
    inner: Context<DbErrorKind>,
    pub status: StatusCode,
}

#[derive(Debug, Fail)]
pub enum DbErrorKind {
    #[fail(display = "A database error occurred: {}", _0)]
    Query(#[cause] diesel::result::Error),

    #[fail(
        display = "An error occurred while establishing a db connection: {}",
        _0
    )]
    Connection(ConnectionError),

    #[fail(display = "A database pool error occurred: {}", _0)]
    Pool(diesel::r2d2::PoolError),

    #[fail(display = "Error migrating the database: {}", _0)]
    Migration(diesel_migrations::RunMigrationsError),

    #[fail(display = "Specified collection does not exist")]
    CollectionNotFound,

    #[fail(display = "Specified bso does not exist")]
    BsoNotFound,

    #[fail(display = "Specified batch does not exist")]
    BatchNotFound,

    #[fail(display = "An attempt at a conflicting write")]
    Conflict,

    #[fail(display = "Database integrity error: {}", _0)]
    Integrity(String),

    #[fail(display = "Invalid SYNC_DATABASE_URL: {}", _0)]
    InvalidUrl(String),

    #[fail(display = "Unexpected error: {}", _0)]
    Internal(String),
}

impl DbError {
    pub fn kind(&self) -> &DbErrorKind {
        self.inner.get_context()
    }

    pub fn internal(msg: &str) -> Self {
        DbErrorKind::Internal(msg.to_owned()).into()
    }
}

impl From<Context<DbErrorKind>> for DbError {
    fn from(inner: Context<DbErrorKind>) -> Self {
        let status = match inner.get_context() {
            DbErrorKind::CollectionNotFound | DbErrorKind::BsoNotFound => StatusCode::NOT_FOUND,
            // Matching the Python code here (a 400 vs 404)
            DbErrorKind::BatchNotFound => StatusCode::BAD_REQUEST,
            // NOTE: the protocol specification states that we should return a
            // "409 Conflict" response here, but clients currently do not
            // handle these respones very well:
            //  * desktop bug: https://bugzilla.mozilla.org/show_bug.cgi?id=959034
            //  * android bug: https://bugzilla.mozilla.org/show_bug.cgi?id=959032
            DbErrorKind::Conflict => StatusCode::SERVICE_UNAVAILABLE,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let error = Self { inner, status };

        if status == StatusCode::INTERNAL_SERVER_ERROR {
            sentry::integrations::failure::capture_fail(&error);
        }

        error
    }
}

// XXX: maybe not worth the effort vs
// DbErrorKind::{DieselConnectionError, SpannerConnectionError}
#[derive(Debug)]
pub enum ConnectionError {
    Diesel(diesel::result::ConnectionError),
    Spanner(String),
    #[cfg(feature = "google_grpc")]
    SpannerGrpc(grpcio::Error),
}

impl fmt::Display for ConnectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConnectionError::Diesel(e) => fmt::Display::fmt(e, formatter),
            ConnectionError::Spanner(msg) => fmt::Display::fmt(msg, formatter),
            #[cfg(feature = "google_grpc")]
            ConnectionError::SpannerGrpc(e) => fmt::Display::fmt(e, formatter),
        }
    }
}

failure_boilerplate!(DbError, DbErrorKind);

from_error!(diesel::result::Error, DbError, DbErrorKind::Query);
from_error!(diesel::result::ConnectionError, DbError, |inner| {
    DbErrorKind::Connection(ConnectionError::Diesel(inner))
});
from_error!(
    google_spanner1::Error,
    DbError,
    |inner: google_spanner1::Error| DbErrorKind::Connection(ConnectionError::Spanner(
        inner.to_string()
    ))
);
#[cfg(feature = "google_grpc")]
from_error!(grpcio::Error, DbError, |inner: grpcio::Error| {
    DbErrorKind::Connection(ConnectionError::SpannerGrpc(inner))
});
from_error!(diesel::r2d2::PoolError, DbError, DbErrorKind::Pool);
from_error!(
    diesel_migrations::RunMigrationsError,
    DbError,
    DbErrorKind::Migration
);
