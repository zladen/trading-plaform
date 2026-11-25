//! Test fixtures.
//!
//! The fixtures build the real router with deterministic settings so that handler
//! and middleware tests exercise production code paths.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::Utc;
use platform_application::{HealthService, InfoService, ReadinessService};
use platform_domain::{Dependency, DependencyCheck};
use platform_shared::BuildInfo;

use crate::config::{
    AppEnvironment, Config, CorsConfig, DatabaseConfig, HttpConfig, LogFormat, LoggingConfig,
    RedisConfig, ShutdownConfig, WebSocketConfig,
};
use crate::state::AppState;

/// A probe with a fixed outcome.
#[derive(Debug, Clone)]
pub struct StubProbe {
    dependency: Dependency,
    healthy: bool,
}

impl StubProbe {
    /// Creates a probe that always reports `up`.
    #[must_use]
    pub const fn up(dependency: Dependency) -> Self {
        Self {
            dependency,
            healthy: true,
        }
    }

    /// Creates a probe that always reports `down`.
    #[must_use]
    pub const fn down(dependency: Dependency) -> Self {
        Self {
            dependency,
            healthy: false,
        }
    }
}

#[async_trait]
impl platform_application::DependencyProbe for StubProbe {
    fn dependency(&self) -> Dependency {
        self.dependency
    }

    async fn probe(&self, _timeout: Duration) -> DependencyCheck {
        if self.healthy {
            DependencyCheck::up(self.dependency, 1)
        } else {
            DependencyCheck::down(self.dependency, 1, "stub dependency is down")
        }
    }
}

/// Deterministic configuration for tests.
#[must_use]
pub fn test_config() -> Config {
    Config {
        environment: AppEnvironment::Test,
        http: HttpConfig {
            host: "127.0.0.1".to_owned(),
            port: 0,
            request_body_limit_bytes: 64 * 1024,
            cors: CorsConfig {
                allowed_origins: vec!["http://localhost:5173".to_owned()],
            },
        },
        logging: LoggingConfig {
            filter: "warn".to_owned(),
            format: LogFormat::Pretty,
        },
        database: DatabaseConfig {
            url: "postgres://platform:password@localhost:5432/platform".to_owned(),
            max_connections: 2,
            min_connections: 0,
            acquire_timeout: Duration::from_millis(200),
            run_migrations: false,
        },
        redis: RedisConfig {
            url: "redis://localhost:6379".to_owned(),
            timeout: Duration::from_millis(200),
        },
        frontend_url: "http://localhost:5173".to_owned(),
        shutdown: ShutdownConfig {
            timeout: Duration::from_secs(1),
        },
        readiness_timeout: Duration::from_millis(200),
        websocket: WebSocketConfig {
            heartbeat_interval: Duration::from_millis(100),
        },
        swagger_enabled: true,
    }
}

/// Application state with healthy dependency probes.
#[must_use]
pub fn test_state() -> AppState {
    test_state_with(vec![
        Arc::new(StubProbe::up(Dependency::Postgres)),
        Arc::new(StubProbe::up(Dependency::Redis)),
    ])
}

/// Application state with the supplied probes.
#[must_use]
pub fn test_state_with(probes: Vec<Arc<dyn platform_application::DependencyProbe>>) -> AppState {
    let config = test_config();
    let started_at = Utc::now();
    let build = BuildInfo::new("platform-backend", "0.1.0-test", Some("1.97.1".to_owned()));

    let health = HealthService::new(build.clone(), started_at);
    let info = InfoService::new(build.clone(), config.environment.as_str(), started_at)
        .expect("test environment name is valid");

    AppState::new(
        Arc::new(config.clone()),
        build,
        started_at,
        health,
        info,
        ReadinessService::new(probes, config.readiness_timeout),
    )
}
