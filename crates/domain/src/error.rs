//! Domain errors.
//!
//! Domain errors describe violations of domain rules. They intentionally carry
//! no transport or storage details.

use thiserror::Error;

/// Errors produced by the domain layer.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum DomainError {
    /// A required value was empty or blank.
    #[error("{field} must not be empty")]
    EmptyValue {
        /// Name of the offending field.
        field: &'static str,
    },

    /// A value violated an upper or lower bound.
    #[error("{field} must be between {min} and {max}")]
    OutOfRange {
        /// Name of the offending field.
        field: &'static str,
        /// Inclusive lower bound.
        min: i64,
        /// Inclusive upper bound.
        max: i64,
    },
}

#[cfg(test)]
mod tests {
    use super::DomainError;

    #[test]
    fn empty_value_error_renders_field_name() {
        let error = DomainError::EmptyValue { field: "name" };
        assert_eq!(error.to_string(), "name must not be empty");
    }

    #[test]
    fn out_of_range_error_renders_bounds() {
        let error = DomainError::OutOfRange {
            field: "port",
            min: 1,
            max: 65535,
        };
        assert_eq!(error.to_string(), "port must be between 1 and 65535");
    }
}
