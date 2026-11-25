//! WebSocket infrastructure.

pub mod handler;
pub mod messages;

pub use handler::websocket_handler;
pub use messages::{ClientMessage, PROTOCOL_VERSION, ServerMessage};
