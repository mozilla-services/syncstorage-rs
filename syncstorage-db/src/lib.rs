//! Generic db abstration.

#[cfg(test)]
#[macro_use]
extern crate slog_scope;

pub mod mock;
#[cfg(test)]
mod tests;

#[cfg(feature = "mysql")]
pub type DbPoolImpl = syncstorage_mysql::MysqlDbPool;
#[cfg(feature = "mysql")]
pub use syncstorage_mysql::DbError;
#[cfg(feature = "mysql")]
pub type DbImpl = syncstorage_mysql::MysqlDb;

#[cfg(feature = "sqlite")]
pub type DbPoolImpl = syncstorage_sqlite::SqliteDbPool;
#[cfg(feature = "sqlite")]
pub use syncstorage_sqlite::DbError;
#[cfg(feature = "sqlite")]
pub type DbImpl = syncstorage_sqlite::SqliteDb;

#[cfg(feature = "spanner")]
pub type DbPoolImpl = syncstorage_spanner::SpannerDbPool;
#[cfg(feature = "spanner")]
pub use syncstorage_spanner::DbError;
#[cfg(feature = "spanner")]
pub type DbImpl = syncstorage_spanner::SpannerDb;

pub use syncserver_db_common::{GetPoolState, PoolState};
pub use syncstorage_db_common::error::DbErrorIntrospect;

pub use syncstorage_db_common::{
    params, results,
    util::{to_rfc3339, SyncTimestamp},
    Db, DbPool, Sorting, UserIdentifier,
};

#[cfg(all(feature = "mysql", feature = "spanner", feature = "sqlite"))]
compile_error!(
    "only one of the \"mysql\", \"spanner\" and \"sqlite\" features can be enabled at a time"
);

#[cfg(not(any(feature = "mysql", feature = "spanner", feature = "sqlite")))]
compile_error!("exactly one of the \"mysql\", \"spanner\" and \"sqlite\" features must be enabled");
