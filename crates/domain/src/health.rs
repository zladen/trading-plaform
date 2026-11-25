//! Dependency health domain model.
//!
//! The model describes the state of the infrastructure the platform depends on.
//! It is transport agnostic; HTTP and other adapters map these types onto their
//! own representations.

use serde::{Deserialize, Serialize};

/// A single component the platform depends on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Dependency {
    /// The PostgreSQL relational database.
    Postgres,
    /// The Redis in-memory store.
    Redis,
}

impl Dependency {
    /// All dependencies that participate in readiness checks.
    pub const ALL: [Dependency; 2] = [Dependency::Postgres, Dependency::Redis];

    /// Stable identifier used in API responses.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Dependency::Postgres => "postgres",
            Dependency::Redis => "redis",
        }
    }
}

impl std::fmt::Display for Dependency {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Outcome of probing a single dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyState {
    /// The dependency answered within the configured timeout.
    Up,
    /// The dependency did not answer or answered with an error.
    Down,
}

impl DependencyState {
    /// Stable identifier used in API responses.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            DependencyState::Up => "up",
            DependencyState::Down => "down",
        }
    }

    /// Returns `true` when the dependency is reachable.
    #[must_use]
    pub const fn is_up(self) -> bool {
        matches!(self, DependencyState::Up)
    }
}

impl std::fmt::Display for DependencyState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Report for a single dependency probe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyCheck {
    dependency: Dependency,
    state: DependencyState,
    latency_ms: u64,
    error: Option<String>,
}

impl DependencyCheck {
    /// Creates a successful check report.
    #[must_use]
    pub const fn up(dependency: Dependency, latency_ms: u64) -> Self {
        Self {
            dependency,
            state: DependencyState::Up,
            latency_ms,
            error: None,
        }
    }

    /// Creates a failed check report with a human readable reason.
    #[must_use]
    pub fn down(dependency: Dependency, latency_ms: u64, error: impl Into<String>) -> Self {
        Self {
            dependency,
            state: DependencyState::Down,
            latency_ms,
            error: Some(error.into()),
        }
    }

    /// The probed dependency.
    #[must_use]
    pub const fn dependency(&self) -> Dependency {
        self.dependency
    }

    /// The probe outcome.
    #[must_use]
    pub const fn state(&self) -> DependencyState {
        self.state
    }

    /// Probe duration in milliseconds.
    #[must_use]
    pub const fn latency_ms(&self) -> u64 {
        self.latency_ms
    }

    /// Failure reason, when the probe failed.
    #[must_use]
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
}

/// Overall readiness state derived from all dependency checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadinessState {
    /// Every dependency is reachable.
    Ready,
    /// At least one dependency is unreachable.
    NotReady,
}

impl ReadinessState {
    /// Stable identifier used in API responses.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            ReadinessState::Ready => "ready",
            ReadinessState::NotReady => "not_ready",
        }
    }
}

impl std::fmt::Display for ReadinessState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Aggregated readiness report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadinessReport {
    state: ReadinessState,
    checks: Vec<DependencyCheck>,
}

impl ReadinessReport {
    /// Builds a report from individual checks.
    ///
    /// The report is ready only when every check reports `up`. An empty check
    /// list is treated as not ready because it cannot prove dependency health.
    #[must_use]
    pub fn new(checks: Vec<DependencyCheck>) -> Self {
        let state = if checks.is_empty() || checks.iter().any(|check| !check.state().is_up()) {
            ReadinessState::NotReady
        } else {
            ReadinessState::Ready
        };

        Self { state, checks }
    }

    /// Overall readiness state.
    #[must_use]
    pub const fn state(&self) -> ReadinessState {
        self.state
    }

    /// Returns `true` when all dependencies are reachable.
    #[must_use]
    pub const fn is_ready(&self) -> bool {
        matches!(self.state, ReadinessState::Ready)
    }

    /// Individual dependency checks.
    #[must_use]
    pub fn checks(&self) -> &[DependencyCheck] {
        &self.checks
    }
}

#[cfg(test)]
mod tests {
    use super::{Dependency, DependencyCheck, DependencyState, ReadinessReport, ReadinessState};

    #[test]
    fn dependency_identifiers_are_stable() {
        assert_eq!(Dependency::Postgres.as_str(), "postgres");
        assert_eq!(Dependency::Redis.as_str(), "redis");
        assert_eq!(Dependency::ALL.len(), 2);
    }

    #[test]
    fn dependency_state_identifiers_are_stable() {
        assert_eq!(DependencyState::Up.as_str(), "up");
        assert_eq!(DependencyState::Down.as_str(), "down");
        assert!(DependencyState::Up.is_up());
        assert!(!DependencyState::Down.is_up());
    }

    #[test]
    fn report_is_ready_when_all_checks_are_up() {
        let report = ReadinessReport::new(vec![
            DependencyCheck::up(Dependency::Postgres, 1),
            DependencyCheck::up(Dependency::Redis, 2),
        ]);

        assert_eq!(report.state(), ReadinessState::Ready);
        assert!(report.is_ready());
        assert_eq!(report.checks().len(), 2);
    }

    #[test]
    fn report_is_not_ready_when_any_check_is_down() {
        let report = ReadinessReport::new(vec![
            DependencyCheck::up(Dependency::Postgres, 1),
            DependencyCheck::down(Dependency::Redis, 2, "connection refused"),
        ]);

        assert_eq!(report.state(), ReadinessState::NotReady);
        assert!(!report.is_ready());
        assert_eq!(
            report.checks()[1].error(),
            Some("connection refused"),
            "failure reason must be preserved"
        );
    }

    #[test]
    fn empty_report_is_not_ready() {
        let report = ReadinessReport::new(Vec::new());
        assert_eq!(report.state(), ReadinessState::NotReady);
    }
}
