pub mod mock;
#[cfg(test)]
mod tests;

use url::Url;

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
        #[cfg(feature = "mysql")]
        "mysql" => Box::new(tokenserver_mysql::TokenserverPool::new(
            settings,
            metrics,
            use_test_transactions,
        )?),
        #[cfg(feature = "postgres")]
        "postgres" => Box::new(tokenserver_postgres::TokenserverPgPool::new(
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

#[cfg(not(any(feature = "mysql", feature = "postgres")))]
compile_error!("at least one of the \"mysql\" or \"postgres\" features must be enabled");
