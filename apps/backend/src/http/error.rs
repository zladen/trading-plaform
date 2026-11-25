//! Unified API error type.
//!
//! Every fallible handler returns [`ApiError`], which renders the same JSON
//! envelope for all failures. Internal details are logged, but never sent to
//! clients.

use axum::Json;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use platform_shared::RequestId;
use serde::Serialize;
use tracing::error;
use utoipa::ToSchema;

/// Machine readable error codes used by API clients.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// The requested resource does not exist.
    NotFound,
    /// The HTTP method is not allowed for this resource.
    MethodNotAllowed,
    /// The request body or parameters failed validation.
    ValidationFailed,
    /// The request body exceeded the configured limit.
    PayloadTooLarge,
    /// The handler did not finish within the configured deadline.
    RequestTimeout,
    /// The dependency could not be reached in time.
    DependencyUnavailable,
    /// An unexpected internal error occurred.
    Internal,
}

impl ErrorCode {
    /// Stable wire representation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            ErrorCode::NotFound => "not_found",
            ErrorCode::MethodNotAllowed => "method_not_allowed",
            ErrorCode::ValidationFailed => "validation_failed",
            ErrorCode::PayloadTooLarge => "payload_too_large",
            ErrorCode::RequestTimeout => "request_timeout",
            ErrorCode::DependencyUnavailable => "dependency_unavailable",
            ErrorCode::Internal => "internal_error",
        }
    }
}

/// Error payload returned to clients.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct ErrorBody {
    /// Stable machine readable code.
    pub code: ErrorCode,
    /// Human readable explanation.
    pub message: String,
    /// Optional field level details.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub details: Vec<FieldError>,
}

/// Field level validation detail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct FieldError {
    /// Name of the offending field.
    pub field: String,
    /// Explanation of the violation.
    pub message: String,
}

/// Unified error envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct ErrorResponse {
    /// Always `false`.
    pub success: bool,
    /// Error payload.
    pub error: ErrorBody,
    /// Request identifier for correlation with server logs.
    pub request_id: String,
}

/// API failure.
#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    body: ErrorBody,
    request_id: Option<RequestId>,
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl ApiError {
    /// Creates a not found error.
    #[must_use]
    pub fn not_found() -> Self {
        Self::new(
            StatusCode::NOT_FOUND,
            ErrorCode::NotFound,
            "the requested resource does not exist",
        )
    }

    /// Creates a method not allowed error.
    #[must_use]
    pub fn method_not_allowed() -> Self {
        Self::new(
            StatusCode::METHOD_NOT_ALLOWED,
            ErrorCode::MethodNotAllowed,
            "the HTTP method is not allowed for this resource",
        )
    }

    /// Creates a payload too large error.
    #[must_use]
    pub fn payload_too_large(limit_bytes: usize) -> Self {
        Self::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            ErrorCode::PayloadTooLarge,
            format!("request body must not exceed {limit_bytes} bytes"),
        )
    }

    /// Creates a validation error with field details.
    #[must_use]
    pub fn validation(details: Vec<FieldError>) -> Self {
        let mut error = Self::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            ErrorCode::ValidationFailed,
            "request validation failed",
        );
        error.body.details = details;
        error
    }

    /// Creates a dependency unavailable error.
    #[must_use]
    pub fn dependency_unavailable(dependency: &str) -> Self {
        Self::new(
            StatusCode::SERVICE_UNAVAILABLE,
            ErrorCode::DependencyUnavailable,
            format!("dependency {dependency} is unavailable"),
        )
    }

    /// Creates a request timeout error.
    #[must_use]
    pub fn request_timeout(duration: std::time::Duration) -> Self {
        Self::new(
            StatusCode::REQUEST_TIMEOUT,
            ErrorCode::RequestTimeout,
            format!(
                "request did not finish within {} ms",
                u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
            ),
        )
    }

    /// Creates an internal error wrapping the original cause.
    ///
    /// The cause is kept for logging only and never serialised.
    #[must_use]
    pub fn internal(source: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Self {
        let mut error = Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            ErrorCode::Internal,
            "an internal error occurred",
        );
        error.source = Some(source.into());
        error
    }

    /// Creates an error with an explicit status and code.
    #[must_use]
    pub fn new(status: StatusCode, code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            status,
            body: ErrorBody {
                code,
                message: message.into(),
                details: Vec::new(),
            },
            request_id: None,
            source: None,
        }
    }

    /// Attaches the request identifier used for correlation.
    #[must_use]
    pub fn with_request_id(mut self, request_id: RequestId) -> Self {
        self.request_id = Some(request_id);
        self
    }

    /// HTTP status code.
    #[must_use]
    pub const fn status(&self) -> StatusCode {
        self.status
    }

    /// Machine readable error code.
    #[must_use]
    pub const fn code(&self) -> ErrorCode {
        self.body.code
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        if self.status.is_server_error() {
            match &self.source {
                Some(source) => error!(
                    target: "platform.api",
                    status = self.status.as_u16(),
                    code = self.body.code.as_str(),
                    error = %source,
                    "request failed"
                ),
                None => error!(
                    target: "platform.api",
                    status = self.status.as_u16(),
                    code = self.body.code.as_str(),
                    "request failed"
                ),
            }
        } else {
            tracing::debug!(
                target: "platform.api",
                status = self.status.as_u16(),
                code = self.body.code.as_str(),
                "request rejected"
            );
        }

        let request_id = self
            .request_id
            .map_or_else(|| "unknown".to_owned(), |id| id.as_str().to_owned());

        let payload = ErrorResponse {
            success: false,
            error: self.body,
            request_id,
        };

        let mut response = (self.status, Json(payload)).into_response();

        if let Ok(value) = HeaderValue::from_str(
            response
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .unwrap_or("application/json"),
        ) {
            response.headers_mut().insert(header::CONTENT_TYPE, value);
        }

        response
    }
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::response::IntoResponse;
    use platform_shared::RequestId;
    use tower::ServiceExt;

    use super::{ApiError, ErrorCode};

    async fn read_json(response: axum::response::Response) -> serde_json::Value {
        let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
            .await
            .expect("response body should be readable");
        serde_json::from_slice(&bytes).expect("response body should be JSON")
    }

    #[tokio::test]
    async fn not_found_renders_unified_envelope() {
        let response = ApiError::not_found()
            .with_request_id(RequestId::parse("req-42").unwrap())
            .into_response();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let payload = read_json(response).await;

        assert_eq!(payload["success"], false);
        assert_eq!(payload["error"]["code"], "not_found");
        assert_eq!(payload["request_id"], "req-42");
    }

    #[tokio::test]
    async fn internal_error_does_not_leak_the_cause() {
        #[derive(Debug, thiserror::Error)]
        #[error("connection string postgres://user:secret@host/db refused")]
        struct Leaky;

        let payload = read_json(ApiError::internal(Leaky).into_response()).await;
        let rendered = payload.to_string();

        assert!(
            !rendered.contains("secret"),
            "cause must not leak: {rendered}"
        );
        assert!(rendered.contains("an internal error occurred"));
    }

    #[tokio::test]
    async fn validation_error_exposes_field_details() {
        let error = ApiError::validation(vec![super::FieldError {
            field: "name".to_owned(),
            message: "must not be empty".to_owned(),
        }]);

        assert_eq!(error.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(error.code(), ErrorCode::ValidationFailed);

        let payload = read_json(error.into_response()).await;
        assert_eq!(payload["error"]["details"][0]["field"], "name");
    }

    #[tokio::test]
    async fn errors_are_json_content_type() {
        let response = ApiError::payload_too_large(1024).into_response();
        let content_type = response
            .headers()
            .get(axum::http::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_owned();

        assert!(content_type.starts_with("application/json"));
    }

    #[tokio::test]
    async fn handler_can_return_api_error() {
        async fn handler() -> Result<&'static str, ApiError> {
            Err(ApiError::method_not_allowed())
        }

        let router = axum::Router::new().route("/x", axum::routing::get(handler));

        let response = router
            .oneshot(Request::builder().uri("/x").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }
}
