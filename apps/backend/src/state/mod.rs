//! Shared application state.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use platform_application::{HealthService, InfoService, ReadinessService};
use platform_shared::BuildInfo;

use crate::config::Config;

/// Immutable state shared by every HTTP and WebSocket handler.
///
/// The state owns use cases only: it never owns database pools or cache clients
/// directly. Those are hidden behind application ports so that handlers cannot
/// reach around the application layer.
#[derive(Clone)]
pub struct AppState {
    config: Arc<Config>,
    build: BuildInfo,
    started_at: DateTime<Utc>,
    health: HealthService,
    info: InfoService,
    readiness: ReadinessService,
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AppState")
            .field("environment", &self.config.environment)
            .field("version", &self.build.version())
            .field("started_at", &self.started_at)
            .finish_non_exhaustive()
    }
}

impl AppState {
    /// Creates the state for a running process.
    #[must_use]
    pub fn new(
        config: Arc<Config>,
        build: BuildInfo,
        started_at: DateTime<Utc>,
        health: HealthService,
        info: InfoService,
        readiness: ReadinessService,
    ) -> Self {
        Self {
            config,
            build,
            started_at,
            health,
            info,
            readiness,
        }
    }

    /// Validated configuration.
    #[must_use]
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Build metadata of the running binary.
    #[must_use]
    pub fn build(&self) -> &BuildInfo {
        &self.build
    }

    /// Instant the process started at.
    #[must_use]
    pub const fn started_at(&self) -> DateTime<Utc> {
        self.started_at
    }

    /// Process health use case.
    #[must_use]
    pub const fn health(&self) -> &HealthService {
        &self.health
    }

    /// Service information use case.
    #[must_use]
    pub const fn info(&self) -> &InfoService {
        &self.info
    }

    /// Dependency readiness use case.
    #[must_use]
    pub const fn readiness(&self) -> &ReadinessService {
        &self.readiness
    }
}
