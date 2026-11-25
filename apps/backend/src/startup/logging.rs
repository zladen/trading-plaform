//! Logging initialisation.

use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::time::UtcTime;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::config::{Config, LogFormat};

/// Initialises the global tracing subscriber.
///
/// Must be called exactly once, as early as possible in `main`, so that
/// configuration and startup failures are themselves observable.
///
/// # Errors
///
/// Returns an error when the configured filter or subscriber cannot be built, or
/// when a global subscriber is already installed.
pub fn init(config: &Config) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let filter = EnvFilter::try_new(&config.logging.filter)?;
    let timer = UtcTime::rfc_3339();

    match config.logging.format {
        LogFormat::Json => {
            let layer = tracing_subscriber::fmt::layer()
                .json()
                .with_timer(timer)
                .with_current_span(true)
                .with_span_list(false);

            tracing_subscriber::registry()
                .with(filter)
                .with(layer)
                .try_init()?;
        }
        LogFormat::Pretty => {
            let layer = tracing_subscriber::fmt::layer()
                .with_timer(timer)
                .with_target(true)
                .with_ansi(!config.is_production());

            tracing_subscriber::registry()
                .with(filter)
                .with(layer)
                .try_init()?;
        }
    }

    Ok(())
}

/// Installs a minimal subscriber for tests and tools that only need filtered
/// logs. Safe to call multiple times: a second call is a no-op.
pub fn init_minimal(filter: &str) {
    let filter = EnvFilter::try_new(filter).unwrap_or_else(|_| EnvFilter::new("warn"));

    let _ = tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .try_init();
}
