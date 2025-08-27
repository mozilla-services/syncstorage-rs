use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use diesel::{
    mysql::MysqlConnection,
    r2d2::{ConnectionManager, Pool},
    Connection,
};
use diesel_logger::LoggingConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use syncserver_common::{BlockingThreadpool, Metrics};
#[cfg(debug_assertions)]
use syncserver_db_common::test::TestTransactionCustomizer;
use syncserver_db_common::{GetPoolState, PoolState};
use tokenserver_settings::Settings;

use super::{
    error::{DbError, DbResult},
    models::{Db, TokenserverDb},
};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

use diesel_async::pooled_connection::AsyncDieselConnectionManager;
pub(crate) type Conn =
    deadpool::managed::Object<AsyncDieselConnectionManager<diesel_async::AsyncMysqlConnection>>;

/// Run the diesel embedded migrations
///
/// Mysql DDL statements implicitly commit which could disrupt MysqlPool's
/// begin_test_transaction during tests. So this runs on its own separate conn.
fn run_embedded_migrations(database_url: &str) -> DbResult<()> {
    let conn = MysqlConnection::establish(database_url)?;

    LoggingConnection::new(conn).run_pending_migrations(MIGRATIONS)?;

    Ok(())
}

#[derive(Clone)]
pub struct TokenserverPool {
    /// Pool of db connections
    //inner: Pool<ConnectionManager<MysqlConnection>>,
    inner: diesel_async::pooled_connection::deadpool::Pool<diesel_async::AsyncMysqlConnection>,
    metrics: Metrics,
    // This field is public so the service ID can be set after the pool is created
    pub service_id: Option<i32>,
    spanner_node_id: Option<i32>,
    blocking_threadpool: Arc<BlockingThreadpool>,
    pub timeout: Option<Duration>,
}

impl TokenserverPool {
    pub fn new(
        settings: &Settings,
        metrics: &Metrics,
        blocking_threadpool: Arc<BlockingThreadpool>,
        _use_test_transactions: bool,
    ) -> DbResult<Self> {
        if settings.run_migrations {
            // XXX: blocking
            run_embedded_migrations(&settings.database_url)?;
        }

        use diesel_async::pooled_connection::deadpool::Pool;
        use diesel_async::pooled_connection::AsyncDieselConnectionManager;
        //use diesel_async::RunQueryDsl;

        // XXX: min_idle?
        // XXX: TestTransactionCustomizer
        let wait = settings
            .database_pool_connection_timeout
            .map(|seconds| Duration::from_secs(seconds as u64));
        let timeouts = deadpool::managed::Timeouts {
            wait,
            ..Default::default()
        };
        let config = deadpool::managed::PoolConfig {
            max_size: settings.database_pool_max_size as usize,
            timeouts,
            // Prefer LIFO to allow the sweeper task to evict least frequently
            // used connections.
            queue_mode: deadpool::managed::QueueMode::Lifo,
        };

        // create a new connection pool with the default config
        let manager = AsyncDieselConnectionManager::<diesel_async::AsyncMysqlConnection>::new(
            settings.database_url.clone(),
        );
        //let pool = Pool::builder(config).build()?;
        //let pool = Pool::builder(config).build().unwrap();

        //let pool = Pool::builder(config).build()?;
        let pool = Pool::builder(manager)
            .config(config)
            .runtime(deadpool::Runtime::Tokio1)
            /*
                    .post_create(deadpool::managed::Hook::async_fn(
                        async |conn: &mut diesel_async::AsyncMysqlConnection,
                               _metrics: &deadpool::managed::Metrics| {
                            use diesel_async::AsyncConnection;
                            conn.begin_test_transaction().await
                        },
            ))*/
            .post_create(deadpool::managed::Hook::async_fn(
                |conn: &mut diesel_async::AsyncMysqlConnection,
                 _metrics: &deadpool::managed::Metrics| {
                    use diesel_async::AsyncConnection;
                    Box::pin(async {
                        conn.begin_test_transaction().await.map_err(|e| {
                            diesel_async::pooled_connection::deadpool::HookError::Backend(
                                diesel_async::pooled_connection::PoolError::QueryError(e),
                            )
                        })
                    })
                },
            ))
            .build()
            .unwrap();

        /*
        // checkout a connection from the pool
        let mut conn = pool.get().await?;
        */
        /*
            let manager = ConnectionManager::<MysqlConnection>::new(settings.database_url.clone());

            let builder = Pool::builder()
                .max_size(settings.database_pool_max_size)
                .connection_timeout(Duration::from_secs(
                    settings.database_pool_connection_timeout.unwrap_or(30) as u64,
                ))
                .min_idle(settings.database_pool_min_idle);

            #[cfg(debug_assertions)]
            let builder = if _use_test_transactions {
                builder.connection_customizer(Box::new(TestTransactionCustomizer))
            } else {
                builder
        };
            */
        let timeout = settings
            .database_request_timeout
            .map(|v| Duration::from_secs(v as u64));

        Ok(Self {
            //inner: builder.build(manager)?,
            inner: pool,
            metrics: metrics.clone(),
            spanner_node_id: settings.spanner_node_id,
            service_id: None,
            blocking_threadpool,
            timeout,
        })
    }

    pub async fn get_sync(&self) -> Result<TokenserverDb, DbError> {
        let conn = self.inner.get().await.unwrap();

        Ok(TokenserverDb::new(
            conn,
            &self.metrics,
            self.service_id,
            self.spanner_node_id,
            self.blocking_threadpool.clone(),
            self.timeout,
        ))
    }

    #[cfg(test)]
    pub async fn get_tokenserver_db(&self) -> Result<TokenserverDb, DbError> {
        self.get_sync().await
        /*
            let pool = self.clone();
            let conn = self
                .blocking_threadpool
                .spawn(move || pool.inner.get().map_err(DbError::from))
                .await?;

            Ok(TokenserverDb::new(
                conn,
                &self.metrics,
                self.service_id,
                self.spanner_node_id,
                self.blocking_threadpool.clone(),
                self.timeout,
        ))
            */
    }
}

#[async_trait]
impl DbPool for TokenserverPool {
    async fn get(&self) -> Result<Box<dyn Db>, DbError> {
        let mut metrics = self.metrics.clone();
        metrics.start_timer("storage.get_pool", None);

        Ok(Box::new(self.get_sync().await?) as Box<dyn Db>)

        /*
        let pool = self.clone();
        let conn = self
            .blocking_threadpool
            .spawn(move || pool.inner.get().map_err(DbError::from))
        .await?;

        Ok(Box::new(TokenserverDb::new(
            conn,
            &self.metrics,
            self.service_id,
            self.spanner_node_id,
            self.blocking_threadpool.clone(),
            self.timeout,
        )) as Box<dyn Db>)
         */
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
        panic!("XXX")
        //self.inner.state().into()
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
