//! Sync Storage Server for Sync 1.6
use std::error::Error;

use docopt::Docopt;
use log::info;
use serde_derive::Deserialize;

use syncstorage::{logging, server, settings};

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

fn main() -> Result<(), Box<dyn Error>> {
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
    let banner = settings.banner();
    let sys = server::Server::with_settings(settings).unwrap();
    info!("Server running on {}", banner);
    sys.run()?;
    info!("Server closing");
    logging::reset_logging();

    Ok(())
}
