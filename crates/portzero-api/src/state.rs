//! Shared application state for the API server.

use portzero_core::store::Store;
use portzero_core::types::*;
use portzero_core::ws::WsHub;

use dashmap::DashMap;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

/// Shared state accessible by all route handlers.
#[derive(Clone)]
pub struct AppState {
    /// SQLite store for persisted data (requests, mocks).
    pub store: Arc<Store>,
    /// WebSocket broadcast hub.
    pub ws_hub: Arc<WsHub>,
    /// In-memory app registry (live process data, not persisted).
    pub apps: Arc<DashMap<String, AppInfo>>,
    /// In-memory log buffers per app.
    pub logs: Arc<DashMap<String, VecDeque<LogLine>>>,
    /// In-memory network simulation profiles (keyed by app name).
    pub network_profiles: Arc<DashMap<String, NetworkProfile>>,
    /// In-memory inferred schemas (keyed by app name).
    pub schemas: Arc<DashMap<String, InferredSchema>>,
    /// In-memory active tunnels (keyed by app name).
    #[cfg(feature = "tunnel")]
    pub tunnels: Arc<DashMap<String, TunnelInfo>>,
    /// Daemon start time for uptime calculation.
    pub started_at: Instant,
    /// State directory path (e.g. ~/.portzero) for cert operations.
    pub state_dir: PathBuf,
}

impl AppState {
    /// Create a new `AppState` with the given store and WS hub.
    pub fn new(store: Arc<Store>, ws_hub: Arc<WsHub>) -> Self {
        let state_dir = dirs_next::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".portzero");
        Self::new_with_state_dir(store, ws_hub, state_dir)
    }

    /// Create a new `AppState` with an explicit state directory.
    pub fn new_with_state_dir(store: Arc<Store>, ws_hub: Arc<WsHub>, state_dir: PathBuf) -> Self {
        Self {
            store,
            ws_hub,
            apps: Arc::new(DashMap::new()),
            logs: Arc::new(DashMap::new()),
            network_profiles: Arc::new(DashMap::new()),
            schemas: Arc::new(DashMap::new()),
            #[cfg(feature = "tunnel")]
            tunnels: Arc::new(DashMap::new()),
            started_at: Instant::now(),
            state_dir,
        }
    }

    /// Create a test state with an in-memory store.
    pub fn test() -> Self {
        let store = Arc::new(Store::in_memory().expect("failed to create in-memory store"));
        let ws_hub = Arc::new(WsHub::new());
        let state_dir = std::env::temp_dir().join(format!("portzero-test-{}", std::process::id()));
        Self::new_with_state_dir(store, ws_hub, state_dir)
    }
}
