use diesel::{
    mysql::MysqlConnection,
    r2d2::{ConnectionManager, Pool},
    Connection,
};
#[cfg(test)]
use diesel_logger::LoggingConnection;
use std::time::Duration;

use super::models::{Db, DbResult, TokenserverDb};
use crate::db::error::DbError;
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

    #[cfg(test)]
    embedded_migrations::run(&LoggingConnection::new(conn))?;
    #[cfg(not(test))]
    embedded_migrations::run(&conn)?;

    Ok(())
}

#[derive(Clone)]
pub struct TokenserverPool {
    /// Pool of db connections
    inner: Pool<ConnectionManager<MysqlConnection>>,
}

impl TokenserverPool {
    pub fn new(settings: &Settings, _use_test_transactions: bool) -> DbResult<Self> {
        run_embedded_migrations(&settings.database_url)?;

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
        })
    }
}

impl DbPool for TokenserverPool {
    fn get(&self) -> Result<Box<dyn Db>, DbError> {
        self.inner
            .get()
            .map(|db_pool| Box::new(TokenserverDb::new(db_pool)) as Box<dyn Db>)
            .map_err(DbError::from)
    }

    fn box_clone(&self) -> Box<dyn DbPool> {
        Box::new(self.clone())
    }
}

pub trait DbPool: Sync + Send {
    fn get(&self) -> Result<Box<dyn Db>, DbError>;

    fn box_clone(&self) -> Box<dyn DbPool>;
}

impl Clone for Box<dyn DbPool> {
    fn clone(&self) -> Box<dyn DbPool> {
        self.box_clone()
    }
}
