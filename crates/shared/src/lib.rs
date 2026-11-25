//! Shared technical primitives used across application layers.
//!
//! Only genuinely cross-cutting, framework agnostic primitives belong here.

pub mod metadata;
pub mod request_id;

pub use metadata::{APP_NAME, BuildInfo, VERSION};
pub use request_id::{MAX_REQUEST_ID_LENGTH, RequestId, parse_request_id};
