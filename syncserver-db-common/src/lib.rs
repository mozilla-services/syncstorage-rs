pub mod error;
pub mod test;

use std::{fmt::Debug, time::Duration};

use cadence::{Gauged, StatsdClient};
use futures::future::LocalBoxFuture;
use tokio::time;

pub type DbFuture<'a, T, E> = LocalBoxFuture<'a, Result<T, E>>;

/// Emit DbPool metrics periodically
pub fn spawn_pool_periodic_reporter<T: GetPoolState + Send + 'static>(
    interval: Duration,
    metrics: StatsdClient,
    pool: T,
) {
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
}

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

impl From<diesel::r2d2::State> for PoolState {
    fn from(state: diesel::r2d2::State) -> PoolState {
        PoolState {
            connections: state.connections,
            idle_connections: state.idle_connections,
        }
    }
}

impl From<deadpool::Status> for PoolState {
    fn from(status: deadpool::Status) -> PoolState {
        PoolState {
            connections: status.size as u32,
            idle_connections: status.available.max(0) as u32,
        }
    }
}

#[macro_export]
macro_rules! sync_db_method {
    ($name:ident, $sync_name:ident, $type:ident) => {
        sync_db_method!($name, $sync_name, $type, results::$type);
    };
    ($name:ident, $sync_name:ident, $type:ident, $result:ty) => {
        fn $name(&self, params: params::$type) -> DbFuture<'_, $result, DbError> {
            let db = self.clone();
            Box::pin(
                self.blocking_threadpool
                    .spawn(move || db.$sync_name(params)),
            )
        }
    };
}
