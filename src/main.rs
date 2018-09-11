// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, you can obtain one at https://mozilla.org/MPL/2.0/.

//! Main application
extern crate actix;
extern crate actix_web;
extern crate chrono;
extern crate config;
#[macro_use]
extern crate diesel;
#[cfg(test)]
extern crate diesel_logger;
#[macro_use]
extern crate diesel_migrations;
extern crate docopt;
extern crate env_logger;
#[macro_use]
extern crate failure;
extern crate futures;
//extern crate hawk;
#[macro_use]
extern crate lazy_static;
extern crate mozsvc_common;
extern crate num_cpus;
extern crate rand;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate uuid;

use std::error::Error;

use docopt::Docopt;

mod db;
mod handlers;
mod server;
mod settings;

const USAGE: &'static str = "
Usage: syncstorage [options]

Options:
    -h, --help               Show this message.
    --config=CONFIGFILE      Syncstorage configuration file path.
";

#[derive(Debug, Deserialize)]
struct Args {
    flag_config: Option<String>,
}

// XXX: failure for Error types
fn main() -> Result<(), Box<Error>> {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());
    let settings = settings::Settings::with_env_and_config_file(&args.flag_config)?;

    // Setup and run the server
    let sys = server::Server::with_settings(&settings);
    println!("Server running");
    let _ = sys.run();

    Ok(())
}
