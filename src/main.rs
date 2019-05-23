//! Sync Storage Server for Sync 1.6
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;
#[macro_use]
extern crate validator_derive;

use std::error::Error;

use docopt::Docopt;
use serde_derive::Deserialize;

#[macro_use]
pub mod error;
pub mod db;
pub mod server;
pub mod settings;
pub mod web;

const USAGE: &str = "
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
    env_logger::init();
    // Set SENTRY_DSN environment variable to enable Sentry
    let sentry = sentry::init(sentry::ClientOptions::default());
    if sentry.is_enabled() {
        sentry::integrations::panic::register_panic_handler();
    }

    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());
    let settings = settings::Settings::with_env_and_config_file(&args.flag_config)?;

    // Setup and run the server
    let sys = server::Server::with_settings(settings).unwrap();
    println!("Server running");
    let _ = sys.run();

    Ok(())
}
