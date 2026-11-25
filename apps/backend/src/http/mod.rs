//! HTTP transport layer.

pub mod dto;
pub mod error;
pub mod handlers;
pub mod middleware;
pub mod openapi;
pub mod response;
pub mod router;
pub mod websocket;

pub use router::{OPENAPI_PATH, SWAGGER_PATH, build_router};
