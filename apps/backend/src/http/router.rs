//! Router assembly.
//!
//! Middleware order matters and is chosen deliberately. Outermost to innermost:
//!
//! 1. request id — so every later step and log line can correlate.
//! 2. catch panic — converts panics into unified `500` responses.
//! 3. tracing — logs method, path, status and latency for every request.
//! 4. timeout — bounds handler execution with a unified `408` response.
//! 5. CORS — answers preflight before body limits are applied.
//! 6. body limit — rejects oversized payloads early.
//!
//! `Router::layer` wraps the previous router, so the last call is the outermost
//! layer. The calls below are therefore written innermost first.

use std::time::Duration;

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::http::{HeaderValue, Method, StatusCode, header};
use axum::middleware::{from_fn, from_fn_with_state};
use axum::response::IntoResponse;
use axum::routing::get;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::trace::{DefaultOnFailure, DefaultOnResponse, TraceLayer};
use tracing::Level;
use utoipa_swagger_ui::{Config as SwaggerConfig, SwaggerUi};

use crate::http::error::ApiError;
use crate::http::handlers::system;
use crate::http::middleware::{request_id, timeout};
use crate::http::openapi;
use crate::http::websocket::websocket_handler;
use crate::state::AppState;

/// Path of the OpenAPI document endpoint.
pub const OPENAPI_PATH: &str = "/api/v1/openapi.json";

/// Path of the Swagger UI in development.
pub const SWAGGER_PATH: &str = "/swagger-ui";

/// Upper bound on handler execution.
///
/// Readiness performs two dependency probes with a bounded per-probe timeout and
/// stays well below this value; anything slower indicates a stuck handler.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

/// Builds the request body limit layer from validated configuration.
///
/// Exposed separately because V0.1 has no endpoint that accepts a request body,
/// so the limit can only be verified by applying the production layer to a test
/// route.
pub fn body_limit_layer(config: &crate::config::Config) -> DefaultBodyLimit {
    DefaultBodyLimit::max(config.http.request_body_limit_bytes)
}

/// Builds the full application router.
pub fn build_router(state: AppState) -> Router {
    let config = state.config();

    let cors = build_cors_layer(config);
    let body_limit = body_limit_layer(config);
    let request_timeout = REQUEST_TIMEOUT;

    let api = Router::new()
        .route("/api/v1/health", get(system::health))
        .route("/api/v1/ready", get(system::ready))
        .route("/api/v1/info", get(system::info))
        .route(OPENAPI_PATH, get(serve))
        .route("/api/v1/ws", get(websocket_handler))
        .method_not_allowed_fallback(system::method_not_allowed);

    let mut router: Router<AppState> = Router::new().merge(api).fallback(system::not_found);

    if config.swagger_enabled {
        // Swagger UI reads the document from our own `/api/v1/openapi.json`
        // route instead of registering a second handler for the same path.
        router =
            router.merge(SwaggerUi::new(SWAGGER_PATH).config(SwaggerConfig::new([OPENAPI_PATH])));
    }

    router
        .with_state(state.clone())
        .layer(body_limit)
        .layer(cors)
        .layer(from_fn(move |request, next| {
            timeout(request_timeout, request, next)
        }))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &axum::http::Request<_>| {
                    tracing::info_span!(
                        "http.request",
                        method = %request.method(),
                        path = %request.uri().path(),
                    )
                })
                .on_response(DefaultOnResponse::new().level(Level::INFO))
                .on_failure(DefaultOnFailure::new().level(Level::ERROR)),
        )
        .layer(CatchPanicLayer::new())
        .layer(from_fn_with_state(state, request_id))
}

/// Serves the OpenAPI document.
///
/// The document describes this endpoint too, so generated clients can discover
/// the contract without external documentation.
#[utoipa::path(
    get,
    path = "/api/v1/openapi.json",
    tag = "system",
    operation_id = "openapi",
    summary = "Retrieve the OpenAPI document",
    description = "\
Returns the OpenAPI 3.1 description of this API. It is the same document the \
frontend generates its types from, so it is the authoritative contract rather \
than a hand-maintained summary.",
    responses(
        (status = 200, description = "The OpenAPI document", body = Object),
        (status = 500, description = "The document could not be serialised", body = crate::http::error::ErrorResponse),
    )
)]
pub async fn serve() -> axum::response::Response {
    match openapi::document().to_json() {
        Ok(body) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            body,
        )
            .into_response(),
        Err(error) => {
            tracing::error!(
                target: "platform.api",
                error = %error,
                "failed to serialise the OpenAPI document"
            );

            ApiError::internal(std::io::Error::other(
                "failed to serialise the OpenAPI document",
            ))
            .into_response()
        }
    }
}

fn build_cors_layer(config: &crate::config::Config) -> CorsLayer {
    let origins: Vec<HeaderValue> = config
        .http
        .cors
        .allowed_origins
        .iter()
        .filter_map(|origin| match HeaderValue::from_str(origin) {
            Ok(value) => Some(value),
            Err(error) => {
                tracing::error!(
                    target: "platform.api",
                    origin = %origin,
                    error = %error,
                    "configured CORS origin is not a valid header value"
                );
                None
            }
        })
        .collect();

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::CONTENT_TYPE,
            header::ACCEPT,
            crate::http::middleware::REQUEST_ID_HEADER,
        ])
        .expose_headers([crate::http::middleware::REQUEST_ID_HEADER])
        .max_age(Duration::from_secs(600))
}
