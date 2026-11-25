//! Request-scoped middleware.
//!
//! The middleware assigns a validated request identifier to every inbound
//! request, stores it in request extensions for handlers, echoes it in the
//! response, and makes it available to the tracing span.

use std::time::Duration;

use axum::extract::{Request, State};
use axum::http::{HeaderName, HeaderValue, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use platform_shared::{RequestId, parse_request_id};
use tracing::Instrument;

use crate::http::error::ApiError;
use crate::state::AppState;

/// Canonical header carrying the request identifier.
pub const REQUEST_ID_HEADER: HeaderName = HeaderName::from_static("x-request-id");

/// Request identifier accessible from handlers.
#[derive(Debug, Clone)]
pub struct RequestContext {
    request_id: RequestId,
}

impl RequestContext {
    /// Creates a context for the given identifier.
    #[must_use]
    pub const fn new(request_id: RequestId) -> Self {
        Self { request_id }
    }

    /// The request identifier.
    #[must_use]
    pub const fn request_id(&self) -> &RequestId {
        &self.request_id
    }
}

impl<S> axum::extract::FromRequestParts<S> for RequestContext
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<RequestContext>()
            .cloned()
            .ok_or_else(|| {
                ApiError::internal(std::io::Error::other(
                    "request id middleware is not installed",
                ))
            })
    }
}

/// Assigns a request identifier, propagates it into the span and the response.
///
/// Client supplied identifiers are accepted only when they pass validation;
/// otherwise a fresh identifier is generated. This keeps log correlation usable
/// without trusting arbitrary header content.
pub async fn request_id(
    State(_state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let supplied = request
        .headers()
        .get(&REQUEST_ID_HEADER)
        .and_then(|value| value.to_str().ok());

    let request_id = parse_request_id(supplied);

    request
        .extensions_mut()
        .insert(RequestContext::new(request_id.clone()));

    let span = tracing::info_span!(
        "http.request",
        request_id = %request_id,
        method = %request.method(),
        path = %request.uri().path(),
    );

    let mut response = next.run(request).instrument(span).await;

    if let Ok(value) = HeaderValue::from_str(request_id.as_str()) {
        response.headers_mut().insert(&REQUEST_ID_HEADER, value);
    }

    response
        .headers_mut()
        .append(header::VARY, HeaderValue::from_static("x-request-id"));

    response
}

/// Bounds handler execution and renders a unified timeout response.
pub async fn timeout(duration: Duration, request: Request, next: Next) -> Response {
    match tokio::time::timeout(duration, next.run(request)).await {
        Ok(response) => response,
        Err(_) => {
            tracing::warn!(
                target: "platform.api",
                timeout_ms = u64::try_from(duration.as_millis()).unwrap_or(u64::MAX),
                "request exceeded the server timeout"
            );

            ApiError::request_timeout(duration).into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::middleware::from_fn_with_state;
    use axum::routing::get;
    use platform_shared::MAX_REQUEST_ID_LENGTH;
    use tower::ServiceExt;

    use super::{REQUEST_ID_HEADER, RequestContext, request_id, timeout};
    use crate::state::AppState;
    use crate::testing::test_state;

    fn router(state: AppState) -> Router {
        Router::new()
            .route(
                "/probe",
                get(|context: RequestContext| async move { context.request_id().to_string() }),
            )
            .layer(from_fn_with_state(state.clone(), request_id))
            .with_state(state)
    }

    async fn call(app: Router, supplied: Option<&str>) -> (StatusCode, Option<String>, String) {
        let mut builder = Request::builder().uri("/probe");
        if let Some(value) = supplied {
            builder = builder.header(&REQUEST_ID_HEADER, value);
        }

        let response = app
            .oneshot(builder.body(Body::empty()).unwrap())
            .await
            .unwrap();

        let status = response.status();
        let header = response
            .headers()
            .get(&REQUEST_ID_HEADER)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);

        let body = axum::body::to_bytes(response.into_body(), 64 * 1024)
            .await
            .unwrap();

        (status, header, String::from_utf8_lossy(&body).into_owned())
    }

    #[tokio::test]
    async fn generates_request_id_when_header_is_absent() {
        let (status, header, body) = call(router(test_state()), None).await;

        assert_eq!(status, StatusCode::OK);
        let header = header.expect("response must carry a request id header");
        assert_eq!(body, header, "handler and response must agree");
        assert!(!header.is_empty());
    }

    #[tokio::test]
    async fn preserves_valid_client_request_id() {
        let (_, header, body) = call(router(test_state()), Some("client-req-1")).await;

        assert_eq!(header.as_deref(), Some("client-req-1"));
        assert_eq!(body, "client-req-1");
    }

    #[tokio::test]
    async fn rejects_unsafe_client_request_id() {
        let supplied = "bad<script>alert(1)</script>";
        let (_, header, body) = call(router(test_state()), Some(supplied)).await;

        assert_ne!(header.as_deref(), Some(supplied));
        assert_ne!(body, supplied);
        assert!(header.is_some_and(|value| value.len() >= 32));
    }

    #[tokio::test]
    async fn rejects_overlong_client_request_id() {
        let supplied = "a".repeat(MAX_REQUEST_ID_LENGTH + 1);
        let (_, header, _) = call(router(test_state()), Some(&supplied)).await;

        assert_ne!(header.as_deref(), Some(supplied.as_str()));
    }

    #[tokio::test]
    async fn timeout_middleware_returns_unified_error() {
        async fn slow() -> &'static str {
            tokio::time::sleep(Duration::from_millis(500)).await;
            "late"
        }

        let app = Router::new()
            .route("/slow", get(slow))
            .layer(axum::middleware::from_fn(|request, next| {
                timeout(Duration::from_millis(20), request, next)
            }));

        let response = app
            .oneshot(Request::builder().uri("/slow").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::REQUEST_TIMEOUT);

        let body = axum::body::to_bytes(response.into_body(), 64 * 1024)
            .await
            .unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(payload["success"], false);
        assert_eq!(payload["error"]["code"], "request_timeout");
    }

    #[tokio::test]
    async fn timeout_middleware_passes_fast_responses_through() {
        async fn fast() -> &'static str {
            "ok"
        }

        let app = Router::new()
            .route("/fast", get(fast))
            .layer(axum::middleware::from_fn(|request, next| {
                timeout(Duration::from_secs(1), request, next)
            }));

        let response = app
            .oneshot(Request::builder().uri("/fast").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
