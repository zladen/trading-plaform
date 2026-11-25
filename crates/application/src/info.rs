//! Application information use case.

use chrono::{DateTime, Utc};
use platform_domain::DomainError;
use platform_shared::BuildInfo;
use serde::{Deserialize, Serialize};

/// Describes the running service to API clients.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceInfo {
    name: String,
    version: String,
    rust_version: Option<String>,
    environment: String,
    started_at: DateTime<Utc>,
    uptime_seconds: u64,
}

impl ServiceInfo {
    /// Application name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Application version.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Rust compiler version recorded at build time.
    #[must_use]
    pub fn rust_version(&self) -> Option<&str> {
        self.rust_version.as_deref()
    }

    /// Deployment environment name.
    #[must_use]
    pub fn environment(&self) -> &str {
        &self.environment
    }

    /// Instant the process started at.
    #[must_use]
    pub const fn started_at(&self) -> DateTime<Utc> {
        self.started_at
    }

    /// Whole seconds the process has been running.
    #[must_use]
    pub const fn uptime_seconds(&self) -> u64 {
        self.uptime_seconds
    }
}

/// Builds [`ServiceInfo`] values for the running process.
#[derive(Debug, Clone)]
pub struct InfoService {
    build: BuildInfo,
    environment: String,
    started_at: DateTime<Utc>,
}

impl InfoService {
    /// Creates the service.
    ///
    /// Fails when the environment name is empty, because an unnamed environment
    /// would make operational reports ambiguous.
    pub fn new(
        build: BuildInfo,
        environment: impl Into<String>,
        started_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        let environment = environment.into();
        if environment.trim().is_empty() {
            return Err(DomainError::EmptyValue {
                field: "environment",
            });
        }

        Ok(Self {
            build,
            environment,
            started_at,
        })
    }

    /// Produces the service description at `now`.
    #[must_use]
    pub fn describe(&self, now: DateTime<Utc>) -> ServiceInfo {
        let elapsed = now.signed_duration_since(self.started_at);

        ServiceInfo {
            name: self.build.name().to_owned(),
            version: self.build.version().to_owned(),
            rust_version: self.build.rust_version().map(str::to_owned),
            environment: self.environment.clone(),
            started_at: self.started_at,
            uptime_seconds: elapsed.num_seconds().max(0).unsigned_abs(),
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone, Utc};
    use platform_domain::DomainError;
    use platform_shared::BuildInfo;

    use super::InfoService;

    fn started_at() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 2, 1, 12, 0, 0).unwrap()
    }

    fn service() -> InfoService {
        InfoService::new(
            BuildInfo::new("platform-backend", "0.1.0", Some("1.97.1".to_owned())),
            "development",
            started_at(),
        )
        .expect("service info should be created")
    }

    #[test]
    fn describes_build_and_runtime() {
        let info = service().describe(started_at() + Duration::seconds(15));

        assert_eq!(info.name(), "platform-backend");
        assert_eq!(info.version(), "0.1.0");
        assert_eq!(info.rust_version(), Some("1.97.1"));
        assert_eq!(info.environment(), "development");
        assert_eq!(info.uptime_seconds(), 15);
        assert_eq!(info.started_at(), started_at());
    }

    #[test]
    fn rejects_blank_environment() {
        let result = InfoService::new(BuildInfo::new("svc", "0.1.0", None), "   ", started_at());

        assert_eq!(
            result.err(),
            Some(DomainError::EmptyValue {
                field: "environment"
            })
        );
    }

    #[test]
    fn clamps_negative_uptime_to_zero() {
        let info = service().describe(started_at() - Duration::seconds(10));
        assert_eq!(info.uptime_seconds(), 0);
    }
}
