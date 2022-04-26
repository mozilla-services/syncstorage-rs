use actix_web::web::block;
use async_trait::async_trait;
use diesel::{
    mysql::MysqlConnection,
    r2d2::{ConnectionManager, Pool},
};
use diesel_logger::LoggingConnection;
use std::time::Duration;

use super::models::{Db, DbResult, TokenserverDb};
use crate::db::{error::DbError, DbErrorKind, GetPoolState, PoolState};
use crate::diesel::Connection;
use crate::server::metrics::Metrics;
use crate::tokenserver::settings::Settings;

#[cfg(test)]
use crate::db::mysql::TestTransactionCustomizer;

embed_migrations!("src/tokenserver/migrations");

/// Run the diesel embedded migrations
///
/// Mysql DDL statements implicitly commit which could disrupt MysqlPool's
/// begin_test_transaction during tests. So this runs on its own separate conn.
pub fn run_embedded_migrations(database_url: &str) -> DbResult<()> {
    let conn = MysqlConnection::establish(database_url)?;

    embedded_migrations::run(&LoggingConnection::new(conn))?;

    Ok(())
}

#[derive(Clone)]
pub struct TokenserverPool {
    /// Pool of db connections
    inner: Pool<ConnectionManager<MysqlConnection>>,
    metrics: Metrics,
}

impl TokenserverPool {
    pub fn new(
        settings: &Settings,
        metrics: &Metrics,
        _use_test_transactions: bool,
    ) -> DbResult<Self> {
        if settings.run_migrations {
            run_embedded_migrations(&settings.database_url)?;
        }

        let manager = ConnectionManager::<MysqlConnection>::new(settings.database_url.clone());
        let builder = Pool::builder()
            .max_size(settings.database_pool_max_size.unwrap_or(10))
            .connection_timeout(Duration::from_secs(
                settings.database_pool_connection_timeout.unwrap_or(30) as u64,
            ))
            .min_idle(settings.database_pool_min_idle);

        #[cfg(test)]
        let builder = if _use_test_transactions {
            builder.connection_customizer(Box::new(TestTransactionCustomizer))
        } else {
            builder
        };

        Ok(Self {
            inner: builder.build(manager)?,
            metrics: metrics.clone(),
        })
    }

    pub fn get_sync(&self) -> Result<TokenserverDb, DbError> {
        let conn = self.inner.get().map_err(DbError::from)?;

        Ok(TokenserverDb::new(conn, &self.metrics))
    }

    #[cfg(test)]
    pub async fn get_tokenserver_db(&self) -> Result<TokenserverDb, DbError> {
        let pool = self.clone();
        let conn = block(move || pool.inner.get().map_err(DbError::from)).await?;

        Ok(TokenserverDb::new(conn, &self.metrics))
    }
}

impl From<actix_web::error::BlockingError<DbError>> for DbError {
    fn from(inner: actix_web::error::BlockingError<DbError>) -> Self {
        match inner {
            actix_web::error::BlockingError::Error(e) => e,
            actix_web::error::BlockingError::Canceled => {
                DbErrorKind::Internal("Db threadpool operation canceled".to_owned()).into()
            }
        }
    }
}

#[async_trait]
impl DbPool for TokenserverPool {
    async fn get(&self) -> Result<Box<dyn Db>, DbError> {
        let mut metrics = self.metrics.clone();
        metrics.start_timer("tokenserver.storage.get_pool", None);

        let pool = self.clone();
        let conn = block(move || pool.inner.get().map_err(DbError::from)).await?;

        Ok(Box::new(TokenserverDb::new(conn, &self.metrics)) as Box<dyn Db>)
    }

    fn box_clone(&self) -> Box<dyn DbPool> {
        Box::new(self.clone())
    }
}

#[async_trait]
pub trait DbPool: Sync + Send + GetPoolState {
    async fn get(&self) -> Result<Box<dyn Db>, DbError>;

    fn box_clone(&self) -> Box<dyn DbPool>;
}

impl GetPoolState for TokenserverPool {
    fn state(&self) -> PoolState {
        self.inner.state().into()
    }
}

impl GetPoolState for Box<dyn DbPool> {
    fn state(&self) -> PoolState {
        (**self).state()
    }
}

impl Clone for Box<dyn DbPool> {
    fn clone(&self) -> Box<dyn DbPool> {
        self.box_clone()
    }
}
