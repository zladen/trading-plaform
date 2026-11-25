//! Request identifier primitive.
//!
//! Request identifiers correlate log records belonging to a single inbound
//! request. Values are validated before they are trusted, because they are
//! supplied by external clients.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Maximum accepted length of a client supplied request identifier.
pub const MAX_REQUEST_ID_LENGTH: usize = 128;

/// A validated request identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RequestId(String);

impl RequestId {
    /// Generates a fresh unique request identifier.
    #[must_use]
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Parses a request identifier from untrusted input.
    ///
    /// Accepted characters are ASCII alphanumerics, `-`, `_`, `.` and `:`.
    /// Anything else is rejected so that identifiers remain safe to log and to
    /// echo back in response headers.
    pub fn parse(value: &str) -> Result<Self, InvalidRequestId> {
        let trimmed = value.trim();

        if trimmed.is_empty() {
            return Err(InvalidRequestId::Empty);
        }

        if trimmed.chars().count() > MAX_REQUEST_ID_LENGTH {
            return Err(InvalidRequestId::TooLong);
        }

        if !trimmed
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':'))
        {
            return Err(InvalidRequestId::InvalidCharacter);
        }

        Ok(Self(trimmed.to_owned()))
    }

    /// Returns the identifier as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RequestId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Reason a request identifier was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InvalidRequestId {
    /// The value was empty or whitespace only.
    #[error("request id must not be empty")]
    Empty,
    /// The value exceeded [`MAX_REQUEST_ID_LENGTH`].
    #[error("request id must be at most {MAX_REQUEST_ID_LENGTH} characters")]
    TooLong,
    /// The value contained a character outside the allowed set.
    #[error("request id contains characters outside [A-Za-z0-9-_.:]")]
    InvalidCharacter,
}

/// Parses a request identifier, generating a new one when the input is absent.
///
/// Invalid values are replaced rather than propagated: an untrusted header must
/// never be able to fail the request, but it also must not be trusted.
#[must_use]
pub fn parse_request_id(value: Option<&str>) -> RequestId {
    value
        .and_then(|candidate| RequestId::parse(candidate).ok())
        .unwrap_or_else(RequestId::generate)
}

#[cfg(test)]
mod tests {
    use super::{InvalidRequestId, MAX_REQUEST_ID_LENGTH, RequestId, parse_request_id};

    #[test]
    fn accepts_safe_identifier() {
        let id = RequestId::parse("req-1234_ab.cd:ef").expect("identifier should be valid");
        assert_eq!(id.as_str(), "req-1234_ab.cd:ef");
        assert_eq!(id.to_string(), "req-1234_ab.cd:ef");
    }

    #[test]
    fn trims_surrounding_whitespace() {
        let id = RequestId::parse("  abc  ").expect("identifier should be valid");
        assert_eq!(id.as_str(), "abc");
    }

    #[test]
    fn rejects_empty_values() {
        assert_eq!(RequestId::parse("   "), Err(InvalidRequestId::Empty));
    }

    #[test]
    fn rejects_overlong_values() {
        let value = "a".repeat(MAX_REQUEST_ID_LENGTH + 1);
        assert_eq!(RequestId::parse(&value), Err(InvalidRequestId::TooLong));
    }

    #[test]
    fn rejects_unsafe_characters() {
        assert_eq!(
            RequestId::parse("abc\nInjected"),
            Err(InvalidRequestId::InvalidCharacter)
        );
        assert_eq!(
            RequestId::parse("abc<script>"),
            Err(InvalidRequestId::InvalidCharacter)
        );
    }

    #[test]
    fn generated_identifiers_are_unique() {
        let first = RequestId::generate();
        let second = RequestId::generate();
        assert_ne!(first, second);
    }

    #[test]
    fn parse_request_id_falls_back_for_invalid_input() {
        let id = parse_request_id(Some("bad value!"));
        assert!(
            id.as_str().len() >= 32,
            "fallback should be a generated UUID"
        );

        let missing = parse_request_id(None);
        assert!(missing.as_str().len() >= 32);
    }

    #[test]
    fn parse_request_id_keeps_valid_input() {
        assert_eq!(parse_request_id(Some("keep-me")).as_str(), "keep-me");
    }
}
