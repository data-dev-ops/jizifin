//! WebSocket fan-out of ledger change notifications. Clients refetch on each event.

use crate::state::AppState;
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use serde_json::json;
use tokio::sync::broadcast;

/// Events buffered per slow client before it starts missing some.
const BUFFER: usize = 256;

/// Broadcast channel of serialized events.
#[derive(Clone)]
pub struct EventHub {
    sender: broadcast::Sender<String>,
}

impl Default for EventHub {
    fn default() -> Self {
        Self { sender: broadcast::channel(BUFFER).0 }
    }
}

impl EventHub {
    /// Announces a change, e.g. `entries.created`. No receivers is not an error.
    pub fn publish(&self, topic: &str) {
        let _ = self.sender.send(json!({ "event": topic }).to_string());
    }

    /// Subscribes to the broadcast channel.
    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.sender.subscribe()
    }

    /// Number of active subscribers.
    pub fn receiver_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

/// `GET /ws/finance`: upgrade and stream events until the client leaves.
pub async fn socket(State(state): State<AppState>, upgrade: WebSocketUpgrade) -> Response {
    upgrade.on_upgrade(move |ws| stream(ws, state.events.subscribe()))
}

async fn stream(mut ws: WebSocket, mut events: broadcast::Receiver<String>) {
    loop {
        tokio::select! {
            event = events.recv() => if !forward(&mut ws, event).await { break },
            incoming = ws.recv() => if !still_open(incoming) { break },
        }
    }
}

async fn forward(ws: &mut WebSocket, event: Result<String, broadcast::error::RecvError>) -> bool {
    match event {
        Ok(text) => ws.send(Message::Text(text.into())).await.is_ok(),
        Err(broadcast::error::RecvError::Lagged(_)) => {
            let resync = json!({ "event": "backup.restored" }).to_string();
            ws.send(Message::Text(resync.into())).await.is_ok()
        }
        Err(broadcast::error::RecvError::Closed) => false,
    }
}

fn still_open(incoming: Option<Result<Message, axum::Error>>) -> bool {
    matches!(incoming, Some(Ok(message)) if !matches!(message, Message::Close(_)))
}
