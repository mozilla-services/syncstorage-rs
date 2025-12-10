pub mod error;
pub mod test;

use std::{error::Error, fmt::Debug};

#[cfg(debug_assertions)]
use diesel::connection::InstrumentationEvent;
use diesel::{result::ConnectionResult, Connection};
use diesel_async::{
    async_connection_wrapper::AsyncConnectionWrapper, pooled_connection::ManagerConfig,
    AsyncConnection, AsyncMigrationHarness,
};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness};
use tokio::task::spawn_blocking;

/// A trait to be implemented by database pool data structures. It provides an interface to
/// derive the current state of the pool, as represented by the `PoolState` struct.
pub trait GetPoolState {
    fn state(&self) -> PoolState;
}

#[derive(Debug, Default)]
/// A mockable r2d2::State
pub struct PoolState {
    pub connections: u32,
    pub idle_connections: u32,
}

impl From<deadpool::Status> for PoolState {
    fn from(status: deadpool::Status) -> PoolState {
        PoolState {
            connections: status.size as u32,
            idle_connections: status.available.max(0) as u32,
        }
    }
}

/// Establish an [AsyncConnection] logging diesel queries to the `debug` log
///
/// Query logging is only enabled on non-optimized (debug_assertions) builds
pub async fn establish_connection_with_logging<T>(url: &str) -> ConnectionResult<T>
where
    T: AsyncConnection,
{
    #[allow(unused_mut)]
    let mut conn = <T as AsyncConnection>::establish(url).await?;
    #[cfg(debug_assertions)]
    conn.set_instrumentation(|event: InstrumentationEvent<'_>| {
        if let InstrumentationEvent::FinishQuery { query, error, .. } = event {
            // Prefer the plain log crate for now as it works easily w/ unit
            // tests via RUST_LOG=syncserver=debug
            if let Some(err) = error {
                log::debug!("QUERY Failed: {} {}", query, err);
            } else {
                log::debug!("QUERY: {}", query);
            }
        };
    });
    Ok(conn)
}

/// Return a [ManagerConfig] configured to log diesel queries to the `debug` log
///
/// Query logging is only enabled on non-optimized (debug_assertions) builds
pub fn manager_config_with_logging<C>() -> ManagerConfig<C>
where
    C: AsyncConnection + 'static,
{
    #[allow(unused_mut)]
    let mut manager_config = ManagerConfig::<C>::default();
    #[cfg(debug_assertions)]
    {
        manager_config.custom_setup =
            Box::new(|url| Box::pin(establish_connection_with_logging(url)));
    }
    manager_config
}

/// Run the diesel embedded migrations
///
/// Note that the migrations require a blocking operation ran via
/// `block_in_place`, so this function runs them via `spawn_blocking` for
/// compatibility with Tokio's current thread runtime
pub async fn run_embedded_migrations<C>(
    conn: C,
    source: EmbeddedMigrations,
) -> Result<(), Box<dyn Error + Send + Sync>>
where
    C: AsyncConnection + 'static,
    AsyncConnectionWrapper<C>: Connection<Backend = C::Backend> + MigrationHarness<C::Backend>,
{
    let mut harness = AsyncMigrationHarness::new(conn);
    spawn_blocking(move || {
        harness.run_pending_migrations(source)?;
        Ok(())
    })
    .await?
}
