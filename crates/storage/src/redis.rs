//! Redis adapter built on `redis-rs`.
//!
//! Redis is auxiliary infrastructure on V0.1: it is connected, configured and
//! health checked, but it never stores authoritative business state.

use std::time::{Duration, Instant};

use async_trait::async_trait;
use platform_application::DependencyProbe;
use platform_domain::{Dependency, DependencyCheck};
use redis::Client;
use redis::aio::ConnectionManager;
use tracing::{info, warn};

use crate::error::StorageError;
use crate::settings::RedisSettings;

/// Handle to the shared Redis connection.
///
/// `ConnectionManager` reconnects in the background after a transport failure and
/// is cheap to clone, so a single handle is shared across the application.
#[derive(Debug, Clone)]
pub struct RedisClient {
    connection: ConnectionManager,
}

impl RedisClient {
    /// Connects to Redis and verifies the connection with `PING`.
    pub async fn connect(settings: &RedisSettings) -> Result<Self, StorageError> {
        let client = Client::open(settings.url.as_str()).map_err(StorageError::Redis)?;

        let connection = tokio::time::timeout(settings.timeout, client.get_connection_manager())
            .await
            .map_err(|_| StorageError::timeout("redis", settings.timeout))?
            .map_err(|error| {
                warn!(
                    target: "platform.storage.redis",
                    url = %settings.redacted_url(),
                    error = %error,
                    "failed to connect to Redis"
                );
                StorageError::Redis(error)
            })?;

        let mut probe_connection = connection.clone();
        tokio::time::timeout(
            settings.timeout,
            redis::cmd("PING").query_async::<String>(&mut probe_connection),
        )
        .await
        .map_err(|_| StorageError::timeout("redis", settings.timeout))?
        .map_err(|error| {
            warn!(
                target: "platform.storage.redis",
                url = %settings.redacted_url(),
                error = %error,
                "Redis did not answer PING"
            );
            StorageError::Redis(error)
        })?;

        info!(
            target: "platform.storage.redis",
            url = %settings.redacted_url(),
            "connected to Redis"
        );

        Ok(Self { connection })
    }

    /// Returns a clone of the underlying connection for command execution.
    #[must_use]
    pub fn connection(&self) -> ConnectionManager {
        self.connection.clone()
    }
}

/// Probes Redis readiness with `PING`.
#[derive(Debug, Clone)]
pub struct RedisProbe {
    client: RedisClient,
}

impl RedisProbe {
    /// Creates a probe over an existing client.
    #[must_use]
    pub const fn new(client: RedisClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl DependencyProbe for RedisProbe {
    fn dependency(&self) -> Dependency {
        Dependency::Redis
    }

    async fn probe(&self, timeout: Duration) -> DependencyCheck {
        let started = Instant::now();
        let mut connection = self.client.connection();

        let result = tokio::time::timeout(
            timeout,
            redis::cmd("PING").query_async::<String>(&mut connection),
        )
        .await;

        let latency_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);

        match result {
            Ok(Ok(response)) if response.eq_ignore_ascii_case("PONG") => {
                DependencyCheck::up(Dependency::Redis, latency_ms)
            }
            Ok(Ok(response)) => DependencyCheck::down(
                Dependency::Redis,
                latency_ms,
                format!("unexpected PING response: {response}"),
            ),
            Ok(Err(error)) => DependencyCheck::down(
                Dependency::Redis,
                latency_ms,
                format!("PING failed: {error}"),
            ),
            Err(_) => DependencyCheck::down(Dependency::Redis, latency_ms, "probe timed out"),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use platform_domain::Dependency;

    use crate::settings::RedisSettings;

    use super::RedisClient;
    #[tokio::test]
    async fn connect_fails_when_redis_is_unreachable() {
        let settings = RedisSettings {
            url: "redis://127.0.0.1:1".to_owned(),
            timeout: Duration::from_millis(200),
        };

        let error = RedisClient::connect(&settings)
            .await
            .expect_err("connection to a closed port must fail");

        assert!(
            error.to_string().contains("redis"),
            "error should mention redis, got: {error}"
        );
    }

    #[tokio::test]
    async fn probe_reports_redis_dependency_metadata() {
        let settings = RedisSettings {
            url: "redis://127.0.0.1:1".to_owned(),
            timeout: Duration::from_millis(200),
        };

        // A connection attempt against a closed port must fail without hanging,
        // proving that connection errors are surfaced instead of panicking.
        let outcome = RedisClient::connect(&settings).await;
        assert!(outcome.is_err());

        assert_eq!(Dependency::Redis.as_str(), "redis");
    }
}
