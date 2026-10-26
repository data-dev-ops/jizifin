mod common;

use axum::http::StatusCode;
use common::TestClient;
use jizifin_api::events::EventHub;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[test]
fn event_hub_zero_subscribers_safe() {
    let hub = EventHub::default();
    assert_eq!(hub.receiver_count(), 0);
    hub.publish("test.topic");
    assert_eq!(hub.receiver_count(), 0);
}

#[tokio::test]
async fn event_hub_fan_out_multiple_subscribers() {
    let hub = EventHub::default();
    let mut rx1 = hub.subscribe();
    let mut rx2 = hub.subscribe();
    let mut rx3 = hub.subscribe();
    assert_eq!(hub.receiver_count(), 3);

    hub.publish("entries.created");
    assert_eq!(rx1.recv().await.unwrap(), "{\"event\":\"entries.created\"}");
    assert_eq!(rx2.recv().await.unwrap(), "{\"event\":\"entries.created\"}");
    assert_eq!(rx3.recv().await.unwrap(), "{\"event\":\"entries.created\"}");
}

#[tokio::test]
async fn event_hub_subscriber_disconnect() {
    let hub = EventHub::default();
    let mut rx1 = hub.subscribe();
    let rx2 = hub.subscribe();
    assert_eq!(hub.receiver_count(), 2);
    drop(rx2);
    assert_eq!(hub.receiver_count(), 1);

    hub.publish("members.updated");
    assert_eq!(rx1.recv().await.unwrap(), "{\"event\":\"members.updated\"}");
}

#[tokio::test]
async fn event_hub_slow_consumer_lag_recovery() {
    let hub = EventHub::default();
    let mut rx = hub.subscribe();
    for i in 0..300 {
        hub.publish(&format!("event.{}", i));
    }
    let first = rx.recv().await;
    assert!(matches!(first, Err(tokio::sync::broadcast::error::RecvError::Lagged(_))));
    let second = rx.recv().await.unwrap();
    assert!(second.contains("event."));
}

#[tokio::test]
async fn ws_unauthenticated_request_rejected() {
    let client = TestClient::uninitialized().await;
    let resp = client.get("/ws/finance").await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

async fn start_ws_server(client: &TestClient) -> (std::net::SocketAddr, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = jizifin_api::router(client.state.clone());
    let handle = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    (addr, handle)
}

async fn ws_handshake(stream: &mut TcpStream, addr: std::net::SocketAddr, token: &str) {
    let req = format!(
        "GET /ws/finance?token={token} HTTP/1.1\r\n\
         Host: {addr}\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
         Sec-WebSocket-Version: 13\r\n\r\n"
    );
    stream.write_all(req.as_bytes()).await.unwrap();
    let mut buf = [0u8; 1024];
    let n = stream.read(&mut buf).await.unwrap();
    let header = String::from_utf8_lossy(&buf[..n]);
    assert!(header.contains("101 Switching Protocols"));
}

#[tokio::test]
async fn ws_finance_stream_broadcast_and_disconnect() {
    let client = TestClient::authenticated().await;
    let (addr, handle) = start_ws_server(&client).await;
    let mut stream = TcpStream::connect(addr).await.unwrap();
    ws_handshake(&mut stream, addr, &client.token).await;

    client.state.events.publish("entries.created");
    let mut frame = [0u8; 256];
    let n = stream.read(&mut frame).await.unwrap();
    let text = String::from_utf8_lossy(&frame[..n]);
    assert!(text.contains("{\"event\":\"entries.created\"}"));

    drop(stream);
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert_eq!(client.state.events.receiver_count(), 0);
    handle.abort();
}

#[tokio::test]
async fn ws_finance_stream_lag_sends_resync() {
    let client = TestClient::authenticated().await;
    let (addr, handle) = start_ws_server(&client).await;
    let mut stream = TcpStream::connect(addr).await.unwrap();
    ws_handshake(&mut stream, addr, &client.token).await;

    for i in 0..300 {
        client.state.events.publish(&format!("event.{i}"));
    }
    let mut frame = [0u8; 2048];
    let mut got_resync = false;
    for _ in 0..50 {
        if let Ok(n) = stream.read(&mut frame).await {
            let text = String::from_utf8_lossy(&frame[..n]);
            if text.contains("backup.restored") {
                got_resync = true;
                break;
            }
        }
    }
    assert!(got_resync, "Expected backup.restored resync notification on lag");
    handle.abort();
}
