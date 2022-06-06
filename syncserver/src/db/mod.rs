//! Generic db abstration.

pub mod mock;
#[cfg(test)]
mod tests;
pub mod transaction;

use std::time::Duration;

use actix_web::{error::BlockingError, web};
use cadence::{Gauged, StatsdClient};
use syncserver_db_common::{results, GetPoolState, PoolState};
#[cfg(feature = "mysql")]
use syncstorage_mysql::pool::MysqlDbPool;
#[cfg(feature = "spanner")]
use syncstorage_spanner::pool::SpannerDbPool;
use tokio::{self, time};

// TODO: can probably clean this up by creating a submodule and applying preprocessor command to that
// TODO: pub use * that submodule to include it here
#[cfg(feature = "mysql")]
pub type DbPool = MysqlDbPool;
#[cfg(feature = "mysql")]
pub use syncstorage_mysql::error::DbError;
#[cfg(feature = "mysql")]
pub use syncstorage_mysql::error::DbErrorKind;
#[cfg(feature = "mysql")]
pub type Db = syncstorage_mysql::models::MysqlDb;

#[cfg(feature = "spanner")]
pub type DbPool = SpannerDbPool;
#[cfg(feature = "spanner")]
pub use syncstorage_spanner::error::DbError;
#[cfg(feature = "spanner")]
pub use syncstorage_spanner::error::DbErrorKind;
#[cfg(feature = "spanner")]
pub type Db = syncstorage_spanner::models::SpannerDb;

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

pub async fn run_on_blocking_threadpool<F, T>(f: F) -> Result<T, DbError>
where
    F: FnOnce() -> Result<T, DbError> + Send + 'static,
    T: Send + 'static,
{
    web::block(f).await.map_err(|e| match e {
        BlockingError::Error(e) => e,
        BlockingError::Canceled => DbError::internal("Db threadpool operation canceled"),
    })
}
