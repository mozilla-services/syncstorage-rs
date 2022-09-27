pub mod error;
pub mod test;

use std::fmt::{self, Debug};

use actix_web::{error::BlockingError, web};
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

pub async fn run_on_blocking_threadpool<F, T, E, M>(f: F, e: M) -> Result<T, E>
where
    F: FnOnce() -> Result<T, E> + Send + 'static,
    T: Send + 'static,
    E: fmt::Debug + Send + 'static,
    M: FnOnce(String) -> E,
{
    web::block(f)
        .await
        .map_err(|blocking_error| match blocking_error {
            BlockingError::Error(inner) => inner,
            BlockingError::Canceled => e("Db threadpool operation canceled".to_owned()),
        })
}

#[macro_export]
macro_rules! sync_db_method {
    ($name:ident, $sync_name:ident, $type:ident) => {
        sync_db_method!($name, $sync_name, $type, results::$type);
    };
    ($name:ident, $sync_name:ident, $type:ident, $result:ty) => {
        fn $name(&self, params: params::$type) -> DbFuture<'_, $result, Self::Error> {
            let db = self.clone();
            Box::pin(syncserver_db_common::run_on_blocking_threadpool(
                move || db.$sync_name(params),
                Self::Error::internal,
            ))
        }
    };
}
