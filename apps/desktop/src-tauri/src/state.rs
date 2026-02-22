use portzero_core::{
    MockEngine, NetworkSim, ProcessManager, Recorder, Router, SchemaInference, Store, WsHub,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::time::Instant;

/// Shared application state, initialized once at startup and injected via `tauri::State`.
pub struct DesktopState {
    pub store: Arc<Store>,
    pub router: Arc<Router>,
    pub ws_hub: Arc<WsHub>,
    pub recorder: Arc<Recorder>,
    pub mock_engine: Arc<MockEngine>,
    pub network_sim: Arc<NetworkSim>,
    pub process_manager: Arc<ProcessManager>,
    pub schema_inference: Arc<SchemaInference>,
    pub started_at: Instant,
    pub state_dir: PathBuf,
    pub proxy_port: u16,
}

impl DesktopState {
    pub fn new(state_dir: &Path) -> anyhow::Result<Self> {
        let proxy_port = portzero_core::DEFAULT_PROXY_PORT;

        // Ensure state directory exists
        std::fs::create_dir_all(state_dir)?;

        let db_path = state_dir.join("portzero.db");
        let store = Arc::new(Store::open(&db_path)?);
        let router = Arc::new(Router::new());
        let ws_hub = Arc::new(WsHub::new());
        let recorder = Arc::new(Recorder::new(store.clone(), ws_hub.clone()));
        let mock_engine = Arc::new(MockEngine::new(Some((*ws_hub).clone())));
        let network_sim = Arc::new(NetworkSim::new());
        let process_manager = Arc::new(ProcessManager::new(
            router.clone(),
            ws_hub.clone(),
            proxy_port,
        ));
        let schema_inference = Arc::new(SchemaInference::new());

        Ok(Self {
            store,
            router,
            ws_hub,
            recorder,
            mock_engine,
            network_sim,
            process_manager,
            schema_inference,
            started_at: Instant::now(),
            state_dir: state_dir.to_path_buf(),
            proxy_port,
        })
    }
}
