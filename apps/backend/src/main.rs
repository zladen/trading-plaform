//! Binary entry point.
//!
//! The entry point is intentionally thin: it loads configuration, installs
//! logging and delegates to the startup orchestrator. All meaningful behaviour
//! lives in the library so that it can be tested.

use std::process::ExitCode;

use platform_backend::config::{self, LoadOptions};
use platform_backend::startup;
use tracing::{error, info};

#[tokio::main]
async fn main() -> ExitCode {
    // Configuration errors must be visible even though the subscriber does not
    // exist yet, so configuration loading happens before logging is installed.
    let options = LoadOptions::with_default_env_file();

    let config = match config::load(&options) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("configuration error: {error}");
            return ExitCode::FAILURE;
        }
    };

    if let Err(error) = startup::logging::init(&config) {
        eprintln!("failed to initialise logging: {error}");
        return ExitCode::FAILURE;
    }

    info!(
        target: "platform.startup",
        environment = %config.environment,
        version = env!("CARGO_PKG_VERSION"),
        "starting platform backend"
    );

    match startup::run(config).await {
        Ok(()) => {
            info!(target: "platform.startup", "platform backend terminated cleanly");
            ExitCode::SUCCESS
        }
        Err(error) => {
            error!(
                target: "platform.startup",
                error = %format!("{error:#}"),
                "platform backend failed to start"
            );
            ExitCode::FAILURE
        }
    }
}
