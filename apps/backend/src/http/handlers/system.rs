//! System endpoints: health, readiness and service information.

use axum::Json;
use axum::extract::State;
use axum::extract::rejection::QueryRejection;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use chrono::Utc;
use platform_shared::RequestId;
use utoipa::ToSchema;

use crate::http::dto::{DependencyCheckDto, HealthDto, InfoDto, ReadyDto};
use crate::http::error::{ApiError, ErrorResponse, FieldError};
use crate::http::middleware::RequestContext;
use crate::http::response::SuccessResponse;
use crate::state::AppState;

/// WebSocket endpoint published by the API.
pub const WEBSOCKET_PATH: &str = "/api/v1/ws";

/// Health probe: process liveness only.
#[utoipa::path(
    get,
    path = "/api/v1/health",
    tag = "system",
    operation_id = "health",
    summary = "Check process liveness",
    description = "\
Reports whether the current process is able to serve HTTP requests. \
This endpoint does not contact PostgreSQL or Redis and must stay cheap, \
because container orchestrators poll it frequently and treat failures as \
a reason to restart the container.",
    responses(
        (status = 200, description = "The process is running", body = HealthSuccess),
        (status = 500, description = "Unexpected internal error", body = ErrorResponse),
    )
)]
pub async fn health(
    State(state): State<AppState>,
    request_context: RequestContext,
) -> Result<Json<SuccessResponse<HealthDto>>, ApiError> {
    let report = state
        .health()
        .check(Utc::now(), request_context.request_id().clone());

    let payload = HealthDto {
        status: report.status().as_str().to_owned(),
        uptime_seconds: report.uptime_seconds(),
        checked_at: report.checked_at(),
    };

    Ok(Json(SuccessResponse::new(
        payload,
        request_context.request_id().as_str(),
    )))
}

/// Readiness probe: real dependency checks.
#[utoipa::path(
    get,
    path = "/api/v1/ready",
    tag = "system",
    operation_id = "ready",
    summary = "Check dependency readiness",
    description = "\
Verifies that every required infrastructure dependency answers. A `200` response \
means the service is ready to receive traffic; `503` means at least one dependency \
is unreachable and the process must not be reported as ready. The response body \
lists each dependency with its measured latency and failure reason.",
    responses(
        (status = 200, description = "All dependencies are reachable", body = ReadySuccess),
        (status = 503, description = "At least one dependency is unreachable", body = ReadySuccess),
    )
)]
pub async fn ready(
    State(state): State<AppState>,
    request_context: RequestContext,
) -> Result<impl IntoResponse, ApiError> {
    let report = state.readiness().check().await;

    let checks = report
        .checks()
        .iter()
        .map(|check| DependencyCheckDto {
            dependency: check.dependency().as_str().to_owned(),
            status: check.state().as_str().to_owned(),
            latency_ms: check.latency_ms(),
            error: check.error().map(str::to_owned),
        })
        .collect();

    let payload = ReadyDto {
        status: report.state().as_str().to_owned(),
        ready: report.is_ready(),
        checks,
        checked_at: Utc::now(),
    };

    let status = if report.is_ready() {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    Ok((
        status,
        Json(SuccessResponse::new(
            payload,
            request_context.request_id().as_str(),
        )),
    ))
}

/// Service information.
#[utoipa::path(
    get,
    path = "/api/v1/info",
    tag = "system",
    operation_id = "info",
    summary = "Describe the running service",
    description = "\
Returns build metadata, the deployment environment and process uptime. \
Clients use this endpoint to display the backend version and to confirm \
which environment they are connected to.",
    responses(
        (status = 200, description = "Service information", body = InfoSuccess),
        (status = 500, description = "Unexpected internal error", body = ErrorResponse),
    )
)]
pub async fn info(
    State(state): State<AppState>,
    request_context: RequestContext,
) -> Result<Json<SuccessResponse<InfoDto>>, ApiError> {
    let described = state.info().describe(Utc::now());

    let payload = InfoDto {
        name: described.name().to_owned(),
        version: described.version().to_owned(),
        rust_version: described.rust_version().map(str::to_owned),
        environment: described.environment().to_owned(),
        started_at: described.started_at(),
        uptime_seconds: described.uptime_seconds(),
        websocket_path: WEBSOCKET_PATH.to_owned(),
    };

    Ok(Json(SuccessResponse::new(
        payload,
        request_context.request_id().as_str(),
    )))
}

/// Fallback for unknown routes.
///
/// The handler takes the request context so the body carries the same
/// identifier as the response header; otherwise a caller correlating logs with
/// a 404 would see `unknown` in the payload.
pub async fn not_found(request_context: RequestContext) -> ApiError {
    ApiError::not_found().with_request_id(request_id_of(&request_context).clone())
}

/// Fallback for known routes with an unsupported method.
pub async fn method_not_allowed(request_context: RequestContext) -> ApiError {
    ApiError::method_not_allowed().with_request_id(request_id_of(&request_context).clone())
}

/// Converts a query rejection into the unified error envelope.
pub fn query_error(rejection: QueryRejection) -> ApiError {
    ApiError::validation(vec![FieldError {
        field: "query".to_owned(),
        message: rejection.body_text(),
    }])
}

/// OpenAPI schema aliases so that documented responses can reference the
/// generic envelope specialisation without duplicating DTO definitions.
#[derive(Debug, ToSchema)]
#[schema(as = HealthSuccess)]
pub struct HealthSuccess {
    /// Always `true`.
    pub success: bool,
    /// Health payload.
    pub data: HealthDto,
    /// Request identifier.
    pub request_id: String,
}

/// Documented readiness response envelope.
#[derive(Debug, ToSchema)]
#[schema(as = ReadySuccess)]
pub struct ReadySuccess {
    /// Always `true`.
    pub success: bool,
    /// Readiness payload.
    pub data: ReadyDto,
    /// Request identifier.
    pub request_id: String,
}

/// Documented info response envelope.
#[derive(Debug, ToSchema)]
#[schema(as = InfoSuccess)]
pub struct InfoSuccess {
    /// Always `true`.
    pub success: bool,
    /// Service information payload.
    pub data: InfoDto,
    /// Request identifier.
    pub request_id: String,
}

/// Returns the request identifier used by a test or handler context.
#[must_use]
pub fn request_id_of(context: &RequestContext) -> &RequestId {
    context.request_id()
}
