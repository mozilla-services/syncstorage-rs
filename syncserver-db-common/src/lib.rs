pub mod error;
pub mod test;
pub mod util;

use std::fmt::Debug;

use futures::future::LocalBoxFuture;

pub type DbFuture<'a, T, E> = LocalBoxFuture<'a, Result<T, E>>;

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
