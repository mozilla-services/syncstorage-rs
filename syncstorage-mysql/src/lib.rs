#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;
#[macro_use]
extern crate slog_scope;

#[macro_use]
mod batch;
mod diesel_ext;
pub mod error;
pub mod models;
pub mod pool;
mod schema;
#[cfg(test)]
mod test;

pub use self::pool::MysqlDbPool;

pub type DbResult<T> = Result<T, error::DbError>;
