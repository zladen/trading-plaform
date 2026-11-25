//! API response envelope.

use serde::Serialize;
use utoipa::ToSchema;

/// Successful response envelope.
///
/// Every successful endpoint returns data inside this envelope so that clients
/// have one shape to unwrap and one place to find the request identifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct SuccessResponse<T> {
    /// Always `true`.
    pub success: bool,
    /// Response payload.
    pub data: T,
    /// Request identifier for correlation with server logs.
    pub request_id: String,
}

impl<T> SuccessResponse<T> {
    /// Wraps a payload with the request identifier.
    #[must_use]
    pub fn new(data: T, request_id: impl Into<String>) -> Self {
        Self {
            success: true,
            data,
            request_id: request_id.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SuccessResponse;

    #[test]
    fn envelope_marks_success_and_keeps_request_id() {
        let response = SuccessResponse::new("payload", "req-7");
        assert!(response.success);
        assert_eq!(response.data, "payload");
        assert_eq!(response.request_id, "req-7");
    }

    #[test]
    fn envelope_serialises_with_expected_keys() {
        let response = SuccessResponse::new(42, "req-8");
        let rendered = serde_json::to_value(&response).unwrap();

        assert_eq!(rendered["success"], true);
        assert_eq!(rendered["data"], 42);
        assert_eq!(rendered["request_id"], "req-8");
    }
}
