//! Process health use case.
//!
//! Health reports only the state of the current process. It deliberately avoids
//! touching PostgreSQL or Redis so that liveness probes stay cheap and cannot be
//! cascaded into a restart loop by a dependency outage.

use chrono::{DateTime, Utc};
use platform_shared::{BuildInfo, RequestId};
use serde::{Deserialize, Serialize};

/// Reported process status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessStatus {
    /// The process is running and able to serve requests.
    Up,
}

impl ProcessStatus {
    /// Stable identifier used in API responses.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            ProcessStatus::Up => "up",
        }
    }
}

impl std::fmt::Display for ProcessStatus {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Result of a process health check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthReport {
    status: ProcessStatus,
    uptime_seconds: u64,
    checked_at: DateTime<Utc>,
    request_id: RequestId,
}

impl HealthReport {
    /// Reported status.
    #[must_use]
    pub const fn status(&self) -> ProcessStatus {
        self.status
    }

    /// Seconds elapsed since the process started.
    #[must_use]
    pub const fn uptime_seconds(&self) -> u64 {
        self.uptime_seconds
    }

    /// Moment of the check.
    #[must_use]
    pub const fn checked_at(&self) -> DateTime<Utc> {
        self.checked_at
    }

    /// Request identifier of the HTTP request that triggered the check.
    #[must_use]
    pub const fn request_id(&self) -> &RequestId {
        &self.request_id
    }
}

/// Produces [`HealthReport`] values for the running process.
#[derive(Debug, Clone)]
pub struct HealthService {
    build: BuildInfo,
    started_at: DateTime<Utc>,
}

impl HealthService {
    /// Creates a health service bounded to the process start instant.
    #[must_use]
    pub const fn new(build: BuildInfo, started_at: DateTime<Utc>) -> Self {
        Self { build, started_at }
    }

    /// Build metadata of the running process.
    #[must_use]
    pub const fn build(&self) -> &BuildInfo {
        &self.build
    }

    /// Instant the process was started at.
    #[must_use]
    pub const fn started_at(&self) -> DateTime<Utc> {
        self.started_at
    }

    /// Builds a health report for the process at `now`.
    #[must_use]
    pub fn check(&self, now: DateTime<Utc>, request_id: RequestId) -> HealthReport {
        let elapsed = now.signed_duration_since(self.started_at);
        let uptime_seconds = elapsed.num_seconds().max(0).unsigned_abs();

        HealthReport {
            status: ProcessStatus::Up,
            uptime_seconds,
            checked_at: now,
            request_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone, Utc};
    use platform_shared::BuildInfo;

    use super::{HealthService, ProcessStatus};

    fn service() -> HealthService {
        let started_at = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        HealthService::new(BuildInfo::new("svc", "0.1.0", None), started_at)
    }

    #[test]
    fn health_reports_uptime_and_status() {
        let started_at = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let now = started_at + Duration::seconds(90);
        let report = service().check(now, platform_shared::RequestId::parse("req-1").unwrap());

        assert_eq!(report.status(), ProcessStatus::Up);
        assert_eq!(report.uptime_seconds(), 90);
        assert_eq!(report.checked_at(), now);
        assert_eq!(report.request_id().as_str(), "req-1");
    }

    #[test]
    fn health_never_reports_negative_uptime() {
        let started_at = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let now = started_at - Duration::seconds(30);
        let report = service().check(now, platform_shared::RequestId::generate());

        assert_eq!(report.uptime_seconds(), 0);
    }

    #[test]
    fn service_exposes_build_metadata() {
        assert_eq!(service().build().version(), "0.1.0");
        assert_eq!(
            service().started_at(),
            Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
        );
    }
}
