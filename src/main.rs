//! Sync Storage Server for Sync 1.6
extern crate actix;
extern crate actix_web;
extern crate base64;
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
#[macro_use]
extern crate lazy_static;
extern crate hawk;
extern crate hkdf;
extern crate hmac;
extern crate mozsvc_common;
extern crate num;
extern crate num_cpus;
extern crate rand;
extern crate regex;
extern crate ring;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate sha2;
extern crate time;
extern crate uuid;
#[macro_use]
extern crate validator_derive;
extern crate validator;

use std::error::Error;

use docopt::Docopt;

pub mod db;
pub mod server;
pub mod settings;
pub mod web;

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
    let sys = server::Server::with_settings(settings);
    println!("Server running");
    let _ = sys.run();

    Ok(())
}
