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

#[cfg(all(feature = "mysql", feature = "spanner"))]
compile_error!("only one of the \"mysql\" and \"spanner\" features can be enabled at a time");

#[cfg(not(any(feature = "mysql", feature = "spanner")))]
compile_error!("exactly one of the \"mysql\" and \"spanner\" features must be enabled");
