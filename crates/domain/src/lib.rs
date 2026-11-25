//! Domain layer for the trading platform.
//!
//! The domain layer contains business models, value objects and rules. It must
//! not depend on HTTP frameworks, database drivers, cache clients or any other
//! infrastructure concern.

pub mod error;
pub mod health;

pub use error::DomainError;
pub use health::{Dependency, DependencyCheck, DependencyState, ReadinessReport, ReadinessState};
