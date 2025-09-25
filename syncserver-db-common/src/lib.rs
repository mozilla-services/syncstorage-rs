pub mod error;
pub mod test;

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

impl From<deadpool::Status> for PoolState {
    fn from(status: deadpool::Status) -> PoolState {
        PoolState {
            connections: status.size as u32,
            idle_connections: status.available.max(0) as u32,
        }
    }
}

#[macro_export]
macro_rules! async_db_method {
    ($name:ident, $async_name:path, $type:ident) => {
        async_db_method!($name, $async_name, $type, results::$type);
    };
    ($name:ident, $async_name:path, $type:ident, $result:ty) => {
        fn $name(&mut self, params: params::$type) -> DbFuture<'_, $result, DbError> {
            Box::pin($async_name(self, params))
        }
    };
}
