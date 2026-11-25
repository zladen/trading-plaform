//! Application layer for the trading platform.
//!
//! The application layer orchestrates use cases and declares the ports that
//! infrastructure adapters implement. It depends on the domain layer only and
//! never on HTTP, SQLx or Redis types.

pub mod health;
pub mod info;
pub mod ports;
pub mod readiness;

pub use health::{HealthReport, HealthService, ProcessStatus};
pub use info::{InfoService, ServiceInfo};
pub use ports::DependencyProbe;
pub use readiness::ReadinessService;
