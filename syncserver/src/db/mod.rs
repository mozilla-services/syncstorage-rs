//! Generic db abstration.

pub mod mock;
pub mod mysql;
pub mod spanner;
#[cfg(test)]
mod tests;
pub mod transaction;

use std::{env, sync::Arc, time::Duration};

use cadence::{Gauged, StatsdClient};
use syncserver_db_common::{
    error::{DbError, DbErrorKind},
    results, DbPool, GetPoolState, PoolState,
};
use syncstorage_settings::Settings;
use tokio::{self, time};
use url::Url;

use crate::server::{metrics::Metrics, BlockingThreadpool};

/// Create/initialize a pool of managed Db connections
pub async fn pool_from_settings(
    settings: &Settings,
    metrics: &Metrics,
    blocking_threadpool: Arc<BlockingThreadpool>,
) -> Result<Box<dyn DbPool>, DbError> {
    let url =
        Url::parse(&settings.database_url).map_err(|e| DbErrorKind::InvalidUrl(e.to_string()))?;
    Ok(match url.scheme() {
        "mysql" => Box::new(mysql::pool::MysqlDbPool::new(
            settings,
            metrics,
            blocking_threadpool,
        )?),
        "spanner" => Box::new(
            spanner::pool::SpannerDbPool::new(settings, metrics, blocking_threadpool).await?,
        ),
        _ => Err(DbErrorKind::InvalidUrl(settings.database_url.to_owned()))?,
    })
}

/// Emit DbPool metrics periodically
pub fn spawn_pool_periodic_reporter<T: GetPoolState + Send + 'static>(
    interval: Duration,
    metrics: StatsdClient,
    pool: T,
    blocking_threadpool: Arc<BlockingThreadpool>,
) -> Result<(), DbError> {
    let hostname = hostname::get()
        .expect("Couldn't get hostname")
        .into_string()
        .expect("Couldn't get hostname");
    let blocking_threadpool_size =
        str::parse::<u64>(&env::var("ACTIX_THREADPOOL").unwrap()).unwrap();
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

            let active_threads = blocking_threadpool.active_threads();
            let idle_threads = blocking_threadpool_size - active_threads;
            metrics
                .gauge_with_tags("blocking_threadpool.active", active_threads)
                .with_tag("hostname", &hostname)
                .send();
            metrics
                .gauge_with_tags("blocking_threadpool.idle", idle_threads)
                .with_tag("hostname", &hostname)
                .send();

            time::delay_for(interval).await;
        }
    });

    Ok(())
}
