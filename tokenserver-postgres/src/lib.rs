#![allow(non_local_definitions)]
extern crate diesel;
extern crate diesel_migrations;

#[macro_use]
extern crate slog_scope;

mod db;
mod pool;

pub use db::{
    TokenserverPgDb,
    orm_models::{Node, Service, User},
};
pub use pool::TokenserverPgPool;
pub use tokenserver_db_common::{Db, DbPool, params, results};
