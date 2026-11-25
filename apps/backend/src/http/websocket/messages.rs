//! WebSocket message contract.
//!
//! Messages are intentionally minimal on V0.1: the transport, heartbeat and
//! lifecycle handling are implemented, while trading and market data events are
//! out of scope. The envelope is versioned through the `type` discriminator so
//! that new variants can be added without breaking existing clients.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Server to client message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Sent immediately after a successful handshake.
    Connected {
        /// Connection identifier assigned by the server.
        connection_id: String,
        /// Protocol version of the message envelope.
        protocol_version: u8,
    },
    /// Periodic heartbeat proving the connection is alive.
    Heartbeat {
        /// Server time in RFC 3339 format.
        server_time: String,
    },
    /// Sent before the server closes the connection.
    Closing {
        /// Human readable reason.
        reason: String,
    },
    /// Result of a client ping request.
    Pong {
        /// Echo of the optional client payload.
        payload: Option<String>,
    },
}

/// Client to server message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Liveness check initiated by the client.
    Ping {
        /// Optional payload echoed back in [`ServerMessage::Pong`].
        payload: Option<String>,
    },
    /// A client acknowledgement of a server heartbeat.
    Pong {},
}

/// Current protocol version of the message envelope.
pub const PROTOCOL_VERSION: u8 = 1;

#[cfg(test)]
mod tests {
    use super::{ClientMessage, PROTOCOL_VERSION, ServerMessage};

    #[test]
    fn server_messages_use_type_discriminator() {
        let message = ServerMessage::Connected {
            connection_id: "abc".to_owned(),
            protocol_version: PROTOCOL_VERSION,
        };

        let rendered = serde_json::to_value(&message).unwrap();
        assert_eq!(rendered["type"], "connected");
        assert_eq!(rendered["connection_id"], "abc");
        assert_eq!(rendered["protocol_version"], PROTOCOL_VERSION);
    }

    #[test]
    fn client_ping_round_trips() {
        let raw = r#"{"type":"ping","payload":"hello"}"#;
        let message: ClientMessage = serde_json::from_str(raw).unwrap();

        assert_eq!(
            message,
            ClientMessage::Ping {
                payload: Some("hello".to_owned())
            }
        );
    }

    #[test]
    fn client_ping_without_payload_is_accepted() {
        let message: ClientMessage = serde_json::from_str(r#"{"type":"ping"}"#).unwrap();
        assert_eq!(message, ClientMessage::Ping { payload: None });
    }

    #[test]
    fn unknown_client_message_is_rejected() {
        let result = serde_json::from_str::<ClientMessage>(r#"{"type":"place_order"}"#);
        assert!(result.is_err(), "trading messages are not part of V0.1");
    }

    #[test]
    fn heartbeat_serialises_its_fields() {
        let message = ServerMessage::Heartbeat {
            server_time: "2026-01-01T00:00:00Z".to_owned(),
        };
        let rendered = serde_json::to_value(&message).unwrap();

        assert_eq!(rendered["type"], "heartbeat");
        assert_eq!(rendered["server_time"], "2026-01-01T00:00:00Z");
    }

    #[test]
    fn closing_message_carries_a_reason() {
        let rendered = serde_json::to_value(ServerMessage::Closing {
            reason: "server shutdown".to_owned(),
        })
        .unwrap();

        assert_eq!(rendered["type"], "closing");
        assert_eq!(rendered["reason"], "server shutdown");
    }
}
