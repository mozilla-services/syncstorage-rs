#[macro_use]
extern crate diesel;
extern crate diesel_migrations;
#[macro_use]
extern crate slog_scope;

mod db;
mod orm_models;
mod pool;
mod schema;

pub use db::PgDb;
pub use orm_models::{Batch, BatchBso, Bso, Collection, UserCollection};
pub use pool::PgDbPool;
pub use syncstorage_db_common::diesel::DbError;

pub(crate) type DbResult<T> = Result<T, DbError>;
