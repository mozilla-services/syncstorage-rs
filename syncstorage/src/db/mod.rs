//! Generic db abstration.

pub mod mock;
pub mod mysql;
pub mod spanner;
#[cfg(test)]
mod tests;
pub mod transaction;

use std::time::Duration;

use actix_web::{error::BlockingError, web};
use cadence::{Gauged, StatsdClient};
use syncstorage_db_common::{
    error::{DbError, DbErrorKind},
    results, DbPool, GetPoolState, PoolState,
};
use tokio::{self, time};
use url::Url;

use crate::server::metrics::Metrics;
use crate::settings::Settings;

/// Create/initialize a pool of managed Db connections
pub async fn pool_from_settings(
    settings: &Settings,
    metrics: &Metrics,
) -> Result<Box<dyn DbPool>, DbError> {
    let url =
        Url::parse(&settings.database_url).map_err(|e| DbErrorKind::InvalidUrl(e.to_string()))?;
    Ok(match url.scheme() {
        "mysql" => Box::new(mysql::pool::MysqlDbPool::new(settings, metrics)?),
        "spanner" => Box::new(spanner::pool::SpannerDbPool::new(settings, metrics).await?),
        _ => Err(DbErrorKind::InvalidUrl(settings.database_url.to_owned()))?,
    })
}

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

/// Return the percentage and KB of system memory currently available.
pub fn mem_avail() -> (f64, u64) {
    use sysinfo::SystemExt;

    let mut sys = sysinfo::System::new();

    sys.refresh_all();

    let total = sys.total_memory();
    let used = sys.used_memory();
    let avail = total - used;
    let p_avail = (used as f64 / total as f64) * 100.0;

    (p_avail, avail)
}
