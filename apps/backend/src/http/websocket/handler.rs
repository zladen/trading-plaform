//! WebSocket connection lifecycle.
//!
//! The handler owns two independent concerns: reading client frames and writing
//! server frames. They run in separate tasks over a split socket so that a slow
//! client cannot block heartbeats, and so that a write failure is detected even
//! when the client never sends anything.

use std::time::Duration;

use axum::extract::State;
use axum::extract::ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade, close_code};
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use platform_shared::RequestId;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

use crate::http::error::ErrorResponse;
use crate::http::middleware::REQUEST_ID_HEADER;
use crate::http::websocket::messages::{ClientMessage, PROTOCOL_VERSION, ServerMessage};
use crate::state::AppState;

/// Maximum accepted size of a single WebSocket message, in bytes.
pub const MAX_MESSAGE_SIZE_BYTES: usize = 64 * 1024;

/// Maximum number of outbound messages buffered per connection.
///
/// A bounded channel provides backpressure: when a client stops reading, the
/// sender observes a closed channel instead of growing memory without limit.
const OUTBOUND_BUFFER: usize = 32;

/// Upgrades an HTTP request to a WebSocket connection.
#[utoipa::path(
    get,
    path = "/api/v1/ws",
    tag = "websocket",
    operation_id = "websocket",
    summary = "Open a WebSocket connection",
    description = "\
Establishes a WebSocket connection for live updates. On V0.1 the server sends a \
`connected` frame with the connection identifier and the protocol version, then a \
`heartbeat` frame on a fixed interval. Clients may send `{\"type\":\"ping\"}` and \
receive `{\"type\":\"pong\"}`; trading and market data events are not part of V0.1.",
    responses(
        (status = 101, description = "Protocol switch to WebSocket"),
        (status = 400, description = "Malformed upgrade request", body = ErrorResponse),
    )
)]
pub async fn websocket_handler(
    websocket: WebSocketUpgrade,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let request_id = headers
        .get(&REQUEST_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .map_or_else(RequestId::generate, |value| {
            platform_shared::parse_request_id(Some(value))
        });

    websocket
        .max_message_size(MAX_MESSAGE_SIZE_BYTES)
        .max_frame_size(MAX_MESSAGE_SIZE_BYTES)
        .on_upgrade(move |socket| handle_socket(socket, state, request_id))
}

async fn handle_socket(socket: WebSocket, state: AppState, request_id: RequestId) {
    let connection_id = uuid::Uuid::new_v4().to_string();
    let heartbeat_interval = state.config().websocket.heartbeat_interval;

    info!(
        target: "platform.ws",
        request_id = %request_id,
        connection_id = %connection_id,
        "websocket connection established"
    );

    let (mut sink, mut stream) = socket.split();
    let (outbound, mut outbound_rx) = mpsc::channel::<Message>(OUTBOUND_BUFFER);

    if send_json(
        &outbound,
        &ServerMessage::Connected {
            connection_id: connection_id.clone(),
            protocol_version: PROTOCOL_VERSION,
        },
    )
    .await
    .is_err()
    {
        return;
    }

    let writer = tokio::spawn(async move {
        while let Some(message) = outbound_rx.recv().await {
            if sink.send(message).await.is_err() {
                break;
            }
        }

        let _ = sink
            .send(Message::Close(Some(CloseFrame {
                code: close_code::NORMAL,
                reason: "connection closed".into(),
            })))
            .await;
        let _ = sink.close().await;
    });

    let heartbeat = spawn_heartbeat(heartbeat_interval, outbound.clone());

    let reason = read_loop(&mut stream, &outbound, heartbeat_interval).await;

    debug!(
        target: "platform.ws",
        connection_id = %connection_id,
        reason = %reason,
        "websocket read loop finished"
    );

    heartbeat.abort();
    let _ = send_json(
        &outbound,
        &ServerMessage::Closing {
            reason: reason.clone(),
        },
    )
    .await;
    drop(outbound);

    match tokio::time::timeout(Duration::from_secs(5), writer).await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => warn!(
            target: "platform.ws",
            connection_id = %connection_id,
            error = %error,
            "websocket writer task failed"
        ),
        Err(_) => warn!(
            target: "platform.ws",
            connection_id = %connection_id,
            "websocket writer did not finish in time"
        ),
    }

    info!(
        target: "platform.ws",
        connection_id = %connection_id,
        reason = %reason,
        "websocket connection closed"
    );
}

fn spawn_heartbeat(interval: Duration, outbound: mpsc::Sender<Message>) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        ticker.tick().await;

        loop {
            ticker.tick().await;

            let message = ServerMessage::Heartbeat {
                server_time: Utc::now().to_rfc3339(),
            };

            if send_json(&outbound, &message).await.is_err() {
                break;
            }
        }
    })
}

async fn read_loop(
    stream: &mut futures_util::stream::SplitStream<WebSocket>,
    outbound: &mpsc::Sender<Message>,
    idle_timeout: Duration,
) -> String {
    let mut deadline = tokio::time::interval(idle_timeout.saturating_mul(2));
    deadline.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    deadline.tick().await;

    loop {
        tokio::select! {
            biased;

            _ = deadline.tick() => {
                return String::from("heartbeat timeout");
            }

            frame = stream.next() => {
                let Some(frame) = frame else {
                    return String::from("client disconnected");
                };

                match frame {
                    Ok(Message::Text(text)) => {
                        if handle_text(text.as_str(), outbound).await.is_err() {
                            return String::from("outbound channel closed");
                        }
                    }
                    Ok(Message::Binary(_)) => {
                        if send_error(outbound, "binary frames are not supported").await.is_err() {
                            return String::from("outbound channel closed");
                        }
                    }
                    Ok(Message::Ping(payload)) => {
                        // axum answers pings automatically; answering explicitly
                        // keeps the behaviour independent of that default.
                        if outbound.send(Message::Pong(payload)).await.is_err() {
                            return String::from("outbound channel closed");
                        }
                        deadline.reset();
                    }
                    Ok(Message::Pong(_)) => {
                        deadline.reset();
                    }
                    Ok(Message::Close(frame)) => {
                        return frame
                            .map(|frame| format!("client closed: {}", frame.reason))
                            .unwrap_or_else(|| String::from("client closed"));
                    }
                    Err(error) => {
                        warn!(
                            target: "platform.ws",
                            error = %error,
                            "websocket frame error"
                        );
                        return String::from("protocol error");
                    }
                }
            }
        }
    }
}

async fn handle_text(
    text: &str,
    outbound: &mpsc::Sender<Message>,
) -> Result<(), mpsc::error::SendError<Message>> {
    match serde_json::from_str::<ClientMessage>(text) {
        Ok(ClientMessage::Ping { payload }) => {
            send_json(outbound, &ServerMessage::Pong { payload }).await
        }
        Ok(ClientMessage::Pong {}) => Ok(()),
        Err(error) => {
            debug!(
                target: "platform.ws",
                error = %error,
                "client sent an unparsable message"
            );
            send_error(outbound, "unrecognised message").await
        }
    }
}

async fn send_json(
    outbound: &mpsc::Sender<Message>,
    message: &ServerMessage,
) -> Result<(), mpsc::error::SendError<Message>> {
    let payload = serde_json::to_string(message)
        .unwrap_or_else(|_| String::from(r#"{"type":"closing","reason":"serialisation failure"}"#));

    outbound.send(Message::Text(payload.into())).await
}

async fn send_error(
    outbound: &mpsc::Sender<Message>,
    reason: &str,
) -> Result<(), mpsc::error::SendError<Message>> {
    send_json(
        outbound,
        &ServerMessage::Closing {
            reason: reason.to_owned(),
        },
    )
    .await
}
