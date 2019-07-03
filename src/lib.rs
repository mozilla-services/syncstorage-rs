#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;
#[macro_use]
extern crate validator_derive;

#[macro_use]
pub mod error;
pub mod db;
pub mod server;
pub mod settings;
pub mod web;
