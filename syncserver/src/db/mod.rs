//! Generic db abstration.

pub mod mock;
#[cfg(test)]
mod tests;
pub mod transaction;

use std::time::Duration;

use cadence::{Gauged, StatsdClient};
use syncserver_db_common::{GetPoolState, PoolState};
use syncstorage_db_common::{results, Db as DbTrait, DbPool as DbPoolTrait};
#[cfg(feature = "mysql")]
use syncstorage_mysql::pool::MysqlDbPool;
#[cfg(feature = "spanner")]
use syncstorage_spanner::pool::SpannerDbPool;
use tokio::{self, time};

#[cfg(feature = "mysql")]
pub type DbPool = MysqlDbPool;
#[cfg(feature = "mysql")]
pub use syncstorage_mysql::error::DbError;
#[cfg(feature = "mysql")]
pub type Db = syncstorage_mysql::models::MysqlDb;

#[cfg(feature = "spanner")]
pub type DbPool = SpannerDbPool;
#[cfg(feature = "spanner")]
pub use syncstorage_spanner::error::DbError;
#[cfg(feature = "spanner")]
pub type Db = syncstorage_spanner::models::SpannerDb;

pub type BoxDb = Box<dyn DbTrait<Error = DbError>>;
pub type BoxDbPool = Box<dyn DbPoolTrait<Error = DbError>>;

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
