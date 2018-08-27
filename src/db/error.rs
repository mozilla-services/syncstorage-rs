use std::fmt;

use diesel;
use diesel_migrations;
use failure::{Backtrace, Context, Fail};

#[derive(Debug)]
pub struct DbError {
    inner: Context<DbErrorKind>,
}

#[derive(Debug, Fail)]
pub enum DbErrorKind {
    #[fail(display = "A database error occurred")]
    Query(#[cause] diesel::result::Error),

    #[fail(display = "A database pool error occurred")]
    Pool(diesel::r2d2::PoolError),

    #[fail(display = "Error migrating the database")]
    Migration(diesel_migrations::RunMigrationsError),
}

impl DbError {
    pub fn kind(&self) -> &DbErrorKind {
        self.inner.get_context()
    }
}

impl Fail for DbError {
    fn cause(&self) -> Option<&Fail> {
        self.inner.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.inner.backtrace()
    }
}

impl fmt::Display for DbError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.inner, f)
    }
}

impl From<DbErrorKind> for DbError {
    fn from(kind: DbErrorKind) -> DbError {
        Context::new(kind).into()
    }
}

impl From<Context<DbErrorKind>> for DbError {
    fn from(inner: Context<DbErrorKind>) -> DbError {
        DbError { inner }
    }
}

impl From<diesel::result::Error> for DbError {
    fn from(inner: diesel::result::Error) -> DbError {
        DbErrorKind::Query(inner).into()
    }
}

impl From<diesel::r2d2::PoolError> for DbError {
    fn from(inner: diesel::r2d2::PoolError) -> DbError {
        DbErrorKind::Pool(inner).into()
    }
}

impl From<diesel_migrations::RunMigrationsError> for DbError {
    fn from(inner: diesel_migrations::RunMigrationsError) -> DbError {
        DbErrorKind::Migration(inner).into()
    }
}
