//! Sync Storage Server for Sync 1.5
#[macro_use]
extern crate slog_scope;

use std::error::Error;

use docopt::Docopt;
use serde::Deserialize;

use logging::init_logging;
use syncserver::{logging, server};
use syncserver_settings::Settings;

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

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());
    let settings = Settings::with_env_and_config_file(args.flag_config.as_deref())?;
    init_logging(!settings.human_logs).expect("Logging failed to initialize");
    debug!("Starting up...");
    let _sentry = sentry::init(sentry::ClientOptions {
        // Note: set "debug: true," to diagnose sentry issues
        release: sentry::release_name!(),
        ..sentry::ClientOptions::default()
    });

    // Setup and run the server
    let banner = settings.banner();
    let server = if !settings.syncstorage.enabled {
        server::Server::tokenserver_only_with_settings(settings)
            .await
            .unwrap()
    } else {
        server::Server::with_settings(settings).await.unwrap()
    };
    info!("Server running on {}", banner);
    server.await?;
    info!("Server closing");
    logging::reset_logging();

    Ok(())
}
