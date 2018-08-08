//! Main application
#[macro_use]
extern crate actix;
extern crate actix_web;
extern crate chrono;
extern crate config;
#[macro_use]
extern crate diesel;
extern crate docopt;
extern crate futures;
//extern crate hawk;
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
mod dispatcher;
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
