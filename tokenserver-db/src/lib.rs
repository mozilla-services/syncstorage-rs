//! Database abstraction layer for Tokenserver.
//!
//! This crate provides a unified interface for interacting with Tokenserver databases,
//! supporting both MySQL and PostgreSQL backends. It includes:
//! - Database connection pool management
//! - Common database operations and traits
//! - Mock implementations for testing

/// Mock database implementations for testing.
pub mod mock;
#[cfg(test)]
mod tests;

use url::Url;

use syncserver_common::Metrics;
pub use tokenserver_db_common::{Db, DbError, DbPool, params, results};
use tokenserver_settings::Settings;

/// Creates a database connection pool from the provided settings.
///
/// This function examines the database URL scheme and returns the appropriate
/// database pool implementation (MySQL or PostgreSQL).
///
/// # Arguments
///
/// * `settings` - Tokenserver configuration settings
/// * `metrics` - Metrics collector for monitoring database operations
/// * `use_test_transactions` - If true, enables test transaction mode where
///   database changes are rolled back after each test
///
/// # Returns
///
/// Returns a boxed `DbPool` trait object on success, or a `DbError` if:
/// - The database URL is invalid
/// - The URL scheme is not supported (must be "mysql" or "postgres")
/// - Pool creation fails
///
/// # Examples
///
/// ```no_run
/// # use tokenserver_settings::Settings;
/// # use syncserver_common::Metrics;
/// # use tokenserver_db::pool_from_settings;
/// let settings = Settings::default();
/// let metrics = Metrics::noop();
/// let pool = pool_from_settings(&settings, &metrics, false).unwrap();
/// ```
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
            )));
        }
    })
}

#[cfg(not(any(feature = "mysql", feature = "postgres")))]
compile_error!("at least one of the \"mysql\" or \"postgres\" features must be enabled");
