extern crate diesel;
extern crate diesel_migrations;

pub mod mock;
mod models;
mod pool;

use url::Url;

pub use models::TokenserverDb;
pub use pool::TokenserverPool;
use syncserver_common::Metrics;
pub use tokenserver_db_common::{params, results, Db, DbError, DbPool};
use tokenserver_settings::Settings;

pub fn pool_from_settings(
    settings: &Settings,
    metrics: &Metrics,
    use_test_transactions: bool,
) -> Result<Box<dyn DbPool>, DbError> {
    let url = Url::parse(&settings.database_url)
        .map_err(|e| DbError::internal(format!("Invalid SYNC_TOKENSERVER__DATABASE_URL: {e}")))?;
    Ok(match url.scheme() {
        "mysql" => Box::new(crate::pool::TokenserverPool::new(
            settings,
            metrics,
            use_test_transactions,
        )?),
        #[cfg(feature = "postgres")]
        "postgres" => Box::new(tokenserver_db_postgres::TokenserverPgPool::new(
            settings,
            metrics,
            use_test_transactions,
        )?),
        invalid_scheme => {
            return Err(DbError::internal(format!(
                "Invalid SYNC_TOKENSERVER__DATABASE_URL scheme: {invalid_scheme}://"
            )))
        }
    })
}
