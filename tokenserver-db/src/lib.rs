#![allow(non_local_definitions)]
extern crate diesel;
extern crate diesel_migrations;
#[macro_use]
extern crate slog_scope;

mod error;
pub mod mock;
mod models;
pub mod params;
mod pool;
pub mod results;

pub use models::{Db, TokenserverDb};
pub use pool::{DbPool, TokenserverPool};

#[macro_export]
macro_rules! async_db_method {
    ($name:ident, $sync_name:ident, $type:ident) => {
        async_db_method!($name, $sync_name, $type, results::$type);
    };
    ($name:ident, $sync_name:ident, $type:ident, $result:ty) => {
        fn $name(&mut self, params: params::$type) -> DbFuture<'_, $result, DbError> {
            Box::pin(self.$sync_name(params))
        }
    };
}
