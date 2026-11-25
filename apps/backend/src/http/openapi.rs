//! OpenAPI document for the public API.
//!
//! The document is the single source of truth for the HTTP contract. The
//! frontend generates its types from it, so hand written duplication of these
//! shapes is not permitted.

use utoipa::OpenApi;
use utoipa::openapi::OpenApi as OpenApiDocument;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::openapi::{Info, Server};

use crate::http::dto::{DependencyCheckDto, HealthDto, InfoDto, ReadyDto};
use crate::http::error::{ErrorBody, ErrorCode, ErrorResponse, FieldError};
use crate::http::handlers::system::{HealthSuccess, InfoSuccess, ReadySuccess};
use crate::http::websocket::messages::{ClientMessage, ServerMessage};

/// Root OpenAPI document.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Platform API",
        version = env!("CARGO_PKG_VERSION"),
        description = "\
Infrastructure API of the automated cryptocurrency trading platform.

V0.1 exposes only system endpoints: process liveness, dependency readiness and \
service information. Trading, exchange integrations and authentication are out of \
scope for this stage.",
        license(name = "Proprietary"),
    ),
    tags(
        (name = "system", description = "Liveness, readiness and service metadata"),
        (name = "websocket", description = "Live updates transport"),
    ),
    paths(
        crate::http::handlers::system::health,
        crate::http::handlers::system::ready,
        crate::http::handlers::system::info,
        crate::http::router::serve,
        crate::http::websocket::handler::websocket_handler,
    ),
    components(
        schemas(
            HealthDto,
            ReadyDto,
            InfoDto,
            DependencyCheckDto,
            HealthSuccess,
            ReadySuccess,
            InfoSuccess,
            ErrorResponse,
            ErrorBody,
            ErrorCode,
            FieldError,
            ServerMessage,
            ClientMessage,
        )
    )
)]
pub struct ApiDoc;

/// Builds the served OpenAPI document with runtime-specific metadata.
pub fn document() -> OpenApiDocument {
    let mut document = ApiDoc::openapi();

    document.info = Info::builder()
        .title("Platform API")
        .version(env!("CARGO_PKG_VERSION"))
        .description(Some(
            "Infrastructure API of the automated cryptocurrency trading platform. \
V0.1 exposes system endpoints only: liveness, readiness and service information.",
        ))
        .license(Some(
            utoipa::openapi::info::License::builder()
                .name("Proprietary")
                .build(),
        ))
        .build();

    document.servers = Some(vec![Server::builder().url("/").build()]);

    // Reserved for the upcoming authentication stage: the scheme is declared so
    // that generated clients retain a stable type, but no route requires it yet.
    if let Some(components) = document.components.as_mut() {
        components.add_security_scheme(
            "bearer_auth",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .description(Some(
                        "Reserved for the authentication stage. No endpoint requires it in V0.1.",
                    ))
                    .build(),
            ),
        );
    }

    document
}

#[cfg(test)]
mod tests {
    use super::document;

    #[test]
    fn document_contains_required_paths() {
        let document = document();
        let paths = &document.paths.paths;

        for path in [
            "/api/v1/health",
            "/api/v1/ready",
            "/api/v1/info",
            "/api/v1/openapi.json",
            "/api/v1/ws",
        ] {
            assert!(
                paths.contains_key(path),
                "OpenAPI document is missing {path}"
            );
        }
    }

    #[test]
    fn document_describes_operations() {
        let document = document();
        let json = document.to_json().expect("document should serialise");

        assert!(
            json.contains("\"operationId\":\"health\""),
            "health operation must be documented"
        );
        assert!(json.contains("\"operationId\":\"ready\""));
        assert!(json.contains("\"operationId\":\"info\""));
    }

    #[test]
    fn document_exposes_component_schemas() {
        let document = document();
        let components = document
            .components
            .as_ref()
            .expect("components must be present");

        for schema in [
            "HealthDto",
            "ReadyDto",
            "InfoDto",
            "DependencyCheckDto",
            "ErrorResponse",
            "ServerMessage",
            "ClientMessage",
        ] {
            assert!(
                components.schemas.contains_key(schema),
                "missing schema {schema}"
            );
        }
    }

    #[test]
    fn document_uses_the_crate_version() {
        let document = document();
        assert_eq!(document.info.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(document.info.title, "Platform API");
    }

    #[test]
    fn document_declares_websocket_path_as_get() {
        let document = document();
        let item = document
            .paths
            .paths
            .get("/api/v1/ws")
            .expect("websocket path must exist");

        assert!(item.get.is_some(), "websocket upgrade is a GET request");
    }
}
