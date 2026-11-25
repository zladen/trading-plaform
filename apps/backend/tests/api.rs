//! HTTP API integration tests.
//!
//! These tests exercise the real router, the real middleware stack and the real
//! serialisation code. Only infrastructure adapters are replaced with stubs, so
//! the tests are deterministic and need no running PostgreSQL or Redis.

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use platform_backend::http::build_router;
use platform_backend::testing::{StubProbe, test_state, test_state_with};
use platform_domain::Dependency;
use serde_json::Value;
use tower::ServiceExt;

fn router() -> Router {
    build_router(test_state())
}

async fn get(app: &Router, uri: &str) -> (StatusCode, Value, axum::http::HeaderMap) {
    let response = app
        .clone()
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();

    let status = response.status();
    let headers = response.headers().clone();
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();

    let value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, value, headers)
}

#[tokio::test]
async fn health_reports_process_liveness() {
    let (status, payload, headers) = get(&router(), "/api/v1/health").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(payload["success"], true);
    assert_eq!(payload["data"]["status"], "up");
    assert!(payload["data"]["uptime_seconds"].is_u64());
    assert!(payload["data"]["checked_at"].is_string());
    assert!(
        payload["request_id"].is_string(),
        "response must carry a request id"
    );
    assert_eq!(
        headers
            .get("x-request-id")
            .and_then(|value| value.to_str().ok()),
        payload["request_id"].as_str(),
        "header and body must agree"
    );
}

#[tokio::test]
async fn health_does_not_touch_dependencies() {
    // Both probes are down; health must still report `up` because it only
    // describes the process.
    let state = test_state_with(vec![
        std::sync::Arc::new(StubProbe::down(Dependency::Postgres)),
        std::sync::Arc::new(StubProbe::down(Dependency::Redis)),
    ]);

    let (status, payload, _) = get(&build_router(state), "/api/v1/health").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(payload["data"]["status"], "up");
}

#[tokio::test]
async fn ready_reports_ready_when_dependencies_answer() {
    let (status, payload, _) = get(&router(), "/api/v1/ready").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(payload["data"]["ready"], true);
    assert_eq!(payload["data"]["status"], "ready");

    let checks = payload["data"]["checks"].as_array().expect("checks array");
    assert_eq!(checks.len(), 2);
    assert_eq!(checks[0]["dependency"], "postgres");
    assert_eq!(checks[0]["status"], "up");
    assert_eq!(checks[1]["dependency"], "redis");
    assert_eq!(checks[1]["status"], "up");
}

#[tokio::test]
async fn ready_returns_503_when_a_dependency_is_down() {
    let state = test_state_with(vec![
        std::sync::Arc::new(StubProbe::up(Dependency::Postgres)),
        std::sync::Arc::new(StubProbe::down(Dependency::Redis)),
    ]);

    let (status, payload, _) = get(&build_router(state), "/api/v1/ready").await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(payload["data"]["ready"], false);
    assert_eq!(payload["data"]["status"], "not_ready");

    let checks = payload["data"]["checks"].as_array().expect("checks array");
    assert_eq!(checks[1]["dependency"], "redis");
    assert_eq!(checks[1]["status"], "down");
    assert_eq!(checks[1]["error"], "stub dependency is down");
}

#[tokio::test]
async fn info_reports_version_and_environment() {
    let (status, payload, _) = get(&router(), "/api/v1/info").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(payload["data"]["name"], "platform-backend");
    assert_eq!(payload["data"]["version"], "0.1.0-test");
    assert_eq!(payload["data"]["environment"], "test");
    assert_eq!(payload["data"]["websocket_path"], "/api/v1/ws");
    assert_eq!(payload["data"]["rust_version"], "1.97.1");
}

#[tokio::test]
async fn openapi_document_is_served_and_parsable() {
    let (status, payload, headers) = get(&router(), "/api/v1/openapi.json").await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        headers
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("application/json"))
    );

    assert_eq!(payload["openapi"], "3.1.0");
    assert!(payload["info"]["version"].is_string());

    for path in [
        "/api/v1/health",
        "/api/v1/ready",
        "/api/v1/info",
        "/api/v1/openapi.json",
        "/api/v1/ws",
    ] {
        assert!(
            payload["paths"].get(path).is_some(),
            "OpenAPI document must document {path}"
        );
    }

    assert!(
        payload["components"]["schemas"]
            .get("ErrorResponse")
            .is_some()
    );
}

#[tokio::test]
async fn swagger_ui_is_mounted_in_non_production_environments() {
    let app = router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/swagger-ui")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(
        response.status().is_redirection() || response.status().is_success(),
        "Swagger UI should be served, got {}",
        response.status()
    );
}

#[tokio::test]
async fn unknown_route_returns_unified_not_found() {
    let (status, payload, _) = get(&router(), "/api/v1/does-not-exist").await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(payload["success"], false);
    assert_eq!(payload["error"]["code"], "not_found");
}

async fn error_body_for(request: Request<Body>) -> (StatusCode, Value, axum::http::HeaderMap) {
    let response = router().oneshot(request).await.unwrap();

    let status = response.status();
    let headers = response.headers().clone();
    let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .unwrap();

    (status, serde_json::from_slice(&bytes).unwrap(), headers)
}

/// The unifying rule is that the body and the header agree. A caller that
/// correlates a failure by `x-request-id` must never read `unknown` back.
#[tokio::test]
async fn fallback_errors_echo_the_caller_request_id() {
    for (method, uri) in [
        ("GET", "/api/v1/does-not-exist"),
        ("POST", "/api/v1/health"),
    ] {
        let (status, payload, headers) = error_body_for(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("x-request-id", "caller-abc-123")
                .body(Body::empty())
                .unwrap(),
        )
        .await;

        assert!(status.is_client_error(), "{method} {uri} -> {status}");
        assert_eq!(payload["request_id"], "caller-abc-123");
        assert_eq!(
            headers
                .get("x-request-id")
                .and_then(|value| value.to_str().ok()),
            Some("caller-abc-123"),
        );
    }
}

#[tokio::test]
async fn unsupported_method_returns_unified_error() {
    let response = router()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);

    let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .unwrap();
    let payload: Value = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(payload["success"], false);
    assert_eq!(payload["error"]["code"], "method_not_allowed");
}

#[tokio::test]
async fn cors_preflight_is_answered_for_allowed_origin() {
    let response = router()
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/api/v1/health")
                .header(header::ORIGIN, "http://localhost:5173")
                .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .and_then(|value| value.to_str().ok()),
        Some("http://localhost:5173")
    );
}

#[tokio::test]
async fn cors_does_not_echo_unlisted_origin() {
    let response = router()
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/api/v1/health")
                .header(header::ORIGIN, "http://evil.example")
                .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(
        response
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .is_none(),
        "unlisted origins must not be granted access"
    );
}

#[tokio::test]
async fn body_limit_layer_enforces_the_configured_limit() {
    use axum::routing::post;
    use platform_backend::config::Config;
    use platform_backend::http::router::body_limit_layer;

    async fn read_body(body: String) -> String {
        body
    }

    let config = Config {
        http: {
            let mut http = platform_backend::testing::test_config().http;
            http.request_body_limit_bytes = 1024;
            http
        },
        ..platform_backend::testing::test_config()
    };

    let app = Router::new()
        .route("/echo", post(read_body))
        .layer(body_limit_layer(&config));

    // Within the limit.
    let ok = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/echo")
                .body(Body::from("x".repeat(512)))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ok.status(), StatusCode::OK);

    // Above the limit.
    let rejected = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/echo")
                .body(Body::from("x".repeat(4096)))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        rejected.status(),
        StatusCode::PAYLOAD_TOO_LARGE,
        "bodies above the configured limit must be rejected"
    );
}

#[tokio::test]
async fn client_request_id_is_propagated_through_the_api() {
    let response = router()
        .oneshot(
            Request::builder()
                .uri("/api/v1/info")
                .header("x-request-id", "trace-me-123")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response
            .headers()
            .get("x-request-id")
            .and_then(|value| value.to_str().ok()),
        Some("trace-me-123")
    );

    let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .unwrap();
    let payload: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(payload["request_id"], "trace-me-123");
}
