//! Sync Storage Server for Sync 1.5
#[macro_use]
extern crate slog_scope;

use std::{error::Error, sync::Arc};

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

    // Set SENTRY_DSN environment variable to enable Sentry.
    // Avoid its default reqwest transport for now due to issues w/
    // likely grpcio's boringssl
    let curl_transport_factory = |options: &sentry::ClientOptions| {
        Arc::new(sentry::transports::CurlHttpTransport::new(options)) as Arc<dyn sentry::Transport>
    };
    // debug-images conflicts w/ our debug = 1 rustc build option:
    // https://github.com/getsentry/sentry-rust/issues/574
    let mut opts = sentry::apply_defaults(sentry::ClientOptions {
        // Note: set "debug: true," to diagnose sentry issues
        transport: Some(Arc::new(curl_transport_factory)),
        release: sentry::release_name!(),
        ..sentry::ClientOptions::default()
    });
    opts.integrations.retain(|i| i.name() != "debug-images");
    opts.default_integrations = false;
    let _sentry = sentry::init(opts);

    // Setup and run the server
    let banner = settings.banner();
    let server = if !settings.syncstorage.enabled {
        server::Server::tokenserver_only_with_settings(settings)
            .await
            .unwrap()
    } else {

        // If mysql isn't available yet this will throw a thread panic and kill
        // the process. This let's us try a few times for mysql to come up before
        // we panic. This is a bit of a hack, but it works for now. We may
        // need to revisit.
        let max_attempts = 10;
        let mut attempt = 0;
        loop {
            match server::Server::with_settings(settings.clone()).await {
                Ok(server) => break server,
                Err(e) => {
                    attempt += 1;
                    warn!("Failed to initialize server on attempt {}: {}", attempt, e);
                    if attempt >= max_attempts {
                        // let env_vars: String = std::env::vars()
                        //     .map(|(key, value)| format!("{}={}", key, value))
                        //     .collect::<Vec<String>>()
                        //     .join("\n");
                        panic!(
                            // "Failed to initialize server after {} attempts: {}\nEnvironment Variables:\n{}",
                            "Failed to initialize server after {} attempts: {}",
                            max_attempts, e
                        );
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }
        }
    };
    info!("Server running on {}", banner);
    server.await?;
    info!("Server closing");
    logging::reset_logging();

    Ok(())
}
