//! PostgreSQL adapter built on SQLx.

use std::time::{Duration, Instant};

use async_trait::async_trait;
use platform_application::DependencyProbe;
use platform_domain::{Dependency, DependencyCheck};
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Postgres, Transaction};
use tracing::{info, warn};

use crate::error::StorageError;
use crate::settings::PostgresSettings;

/// Connection pool handle exposed to the rest of the application.
pub type PostgresPool = PgPool;

/// Opens the PostgreSQL connection pool and verifies connectivity.
///
/// The pool is created eagerly so that a misconfigured or unreachable database is
/// detected during startup instead of on the first request.
pub async fn connect(settings: &PostgresSettings) -> Result<PostgresPool, StorageError> {
    let pool = PgPoolOptions::new()
        .max_connections(settings.max_connections)
        .min_connections(settings.min_connections)
        .acquire_timeout(settings.acquire_timeout)
        .test_before_acquire(true)
        .connect(&settings.url)
        .await
        .map_err(|error| {
            warn!(
                target: "platform.storage.postgres",
                url = %settings.redacted_url(),
                error = %error,
                "failed to connect to PostgreSQL"
            );
            StorageError::Postgres(error)
        })?;

    info!(
        target: "platform.storage.postgres",
        url = %settings.redacted_url(),
        max_connections = settings.max_connections,
        "connected to PostgreSQL"
    );

    Ok(pool)
}

/// Runs embedded migrations against the pool.
pub async fn run_migrations(pool: &PostgresPool) -> Result<(), StorageError> {
    sqlx::migrate!("../../migrations")
        .run(pool)
        .await
        .map_err(StorageError::Migration)?;

    info!(target: "platform.storage.postgres", "database migrations applied");

    Ok(())
}

/// Begins a transaction used for atomic multi-statement operations.
///
/// Exposed so that repositories in this crate can compose atomic operations
/// without leaking SQLx types into the application layer.
pub async fn begin(pool: &PostgresPool) -> Result<Transaction<'static, Postgres>, StorageError> {
    pool.begin().await.map_err(StorageError::Postgres)
}

/// Probes PostgreSQL readiness with `SELECT 1`.
#[derive(Debug, Clone)]
pub struct PostgresProbe {
    pool: PostgresPool,
}

impl PostgresProbe {
    /// Creates a probe over an existing pool.
    #[must_use]
    pub const fn new(pool: PostgresPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DependencyProbe for PostgresProbe {
    fn dependency(&self) -> Dependency {
        Dependency::Postgres
    }

    async fn probe(&self, timeout: Duration) -> DependencyCheck {
        let started = Instant::now();

        let result = tokio::time::timeout(
            timeout,
            sqlx::query_scalar::<_, i32>("SELECT 1").fetch_one(&self.pool),
        )
        .await;

        let latency_ms = elapsed_ms(started);

        match result {
            Ok(Ok(1)) => DependencyCheck::up(Dependency::Postgres, latency_ms),
            Ok(Ok(value)) => DependencyCheck::down(
                Dependency::Postgres,
                latency_ms,
                format!("unexpected health query result: {value}"),
            ),
            Ok(Err(error)) => DependencyCheck::down(
                Dependency::Postgres,
                latency_ms,
                format!("health query failed: {error}"),
            ),
            Err(_) => DependencyCheck::down(Dependency::Postgres, latency_ms, "probe timed out"),
        }
    }
}

fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use platform_application::DependencyProbe;
    use platform_domain::Dependency;

    use super::PostgresProbe;

    #[tokio::test]
    async fn probe_reports_down_when_database_is_unreachable() {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .acquire_timeout(Duration::from_millis(200))
            .connect_lazy("postgres://platform:wrong@127.0.0.1:1/platform")
            .expect("lazy pool should be created");

        let check = PostgresProbe::new(pool)
            .probe(Duration::from_millis(300))
            .await;

        assert_eq!(check.dependency(), Dependency::Postgres);
        assert!(!check.state().is_up());
        assert!(check.error().is_some());
    }
}
