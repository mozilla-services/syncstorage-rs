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

    #[fail(display = "A database pool error occurred: {}", _0)]
    Pool(diesel::r2d2::PoolError),

    #[fail(display = "Error migrating the database: {}", _0)]
    Migration(diesel_migrations::RunMigrationsError),

    #[fail(display = "Specified collection does not exist")]
    CollectionNotFound,

    #[fail(display = "Specified item does not exist")]
    ItemNotFound,

    #[fail(display = "An attempt at a conflicting write")]
    Conflict,

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
            DbErrorKind::CollectionNotFound => StatusCode::BAD_REQUEST,
            DbErrorKind::ItemNotFound => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        Self { inner, status }
    }
}

failure_boilerplate!(DbError, DbErrorKind);

from_error!(diesel::result::Error, DbError, DbErrorKind::Query);
from_error!(diesel::r2d2::PoolError, DbError, DbErrorKind::Pool);
from_error!(
    diesel_migrations::RunMigrationsError,
    DbError,
    DbErrorKind::Migration
);
