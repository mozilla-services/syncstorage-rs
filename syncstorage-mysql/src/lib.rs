#![allow(non_local_definitions)]
#[macro_use]
extern crate diesel;
extern crate diesel_migrations;
#[macro_use]
extern crate slog_scope;

#[macro_use]
mod batch;
mod diesel_ext;
mod models;
mod pool;
mod schema;
#[cfg(test)]
mod test;

pub use models::MysqlDb;
pub use pool::MysqlDbPool;
pub use syncstorage_db_common::diesel::DbError;

pub(crate) type DbResult<T> = Result<T, DbError>;
