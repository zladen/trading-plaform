//! HTTP data transfer objects.
//!
//! DTOs are intentionally separate from domain and application models: they
//! define the public wire contract documented by OpenAPI.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Health probe payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct HealthDto {
    /// Process status; always `up` for a served response.
    pub status: String,
    /// Seconds the process has been running.
    pub uptime_seconds: u64,
    /// Moment of the check in UTC.
    pub checked_at: DateTime<Utc>,
}

/// Single dependency readiness result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct DependencyCheckDto {
    /// Dependency identifier, for example `postgres`.
    pub dependency: String,
    /// Dependency state: `up` or `down`.
    pub status: String,
    /// Probe duration in milliseconds.
    pub latency_ms: u64,
    /// Failure reason when the probe failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Readiness probe payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ReadyDto {
    /// Overall readiness: `ready` or `not_ready`.
    pub status: String,
    /// Whether every dependency answered.
    pub ready: bool,
    /// Per dependency results.
    pub checks: Vec<DependencyCheckDto>,
    /// Moment of the check in UTC.
    pub checked_at: DateTime<Utc>,
}

/// Service information payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct InfoDto {
    /// Application name.
    pub name: String,
    /// Application version.
    pub version: String,
    /// Rust toolchain used for the build.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rust_version: Option<String>,
    /// Deployment environment.
    pub environment: String,
    /// Instant the process started at.
    pub started_at: DateTime<Utc>,
    /// Seconds the process has been running.
    pub uptime_seconds: u64,
    /// WebSocket endpoint path for live updates.
    pub websocket_path: String,
}
