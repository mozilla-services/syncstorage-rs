use std::time::Duration;

use async_trait::async_trait;
use deadpool::managed::PoolError;
use diesel::Connection;
use diesel_async::{
    async_connection_wrapper::AsyncConnectionWrapper,
    pooled_connection::{
        deadpool::{Object, Pool},
        AsyncDieselConnectionManager,
    },
    AsyncPgConnection,
};

use diesel_logger::LoggingConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

/// The `embed_migrations!` macro reads migrations at compile time.
/// This creates a constant that references a list of migrations.
/// See https://docs.rs/diesel_migrations/2.2.0/diesel_migrations/macro.embed_migrations.html
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub(crate) type Conn = Object<AsyncPgConnection>;

fn run_embedded_migrations(database_url: &str) -> Result<()> {
    let conn = AsyncConnectionWrapper::<AsyncPgConnection>::establish(database_url)?;
    LoggingConnection::new(conn).run_pending_migrations(MIGRATIONS)?;

    Ok(())
}

#[derive(Clone)]
pub struct TokenserverPgPool {
    /// Pool of db connections
    inner: Pool<AsyncPgConnection>,
    // This field is public so the service ID can be set after the pool is created
    pub service_id: Option<i32>,
    spanner_node_id: Option<i32>,
    pub timeout: Option<Duration>,
    run_migrations: bool,
    database_url: String,
}
