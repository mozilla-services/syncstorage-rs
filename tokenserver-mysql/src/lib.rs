extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

pub mod mock;
pub mod models;
pub mod params;
pub mod pool;
pub mod results;

use syncstorage_mysql::error::DbError;

pub(crate) type DbFuture<'a, T> = syncserver_db_common::DbFuture<'a, T, DbError>;
pub(crate) type DbResult<T> = Result<T, DbError>;
