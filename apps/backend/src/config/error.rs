//! Configuration errors.

use thiserror::Error;

/// Errors produced while loading or validating configuration.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// A required variable is missing.
    #[error("missing required configuration variable: {0}")]
    Missing(String),

    /// A variable is present but cannot be parsed or is semantically invalid.
    ///
    /// Only the variable name and a description of the problem are carried; the
    /// value itself is never included, so secrets cannot leak into logs.
    #[error("invalid value for configuration variable {key}: {reason}")]
    Invalid {
        /// Environment variable name, never the value.
        key: String,
        /// Explanation of what is wrong.
        reason: String,
    },
}
