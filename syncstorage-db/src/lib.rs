//! Generic db abstration.

#[cfg(test)]
#[macro_use]
extern crate slog_scope;

pub mod mock;
#[cfg(test)]
mod tests;

use std::time::Duration;

use cadence::{Gauged, StatsdClient};
use tokio::{self, time};

#[cfg(feature = "mysql")]
pub type DbPool = syncstorage_mysql::MysqlDbPool;
#[cfg(feature = "mysql")]
pub use syncstorage_mysql::DbError;
#[cfg(feature = "mysql")]
pub type Db = syncstorage_mysql::MysqlDb;

#[cfg(feature = "spanner")]
pub type DbPool = syncstorage_spanner::SpannerDbPool;
#[cfg(feature = "spanner")]
pub use syncstorage_spanner::DbError;
#[cfg(feature = "spanner")]
pub type Db = syncstorage_spanner::SpannerDb;

pub use syncserver_db_common::{GetPoolState, PoolState};
pub use syncstorage_db_common::error::DbErrorIntrospect;

pub use syncstorage_db_common::{
    params, results,
    util::{to_rfc3339, SyncTimestamp},
    DbPoolTrait, DbTrait, Sorting, UserIdentifier,
};

#[cfg(all(feature = "mysql", feature = "spanner"))]
compile_error!("only one of the \"mysql\" and \"spanner\" features can be enabled at a time");

#[cfg(not(any(feature = "mysql", feature = "spanner")))]
compile_error!("exactly one of the \"mysql\" and \"spanner\" features must be enabled");

/// Emit DbPool metrics periodically
pub fn spawn_pool_periodic_reporter<T: GetPoolState + Send + 'static>(
    interval: Duration,
    metrics: StatsdClient,
    pool: T,
) -> Result<(), DbError> {
    let hostname = hostname::get()
        .expect("Couldn't get hostname")
        .into_string()
        .expect("Couldn't get hostname");
    tokio::spawn(async move {
        loop {
            let PoolState {
                connections,
                idle_connections,
            } = pool.state();
            metrics
                .gauge_with_tags(
                    "storage.pool.connections.active",
                    (connections - idle_connections) as u64,
                )
                .with_tag("hostname", &hostname)
                .send();
            metrics
                .gauge_with_tags("storage.pool.connections.idle", idle_connections as u64)
                .with_tag("hostname", &hostname)
                .send();
            time::delay_for(interval).await;
        }
    });

    Ok(())
}
