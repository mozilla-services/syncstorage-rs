#[macro_use]
extern crate diesel;
#[macro_use]
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

pub use syncstorage_sql_db_common::error::DbError;
pub use models::MysqlDb;
pub use pool::MysqlDbPool;

pub(crate) type DbResult<T> = Result<T, DbError>;
