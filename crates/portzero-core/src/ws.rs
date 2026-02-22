//! WebSocket hub for broadcasting real-time events to connected dashboard clients.
//!
//! Uses `tokio::sync::broadcast` to fan out events. Clients subscribe via the API server.

use crate::types::WsEvent;
use tokio::sync::broadcast;

/// Broadcast channel capacity.
const CHANNEL_CAPACITY: usize = 256;

/// Central hub for broadcasting WebSocket events to all connected clients.
#[derive(Debug)]
pub struct WsHub {
    tx: broadcast::Sender<WsEvent>,
}

impl WsHub {
    /// Create a new WebSocket hub.
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(CHANNEL_CAPACITY);
        Self { tx }
    }

    /// Broadcast an event to all connected clients.
    ///
    /// Returns the number of receivers that received the event.
    /// If no clients are connected, returns 0 (not an error).
    pub fn broadcast(&self, event: WsEvent) -> usize {
        match self.tx.send(event) {
            Ok(n) => n,
            Err(_) => {
                // No active receivers -- that's fine, nobody is listening.
                0
            }
        }
    }

    /// Subscribe to events. Returns a receiver that will get all future broadcasts.
    pub fn subscribe(&self) -> broadcast::Receiver<WsEvent> {
        self.tx.subscribe()
    }

    /// Get the current number of active subscribers.
    pub fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
    }
}

impl Default for WsHub {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for WsHub {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_broadcast_no_subscribers() {
        let hub = WsHub::new();
        let count = hub.broadcast(WsEvent::AppRemoved {
            name: "test".to_string(),
        });
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_broadcast_with_subscriber() {
        let hub = WsHub::new();
        let mut rx = hub.subscribe();

        hub.broadcast(WsEvent::AppRemoved {
            name: "test".to_string(),
        });

        let event = rx.recv().await.unwrap();
        match event {
            WsEvent::AppRemoved { name } => assert_eq!(name, "test"),
            _ => panic!("unexpected event type"),
        }
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let hub = WsHub::new();
        let mut rx1 = hub.subscribe();
        let mut rx2 = hub.subscribe();

        assert_eq!(hub.subscriber_count(), 2);

        hub.broadcast(WsEvent::RequestComplete {
            id: "req-1".to_string(),
            app: "my-app".to_string(),
            status_code: 200,
            duration_ms: 42,
        });

        let e1 = rx1.recv().await.unwrap();
        let e2 = rx2.recv().await.unwrap();

        match (&e1, &e2) {
            (
                WsEvent::RequestComplete {
                    id: id1,
                    status_code: s1,
                    ..
                },
                WsEvent::RequestComplete {
                    id: id2,
                    status_code: s2,
                    ..
                },
            ) => {
                assert_eq!(id1, "req-1");
                assert_eq!(id2, "req-1");
                assert_eq!(*s1, 200);
                assert_eq!(*s2, 200);
            }
            _ => panic!("unexpected event types"),
        }
    }
}
