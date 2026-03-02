//! PostgreSQL database implementation for Tokenserver.
//!
//! This crate provides PostgreSQL-specific implementation of the Tokenserver database
//! traits, including connection pooling and database operations.

#![allow(non_local_definitions)]
extern crate diesel;
extern crate diesel_migrations;

mod db;
mod pool;

pub use db::{
    TokenserverPgDb,
    orm_models::{Node, Service, User},
};
pub use pool::TokenserverPgPool;
pub use tokenserver_db_common::{Db, DbPool, params, results};
