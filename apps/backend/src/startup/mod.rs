//! Startup orchestration.
//!
//! The startup sequence is deliberately explicit and ordered: configuration is
//! validated first, then logging, then dependencies, then the HTTP listener. The
//! process never reports readiness before every mandatory dependency answered.

pub mod logging;
pub mod shutdown;

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use chrono::Utc;
use platform_application::{HealthService, InfoService, ReadinessService};
use platform_shared::BuildInfo;
use platform_storage::{PostgresProbe, RedisClient, RedisProbe};
use tokio::net::TcpListener;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::config::Config;
use crate::http::build_router;
use crate::state::AppState;

/// How long startup keeps retrying an unavailable dependency.
const DEPENDENCY_CONNECT_DEADLINE: Duration = Duration::from_secs(60);

/// Initial delay between dependency connection attempts.
const DEPENDENCY_RETRY_INITIAL: Duration = Duration::from_millis(500);

/// Upper bound for the exponential retry delay.
const DEPENDENCY_RETRY_MAX: Duration = Duration::from_secs(5);

/// Runs the backend until a shutdown signal arrives.
///
/// # Errors
///
/// Returns an error when configuration, dependency connections, migrations or
/// the listener cannot be established. Startup failures are fatal: the process
/// must not advertise readiness it cannot honour.
pub async fn run(config: Config) -> anyhow::Result<()> {
    let started_at = Utc::now();
    let cancellation = CancellationToken::new();

    let build = BuildInfo::current();

    let postgres_settings = config.postgres_settings();
    let redis_settings = config.redis_settings();

    let pool = connect_with_retry("postgres", &cancellation, || {
        let settings = postgres_settings.clone();
        async move {
            platform_storage::postgres::connect(&settings)
                .await
                .map_err(anyhow::Error::from)
        }
    })
    .await
    .context("failed to connect to PostgreSQL")?;

    if config.database.run_migrations {
        platform_storage::postgres::run_migrations(&pool)
            .await
            .context("failed to run database migrations")?;
    } else {
        warn!(
            target: "platform.startup",
            "database migrations are disabled by configuration"
        );
    }

    let redis = connect_with_retry("redis", &cancellation, || {
        let settings = redis_settings.clone();
        async move {
            RedisClient::connect(&settings)
                .await
                .map_err(anyhow::Error::from)
        }
    })
    .await
    .context("failed to connect to Redis")?;

    let health = HealthService::new(build.clone(), started_at);
    let info = InfoService::new(build.clone(), config.environment.as_str(), started_at)
        .context("failed to build the service information use case")?;

    let readiness = ReadinessService::new(
        vec![
            Arc::new(PostgresProbe::new(pool.clone())),
            Arc::new(RedisProbe::new(redis.clone())),
        ],
        config.readiness_timeout,
    );

    let state = AppState::new(
        Arc::new(config.clone()),
        build,
        started_at,
        health,
        info,
        readiness,
    );

    let router = build_router(state);

    let listener = TcpListener::bind(config.bind_address())
        .await
        .with_context(|| format!("failed to bind {}", config.bind_address()))?;

    let local_address = listener
        .local_addr()
        .context("failed to read the bound listener address")?;

    info!(
        target: "platform.startup",
        address = %local_address,
        environment = %config.environment,
        swagger = config.swagger_enabled,
        "platform backend is ready to accept connections"
    );

    let signal_token = cancellation.clone();
    let signal_task = tokio::spawn(async move {
        shutdown::shutdown_signal(signal_token).await;
    });

    let serve_result = axum::serve(
        listener,
        router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown({
        let cancellation = cancellation.clone();
        async move { cancellation.cancelled().await }
    })
    .await;

    match &serve_result {
        Ok(()) => {
            info!(
                target: "platform.startup",
                "HTTP server stopped accepting connections"
            );
        }
        Err(error) => {
            error!(
                target: "platform.startup",
                error = %error,
                "HTTP server terminated with an error"
            );
            cancellation.cancel();
        }
    }

    // Allow in-flight requests to finish before infrastructure handles close.
    tokio::time::sleep(config.shutdown.timeout.min(Duration::from_secs(10))).await;

    signal_task.abort();

    info!(target: "platform.startup", "closing PostgreSQL connection pool");
    pool.close().await;

    drop(redis);

    info!(target: "platform.startup", "platform backend stopped");

    serve_result.context("HTTP server failed")
}

/// Retries a connection attempt until it succeeds or the startup deadline passes.
async fn connect_with_retry<T, F, Fut>(
    dependency: &'static str,
    cancellation: &CancellationToken,
    mut attempt: F,
) -> anyhow::Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<T>>,
{
    let started = Instant::now();
    let mut delay = DEPENDENCY_RETRY_INITIAL;
    let mut attempts: u32 = 0;

    loop {
        if cancellation.is_cancelled() {
            anyhow::bail!("startup cancelled while connecting to {dependency}");
        }

        attempts += 1;

        match attempt().await {
            Ok(value) => {
                info!(
                    target: "platform.startup",
                    dependency = dependency,
                    attempts = attempts,
                    elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
                    "dependency connection established"
                );
                return Ok(value);
            }
            Err(error) => {
                if started.elapsed() >= DEPENDENCY_CONNECT_DEADLINE {
                    return Err(error).with_context(|| {
                        format!(
                            "{dependency} was unreachable for {} seconds during startup",
                            DEPENDENCY_CONNECT_DEADLINE.as_secs()
                        )
                    });
                }

                warn!(
                    target: "platform.startup",
                    dependency = dependency,
                    attempt = attempts,
                    error = %error,
                    retry_in_ms = u64::try_from(delay.as_millis()).unwrap_or(u64::MAX),
                    "dependency is not available yet, retrying"
                );

                tokio::select! {
                    () = tokio::time::sleep(delay) => {}
                    () = cancellation.cancelled() => {
                        anyhow::bail!("startup cancelled while connecting to {dependency}");
                    }
                }

                delay = (delay * 2).min(DEPENDENCY_RETRY_MAX);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio_util::sync::CancellationToken;

    use super::connect_with_retry;

    #[tokio::test]
    async fn retry_returns_the_first_successful_value() {
        let cancellation = CancellationToken::new();
        let mut calls = 0;

        let value = connect_with_retry::<u8, _, _>("test", &cancellation, || {
            calls += 1;
            async move { Ok(7_u8) }
        })
        .await
        .expect("first attempt succeeds");

        assert_eq!(value, 7);
        assert_eq!(calls, 1);
    }

    #[tokio::test]
    async fn retry_stops_when_cancelled() {
        let cancellation = CancellationToken::new();
        cancellation.cancel();

        let result =
            connect_with_retry::<u8, _, _>("test", &cancellation, || async { Ok(1_u8) }).await;

        assert!(result.is_err(), "cancelled startup must not report success");
    }

    #[tokio::test(start_paused = true)]
    async fn retry_gives_up_after_the_startup_deadline() {
        let cancellation = CancellationToken::new();

        let error = connect_with_retry::<u8, _, _>("postgres", &cancellation, || async {
            Err(anyhow::anyhow!("still unavailable"))
        })
        .await
        .expect_err("an unreachable dependency must fail startup");

        let rendered = format!("{error:#}").to_lowercase();
        assert!(
            rendered.contains("postgres"),
            "error must name the dependency, got: {rendered}"
        );
    }
}
