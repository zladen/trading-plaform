//! Readiness use case.
//!
//! Readiness proves that the process can serve meaningful traffic, which means
//! every required infrastructure dependency must answer.

use std::sync::Arc;
use std::time::Duration;

use platform_domain::{Dependency, DependencyCheck, ReadinessReport};
use serde::{Deserialize, Serialize};

use crate::ports::DependencyProbe;

/// Runs dependency probes and aggregates their results.
#[derive(Clone)]
pub struct ReadinessService {
    probes: Vec<Arc<dyn DependencyProbe>>,
    timeout: Duration,
}

impl std::fmt::Debug for ReadinessService {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ReadinessService")
            .field("dependencies", &self.dependencies())
            .field("timeout", &self.timeout)
            .finish()
    }
}

impl ReadinessService {
    /// Creates a readiness service from probes and a per-probe timeout.
    #[must_use]
    pub fn new(probes: Vec<Arc<dyn DependencyProbe>>, timeout: Duration) -> Self {
        Self { probes, timeout }
    }

    /// Dependencies covered by this service, in probe order.
    #[must_use]
    pub fn dependencies(&self) -> Vec<Dependency> {
        self.probes.iter().map(|probe| probe.dependency()).collect()
    }

    /// Runs all probes concurrently and returns the aggregated report.
    ///
    /// Probe failures never abort the run: a failed dependency must surface as a
    /// `down` check so that the caller can see which dependency broke.
    pub async fn check(&self) -> ReadinessReport {
        let checks = futures_util::future::join_all(self.probes.iter().map(|probe| {
            let probe = Arc::clone(probe);
            let timeout = self.timeout;
            async move { probe.probe(timeout).await }
        }))
        .await;

        ReadinessReport::new(checks)
    }
}

/// Serialisable readiness payload for API adapters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadinessPayload {
    /// Whether the service can serve traffic.
    pub ready: bool,
    /// Individual dependency results.
    pub checks: Vec<DependencyCheckPayload>,
}

/// Serialisable single dependency result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyCheckPayload {
    /// Dependency identifier.
    pub dependency: String,
    /// Dependency state (`up` or `down`).
    pub status: String,
    /// Probe duration in milliseconds.
    pub latency_ms: u64,
    /// Failure reason when the probe failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl From<&ReadinessReport> for ReadinessPayload {
    fn from(report: &ReadinessReport) -> Self {
        Self {
            ready: report.is_ready(),
            checks: report
                .checks()
                .iter()
                .map(DependencyCheckPayload::from)
                .collect(),
        }
    }
}

impl From<&DependencyCheck> for DependencyCheckPayload {
    fn from(check: &DependencyCheck) -> Self {
        Self {
            dependency: check.dependency().as_str().to_owned(),
            status: check.state().as_str().to_owned(),
            latency_ms: check.latency_ms(),
            error: check.error().map(str::to_owned),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use async_trait::async_trait;
    use platform_domain::{Dependency, DependencyCheck};

    use super::{ReadinessPayload, ReadinessService};
    use crate::ports::DependencyProbe;

    struct StubProbe {
        dependency: Dependency,
        healthy: bool,
    }

    #[async_trait]
    impl DependencyProbe for StubProbe {
        fn dependency(&self) -> Dependency {
            self.dependency
        }

        async fn probe(&self, _timeout: Duration) -> DependencyCheck {
            if self.healthy {
                DependencyCheck::up(self.dependency, 1)
            } else {
                DependencyCheck::down(self.dependency, 1, "unavailable")
            }
        }
    }

    #[tokio::test]
    async fn reports_ready_when_every_probe_is_up() {
        let service = ReadinessService::new(
            vec![
                std::sync::Arc::new(StubProbe {
                    dependency: Dependency::Postgres,
                    healthy: true,
                }),
                std::sync::Arc::new(StubProbe {
                    dependency: Dependency::Redis,
                    healthy: true,
                }),
            ],
            Duration::from_millis(50),
        );

        let report = service.check().await;
        assert!(report.is_ready());
        assert_eq!(service.dependencies().len(), 2);
    }

    #[tokio::test]
    async fn reports_not_ready_when_a_probe_fails() {
        let service = ReadinessService::new(
            vec![
                std::sync::Arc::new(StubProbe {
                    dependency: Dependency::Postgres,
                    healthy: true,
                }),
                std::sync::Arc::new(StubProbe {
                    dependency: Dependency::Redis,
                    healthy: false,
                }),
            ],
            Duration::from_millis(50),
        );

        let report = service.check().await;
        assert!(!report.is_ready());

        let payload = ReadinessPayload::from(&report);
        assert!(!payload.ready);
        assert_eq!(payload.checks.len(), 2);
        assert_eq!(payload.checks[1].dependency, "redis");
        assert_eq!(payload.checks[1].status, "down");
        assert_eq!(payload.checks[1].error.as_deref(), Some("unavailable"));
    }

    #[test]
    fn debug_lists_dependencies_without_secrets() {
        let service = ReadinessService::new(
            vec![std::sync::Arc::new(StubProbe {
                dependency: Dependency::Postgres,
                healthy: true,
            })],
            Duration::from_millis(10),
        );

        let rendered = format!("{service:?}").to_lowercase();
        assert!(rendered.contains("postgres"));
    }
}
