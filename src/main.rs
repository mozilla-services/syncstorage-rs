//! Sync Storage Server for Sync 1.5
#[macro_use]
extern crate slog_scope;

use std::error::Error;

use docopt::Docopt;
use serde_derive::Deserialize;

use logging::init_logging;
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

#[actix_rt::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());
    let settings = settings::Settings::with_env_and_config_file(&args.flag_config)?;
    init_logging(!settings.human_logs).expect("Logging failed to initialize");
    debug!("Starting up...");
    // Set SENTRY_DSN environment variable to enable Sentry.
    // Avoid its default reqwest transport for now due to issues w/
    // likely grpcio's boringssl
    let curl_transport_factory = |options: &sentry::ClientOptions| {
        // Note: set options.debug = true when diagnosing sentry issues.
        Box::new(sentry::transports::CurlHttpTransport::new(&options))
            as Box<dyn sentry::internals::Transport>
    };
    let sentry = sentry::init(sentry::ClientOptions {
        transport: Box::new(curl_transport_factory),
        release: sentry::release_name!(),
        ..sentry::ClientOptions::default()
    });
    if sentry.is_enabled() {
        sentry::integrations::panic::register_panic_handler();
    }

    // Setup and run the server
    let banner = settings.banner();
    let server = server::Server::with_settings(settings).unwrap();
    info!("Server running on {}", banner);
    server.await?;
    info!("Server closing");
    logging::reset_logging();

    Ok(())
}
