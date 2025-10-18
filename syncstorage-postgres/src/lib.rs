#![allow(non_local_definitions)]
#![allow(unused_imports)] // XXX:
#[macro_use]
extern crate diesel;
extern crate diesel_migrations;
#[macro_use]
extern crate slog_scope;

mod db;
mod pool;

pub use db::PgDb;
pub use pool::PgDbPool;
pub use syncstorage_db_common::diesel::DbError;

pub(crate) type DbResult<T> = Result<T, DbError>;
