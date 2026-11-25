//! Ports implemented by infrastructure adapters.

use std::time::Duration;

use async_trait::async_trait;
use platform_domain::{Dependency, DependencyCheck};

/// Probes a single infrastructure dependency.
///
/// Implementations live in the infrastructure layer. The trait exists so that
/// readiness checks can be exercised without a live PostgreSQL or Redis server.
#[async_trait]
pub trait DependencyProbe: Send + Sync {
    /// The dependency this probe checks.
    fn dependency(&self) -> Dependency;

    /// Performs the probe.
    ///
    /// Implementations must not panic and must report failures through the
    /// returned [`DependencyCheck`]. A failure to answer within `timeout` counts
    /// as a failed check.
    async fn probe(&self, timeout: Duration) -> DependencyCheck;
}
