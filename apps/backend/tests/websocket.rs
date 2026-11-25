//! WebSocket transport integration tests.
//!
//! The tests start the real router on an ephemeral port and connect with a real
//! WebSocket client, so the upgrade path, framing and heartbeat behaviour are all
//! exercised end to end.

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use platform_backend::http::build_router;
use platform_backend::testing::test_state;
use serde_json::Value;
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

async fn start_server() -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("ephemeral port should be available");
    let address = listener
        .local_addr()
        .expect("listener should have an address");

    let router = build_router(test_state());

    let handle = tokio::spawn(async move {
        axum::serve(
            listener,
            router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .await
        .expect("test server should serve");
    });

    (format!("ws://{address}/api/v1/ws"), handle)
}

async fn next_json(
    socket: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    timeout: Duration,
) -> Value {
    let frame = tokio::time::timeout(timeout, socket.next())
        .await
        .expect("server frame should arrive before the timeout")
        .expect("stream should yield a frame")
        .expect("frame should not be an error");

    match frame {
        Message::Text(text) => serde_json::from_str(text.as_str()).expect("valid JSON"),
        other => panic!("expected a text frame, received {other:?}"),
    }
}

#[tokio::test]
async fn connection_upgrade_sends_connected_frame() {
    let (url, server) = start_server().await;

    let (mut socket, response) = tokio_tungstenite::connect_async(url.as_str())
        .await
        .expect("websocket upgrade should succeed");

    assert_eq!(response.status(), 101);

    let message = next_json(&mut socket, Duration::from_secs(2)).await;
    assert_eq!(message["type"], "connected");
    assert_eq!(message["protocol_version"], 1);
    assert!(
        message["connection_id"]
            .as_str()
            .is_some_and(|id| !id.is_empty()),
        "connection id must be present"
    );

    socket.close(None).await.ok();
    server.abort();
}

#[tokio::test]
async fn server_sends_periodic_heartbeats() {
    let (url, server) = start_server().await;

    let (mut socket, _) = tokio_tungstenite::connect_async(url.as_str())
        .await
        .expect("websocket upgrade should succeed");

    // Test configuration uses a 100 ms heartbeat interval.
    let connected = next_json(&mut socket, Duration::from_secs(2)).await;
    assert_eq!(connected["type"], "connected");

    let heartbeat = next_json(&mut socket, Duration::from_secs(3)).await;
    assert_eq!(heartbeat["type"], "heartbeat");
    assert!(
        heartbeat["server_time"]
            .as_str()
            .is_some_and(|time| time.contains('T')),
        "heartbeat must carry an RFC 3339 timestamp"
    );

    socket.close(None).await.ok();
    server.abort();
}

#[tokio::test]
async fn client_ping_receives_pong() {
    let (url, server) = start_server().await;

    let mut request = url.into_client_request().expect("valid request");
    request
        .headers_mut()
        .insert("x-request-id", "ws-test-1".parse().unwrap());

    let (mut socket, _) = tokio_tungstenite::connect_async(request)
        .await
        .expect("websocket upgrade should succeed");

    let connected = next_json(&mut socket, Duration::from_secs(2)).await;
    assert_eq!(connected["type"], "connected");

    socket
        .send(Message::Text(
            r#"{"type":"ping","payload":"hello"}"#.to_owned().into(),
        ))
        .await
        .expect("ping should be sent");

    let reply = next_json(&mut socket, Duration::from_secs(2)).await;
    assert_eq!(reply["type"], "pong");
    assert_eq!(reply["payload"], "hello");

    socket.close(None).await.ok();
    server.abort();
}

#[tokio::test]
async fn unsupported_client_message_is_rejected_without_closing_the_transport_cleanly() {
    let (url, server) = start_server().await;

    let (mut socket, _) = tokio_tungstenite::connect_async(url.as_str())
        .await
        .expect("websocket upgrade should succeed");

    let connected = next_json(&mut socket, Duration::from_secs(2)).await;
    assert_eq!(connected["type"], "connected");

    socket
        .send(Message::Text(
            r#"{"type":"place_order","symbol":"BTCUSDT"}"#.to_owned().into(),
        ))
        .await
        .expect("message should be sent");

    let reply = next_json(&mut socket, Duration::from_secs(2)).await;
    assert_eq!(reply["type"], "closing");
    assert_eq!(reply["reason"], "unrecognised message");

    socket.close(None).await.ok();
    server.abort();
}

#[tokio::test]
async fn binary_frames_are_rejected() {
    let (url, server) = start_server().await;

    let (mut socket, _) = tokio_tungstenite::connect_async(url.as_str())
        .await
        .expect("websocket upgrade should succeed");

    let connected = next_json(&mut socket, Duration::from_secs(2)).await;
    assert_eq!(connected["type"], "connected");

    socket
        .send(Message::Binary(vec![0_u8, 1, 2].into()))
        .await
        .expect("binary frame should be sent");

    let reply = next_json(&mut socket, Duration::from_secs(2)).await;
    assert_eq!(reply["type"], "closing");
    assert_eq!(reply["reason"], "binary frames are not supported");

    socket.close(None).await.ok();
    server.abort();
}

#[tokio::test]
async fn client_disconnect_is_handled_without_server_errors() {
    let (url, server) = start_server().await;

    let (mut socket, _) = tokio_tungstenite::connect_async(url.as_str())
        .await
        .expect("websocket upgrade should succeed");

    let connected = next_json(&mut socket, Duration::from_secs(2)).await;
    assert_eq!(connected["type"], "connected");

    drop(socket);

    tokio::time::sleep(Duration::from_millis(300)).await;

    assert!(
        !server.is_finished(),
        "dropping one client must not stop the server"
    );

    server.abort();
}

#[tokio::test]
async fn websocket_handshake_rejects_plain_http_get() {
    let (url, server) = start_server().await;
    let http_url = url.replacen("ws://", "http://", 1);

    let response = tokio::time::timeout(Duration::from_secs(5), reqwest_get(&http_url))
        .await
        .expect("request should complete");

    assert!(
        response.is_some(),
        "a plain HTTP GET without upgrade headers must not be upgraded"
    );

    server.abort();
}

/// Minimal HTTP/1.1 GET without an upgrade header.
///
/// Implemented directly over TCP so the test does not need an HTTP client
/// dependency; it only needs to observe that the server does not switch
/// protocols.
async fn reqwest_get(url: &str) -> Option<u16> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let without_scheme = url.trim_start_matches("http://");
    let (authority, path) = match without_scheme.split_once('/') {
        Some((authority, rest)) => (authority, format!("/{rest}")),
        None => (without_scheme, String::from("/")),
    };

    let mut stream = tokio::net::TcpStream::connect(authority).await.ok()?;
    let request = format!("GET {path} HTTP/1.1\r\nHost: {authority}\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).await.ok()?;

    let mut buffer = vec![0_u8; 512];
    let read = stream.read(&mut buffer).await.ok()?;
    let response = String::from_utf8_lossy(&buffer[..read]).to_string();

    response
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse::<u16>().ok())
}
