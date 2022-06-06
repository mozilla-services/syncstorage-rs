#![warn(rust_2018_idioms)]
#![allow(clippy::try_err)]

#[macro_use]
extern crate slog_scope;
#[macro_use]
extern crate validator_derive;

#[macro_use]
pub mod error;
pub mod db;
pub mod logging;
pub mod server;
pub mod tokenserver;
pub mod web;

// TODO: which dependencies can be optional?
// TODO: compile_error!
