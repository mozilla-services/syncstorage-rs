//! Generic db abstration.

pub mod mock;
#[cfg(feature = "mysql")]
pub mod mysql;
#[cfg(feature = "spanner")]
pub mod spanner;
#[cfg(feature = "sqlite")]
pub mod sqlite;
#[cfg(test)]
mod tests;
pub mod transaction;

use std::sync::Arc;

use syncserver_db_common::{
    error::{DbError, DbErrorKind},
    results, DbPool,
};
use syncstorage_settings::Settings;
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
        #[cfg(feature = "mysql")]
        "mysql" => Box::new(mysql::pool::MysqlDbPool::new(
            settings,
            metrics,
            blocking_threadpool,
        )?),
        #[cfg(feature = "spanner")]
        "spanner" => Box::new(
            spanner::pool::SpannerDbPool::new(settings, metrics, blocking_threadpool).await?,
        ),
        #[cfg(feature = "sqlite")]
        "sqlite" => Box::new(sqlite::pool::SqliteDbPool::new(
            settings,
            metrics,
            blocking_threadpool,
        )?),
        _ => Err(DbErrorKind::InvalidUrl(settings.database_url.to_owned()))?,
    })
}
