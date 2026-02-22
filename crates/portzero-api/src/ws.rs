//! WebSocket endpoint handler.
//!
//! Connects a dashboard client to the WsHub broadcast channel and forwards
//! events as JSON text frames.

use crate::state::AppState;
use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use tracing;

/// Upgrade HTTP to WebSocket and start streaming events.
pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws_connection(socket, state))
}

async fn handle_ws_connection(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.ws_hub.subscribe();

    tracing::info!(
        "WebSocket client connected (total: {})",
        state.ws_hub.subscriber_count()
    );

    // Spawn a task to forward broadcast events to this WebSocket client.
    let send_task = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    let json = match serde_json::to_string(&event) {
                        Ok(j) => j,
                        Err(e) => {
                            tracing::error!("Failed to serialize WS event: {}", e);
                            continue;
                        }
                    };
                    if sender.send(Message::Text(json.into())).await.is_err() {
                        // Client disconnected
                        break;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("WebSocket client lagged, skipped {} events", n);
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
        }
    });

    // Receive messages from client (we don't expect any, but drain them).
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Close(_)) => break,
            Err(_) => break,
            _ => {} // Ignore text/binary from client
        }
    }

    // Client disconnected — abort the send task.
    send_task.abort();
    tracing::info!("WebSocket client disconnected");
}
