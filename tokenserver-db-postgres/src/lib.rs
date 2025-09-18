#![allow(non_local_definitions)]
extern crate diesel;
extern crate diesel_migrations;

mod models;
mod pool;

pub use models::TokenserverPgDb;
pub use pool::TokenserverPgPool;
