#![allow(non_local_definitions)]
extern crate diesel;
extern crate diesel_migrations;

mod pool;

pub use tokenserver_db_common::{params, results, Db, DbPool};
