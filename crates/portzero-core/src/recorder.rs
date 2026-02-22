//! Request/response recorder.
//!
//! Thin wrapper around [`Store`] that handles:
//! - Building a `RequestRecord` from proxy callback data
//! - Async insertion (offloaded to a background task to avoid blocking the proxy)
//! - Eviction of old records beyond the retention limit
//! - Broadcasting `RequestComplete` events via `WsHub`

use crate::store::Store;
use crate::types::{RequestRecord, WsEvent, MAX_BODY_CAPTURE_SIZE};
use crate::ws::WsHub;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

/// Channel capacity for async record insertion.
const RECORD_CHANNEL_CAPACITY: usize = 1024;

/// The recorder collects request/response data and persists it to SQLite.
pub struct Recorder {
    /// Sender side of the background persistence channel.
    tx: mpsc::Sender<RecordMessage>,
    /// WebSocket hub for broadcasting completion events.
    ws_hub: Arc<WsHub>,
}

enum RecordMessage {
    Save(RequestRecord),
    Evict,
}

impl Recorder {
    /// Create a new recorder backed by the given store.
    ///
    /// Spawns a background task that processes insert/evict messages
    /// without blocking the proxy hot path.
    pub fn new(store: Arc<Store>, ws_hub: Arc<WsHub>) -> Self {
        let (tx, rx) = mpsc::channel(RECORD_CHANNEL_CAPACITY);

        // Spawn the background persistence task
        tokio::spawn(Self::persistence_loop(store, rx));

        Self { tx, ws_hub }
    }

    /// Begin recording a new request. Returns a `RecordingSession` that
    /// collects data incrementally and finalizes on `complete()`.
    pub fn start_recording(&self, app_name: &str, method: &str, url: &str, path: &str) -> RecordingSession {
        let id = Uuid::new_v4().to_string();

        self.ws_hub.broadcast(WsEvent::RequestStart {
            id: id.clone(),
            app: app_name.to_string(),
            method: method.to_string(),
            url: url.to_string(),
            timestamp: Utc::now(),
        });

        RecordingSession {
            id,
            app_name: app_name.to_string(),
            timestamp: Utc::now(),
            start_instant: std::time::Instant::now(),

            method: method.to_string(),
            url: url.to_string(),
            path: path.to_string(),
            query_string: String::new(),
            request_headers: HashMap::new(),
            request_body: Vec::new(),
            request_content_type: None,

            status_code: 0,
            response_headers: HashMap::new(),
            response_body: Vec::new(),
            response_content_type: None,

            mocked: false,
            parent_id: None,

            tx: self.tx.clone(),
            ws_hub: self.ws_hub.clone(),
        }
    }

    /// Directly save a fully-built record (e.g. for mock responses).
    pub async fn save(&self, record: RequestRecord) {
        let _ = self.tx.send(RecordMessage::Save(record)).await;
    }

    /// Trigger eviction of old records.
    pub async fn evict(&self) {
        let _ = self.tx.send(RecordMessage::Evict).await;
    }

    /// Background loop that processes persistence messages.
    async fn persistence_loop(store: Arc<Store>, mut rx: mpsc::Receiver<RecordMessage>) {
        let mut insert_count: u64 = 0;

        while let Some(msg) = rx.recv().await {
            match msg {
                RecordMessage::Save(record) => {
                    if let Err(e) = store.insert_request(&record) {
                        tracing::error!(
                            id = %record.id,
                            error = %e,
                            "Failed to persist request record"
                        );
                    }

                    insert_count += 1;

                    // Evict every 100 inserts to keep the DB trimmed
                    if insert_count % 100 == 0 {
                        if let Err(e) = store.evict_old_requests() {
                            tracing::error!(error = %e, "Failed to evict old requests");
                        }
                    }
                }
                RecordMessage::Evict => {
                    if let Err(e) = store.evict_old_requests() {
                        tracing::error!(error = %e, "Failed to evict old requests");
                    }
                }
            }
        }
    }
}

/// An in-progress recording session.
///
/// Collects request/response data incrementally through the Pingora proxy
/// callbacks, then finalizes and persists when `complete()` is called.
pub struct RecordingSession {
    pub id: String,
    pub app_name: String,
    pub timestamp: chrono::DateTime<Utc>,
    start_instant: std::time::Instant,

    // Request data
    pub method: String,
    pub url: String,
    pub path: String,
    pub query_string: String,
    pub request_headers: HashMap<String, String>,
    request_body: Vec<u8>,
    pub request_content_type: Option<String>,

    // Response data
    pub status_code: u16,
    pub response_headers: HashMap<String, String>,
    response_body: Vec<u8>,
    pub response_content_type: Option<String>,

    // Metadata
    pub mocked: bool,
    pub parent_id: Option<String>,

    // Channel to persistence task
    tx: mpsc::Sender<RecordMessage>,
    ws_hub: Arc<WsHub>,
}

impl RecordingSession {
    /// Set the request headers.
    pub fn set_request_headers(&mut self, headers: HashMap<String, String>) {
        self.request_content_type = headers.get("content-type").cloned();
        self.request_headers = headers;
    }

    /// Append request body bytes (capped at MAX_BODY_CAPTURE_SIZE).
    pub fn append_request_body(&mut self, data: &[u8]) {
        if self.request_body.len() < MAX_BODY_CAPTURE_SIZE {
            let remaining = MAX_BODY_CAPTURE_SIZE - self.request_body.len();
            let to_take = data.len().min(remaining);
            self.request_body.extend_from_slice(&data[..to_take]);
        }
    }

    /// Set the response status code.
    pub fn set_response_status(&mut self, status: u16) {
        self.status_code = status;
    }

    /// Set the response headers.
    pub fn set_response_headers(&mut self, headers: HashMap<String, String>) {
        self.response_content_type = headers.get("content-type").cloned();
        self.response_headers = headers;
    }

    /// Append response body bytes (capped at MAX_BODY_CAPTURE_SIZE).
    pub fn append_response_body(&mut self, data: &[u8]) {
        if self.response_body.len() < MAX_BODY_CAPTURE_SIZE {
            let remaining = MAX_BODY_CAPTURE_SIZE - self.response_body.len();
            let to_take = data.len().min(remaining);
            self.response_body.extend_from_slice(&data[..to_take]);
        }
    }

    /// Set the query string.
    pub fn set_query_string(&mut self, qs: String) {
        self.query_string = qs;
    }

    /// Mark as mocked response.
    pub fn set_mocked(&mut self) {
        self.mocked = true;
    }

    /// Finalize the recording: build the `RequestRecord`, send it to
    /// the persistence task, and broadcast a completion event.
    pub async fn complete(self) {
        let duration_ms = self.start_instant.elapsed().as_millis() as u64;

        let record = RequestRecord {
            id: self.id.clone(),
            app_name: self.app_name.clone(),
            timestamp: self.timestamp,
            duration_ms,

            method: self.method.clone(),
            url: self.url.clone(),
            path: self.path.clone(),
            query_string: self.query_string,
            request_headers: self.request_headers,
            request_body: if self.request_body.is_empty() {
                None
            } else {
                Some(self.request_body)
            },
            request_content_type: self.request_content_type,

            status_code: self.status_code,
            status_message: String::new(),
            response_headers: self.response_headers,
            response_body: if self.response_body.is_empty() {
                None
            } else {
                Some(self.response_body)
            },
            response_content_type: self.response_content_type,

            mocked: self.mocked,
            parent_id: self.parent_id,
        };

        // Broadcast completion
        self.ws_hub.broadcast(WsEvent::RequestComplete {
            id: self.id.clone(),
            app: self.app_name.clone(),
            status_code: self.status_code,
            duration_ms,
        });

        // Send to persistence task
        let _ = self.tx.send(RecordMessage::Save(record)).await;
    }

    /// Complete synchronously (non-async, best-effort send).
    pub fn complete_sync(self) {
        let duration_ms = self.start_instant.elapsed().as_millis() as u64;

        let record = RequestRecord {
            id: self.id.clone(),
            app_name: self.app_name.clone(),
            timestamp: self.timestamp,
            duration_ms,

            method: self.method.clone(),
            url: self.url.clone(),
            path: self.path.clone(),
            query_string: self.query_string,
            request_headers: self.request_headers,
            request_body: if self.request_body.is_empty() {
                None
            } else {
                Some(self.request_body)
            },
            request_content_type: self.request_content_type,

            status_code: self.status_code,
            status_message: String::new(),
            response_headers: self.response_headers,
            response_body: if self.response_body.is_empty() {
                None
            } else {
                Some(self.response_body)
            },
            response_content_type: self.response_content_type,

            mocked: self.mocked,
            parent_id: self.parent_id,
        };

        self.ws_hub.broadcast(WsEvent::RequestComplete {
            id: self.id.clone(),
            app: self.app_name.clone(),
            status_code: self.status_code,
            duration_ms,
        });

        // Best-effort send (may fail if channel is full)
        let _ = self.tx.try_send(RecordMessage::Save(record));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Store;

    #[tokio::test]
    async fn test_recording_session_body_capping() {
        let store = Arc::new(Store::in_memory().unwrap());
        let ws_hub = Arc::new(WsHub::new());
        let recorder = Recorder::new(store.clone(), ws_hub);

        let mut session =
            recorder.start_recording("test-app", "POST", "http://test/api", "/api");

        // Append body up to cap
        let chunk = vec![0u8; 100_000];
        for _ in 0..20 {
            session.append_request_body(&chunk);
        }

        // Should be capped at MAX_BODY_CAPTURE_SIZE
        assert!(session.request_body.len() <= MAX_BODY_CAPTURE_SIZE);
    }

    #[tokio::test]
    async fn test_recording_session_complete() {
        let store = Arc::new(Store::in_memory().unwrap());
        let ws_hub = Arc::new(WsHub::new());
        let mut rx = ws_hub.subscribe();
        let recorder = Recorder::new(store.clone(), ws_hub);

        let mut session =
            recorder.start_recording("my-app", "GET", "http://my-app.localhost/api/users", "/api/users");

        // Consume the RequestStart event
        let _start_event = rx.recv().await.unwrap();

        session.set_response_status(200);
        session.set_response_headers(HashMap::from([
            ("content-type".to_string(), "application/json".to_string()),
        ]));
        session.append_response_body(b"{\"users\":[]}");

        session.complete().await;

        // Should have broadcast RequestComplete
        let event = rx.recv().await.unwrap();
        match event {
            WsEvent::RequestComplete {
                app, status_code, ..
            } => {
                assert_eq!(app, "my-app");
                assert_eq!(status_code, 200);
            }
            _ => panic!("expected RequestComplete event"),
        }

        // Give the background task a moment to persist
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Verify it was persisted
        assert_eq!(store.request_count().unwrap(), 1);
    }
}
