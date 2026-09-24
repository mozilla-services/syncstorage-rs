#![allow(non_local_definitions)]
#[macro_use]
extern crate diesel;
extern crate diesel_migrations;
#[macro_use]
extern crate slog_scope;

#[macro_use]
mod db;
mod pool;
#[cfg(test)]
mod test;

pub use db::SqliteDb;
pub use pool::SqliteDbPool;
pub use syncstorage_db_common::diesel::DbError;

pub(crate) type DbResult<T> = Result<T, DbError>;
