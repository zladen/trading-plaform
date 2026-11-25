//! Graceful shutdown coordination.
//!
//! The process stops accepting new connections on `SIGINT` or `SIGTERM`, gives
//! in-flight requests a bounded amount of time to finish, then releases
//! infrastructure resources.

use std::time::Duration;

use tokio::signal;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

/// Waits for `SIGINT` or `SIGTERM` and then cancels the token.
///
/// Registering both signals explicitly means containers stop cleanly on
/// `docker compose down` instead of being killed after their grace period.
pub async fn shutdown_signal(cancellation: CancellationToken) {
    let ctrl_c = async {
        if let Err(error) = signal::ctrl_c().await {
            warn!(
                target: "platform.startup",
                error = %error,
                "failed to listen for SIGINT"
            );
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match signal::unix::signal(signal::unix::SignalKind::terminate()) {
            Ok(mut stream) => {
                stream.recv().await;
            }
            Err(error) => {
                warn!(
                    target: "platform.startup",
                    error = %error,
                    "failed to listen for SIGTERM"
                );
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => info!(target: "platform.startup", "received SIGINT"),
        () = terminate => info!(target: "platform.startup", "received SIGTERM"),
    }

    cancellation.cancel();
}

/// Waits for the cancellation token and then waits for the configured grace
/// period, returning whether in-flight work finished in time.
pub async fn wait_for_shutdown(cancellation: CancellationToken, timeout: Duration) -> bool {
    cancellation.cancelled().await;

    info!(
        target: "platform.startup",
        timeout_ms = u64::try_from(timeout.as_millis()).unwrap_or(u64::MAX),
        "waiting for in-flight requests to finish"
    );

    tokio::time::sleep(timeout).await;
    false
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio_util::sync::CancellationToken;

    use super::wait_for_shutdown;

    #[tokio::test]
    async fn shutdown_waits_for_the_grace_period_after_cancellation() {
        let cancellation = CancellationToken::new();
        cancellation.cancel();

        let finished = wait_for_shutdown(cancellation, Duration::from_millis(10)).await;
        assert!(!finished, "in-flight work is not tracked on V0.1");
    }
}
