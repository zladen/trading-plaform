//! Storage layer errors.

use thiserror::Error;

/// Errors produced by infrastructure adapters.
#[derive(Debug, Error)]
pub enum StorageError {
    /// The PostgreSQL connection pool could not be created or a query failed.
    #[error("postgres error: {0}")]
    Postgres(#[from] sqlx::Error),

    /// Database migrations failed.
    #[error("failed to run database migrations: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    /// The Redis client could not be created or a command failed.
    #[error("redis error: {0}")]
    Redis(#[from] redis::RedisError),

    /// A dependency could not be reached within the configured deadline.
    #[error("{dependency} did not respond within {timeout_ms} ms")]
    Timeout {
        /// Dependency identifier.
        dependency: &'static str,
        /// Configured timeout in milliseconds.
        timeout_ms: u64,
    },
}

impl StorageError {
    /// Builds a timeout error for a named dependency.
    #[must_use]
    pub fn timeout(dependency: &'static str, timeout: std::time::Duration) -> Self {
        Self::Timeout {
            dependency,
            timeout_ms: u64::try_from(timeout.as_millis()).unwrap_or(u64::MAX),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::StorageError;

    #[test]
    fn timeout_error_includes_dependency_and_deadline() {
        let error = StorageError::timeout("redis", Duration::from_millis(250));
        assert_eq!(error.to_string(), "redis did not respond within 250 ms");
    }
}
