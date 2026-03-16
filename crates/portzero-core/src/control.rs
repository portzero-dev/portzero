//! Control socket for daemon ↔ CLI communication.
//!
//! The daemon listens on a Unix socket (`~/.portzero/portzero.sock`).
//! CLI processes connect to register/deregister apps and query state.
//!
//! Protocol: newline-delimited JSON, one request → one response per line.

use crate::log_store::LogStore;
use crate::mock_engine::MockEngine;
use crate::network_sim::NetworkSim;
use crate::router::Router;
use crate::store::Store;
use crate::tunnel::TunnelManager;
use crate::types::{
    CreateMockRule, LogLine, LogStream, MockRule, NetworkProfile, SetNetworkProfile,
    UpdateMockRule, WsEvent,
};
use crate::ws::WsHub;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

/// Socket filename within the state directory.
const SOCKET_NAME: &str = "portzero.sock";

/// Returns the socket path for a given state directory.
pub fn socket_path(state_dir: &Path) -> PathBuf {
    state_dir.join(SOCKET_NAME)
}

// ---------------------------------------------------------------------------
// Protocol messages
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ControlRequest {
    /// Register an app with the proxy. Sent by CLI after spawning a child.
    #[serde(rename = "register")]
    Register {
        name: String,
        port: u16,
        pid: u32,
        command: Vec<String>,
        cwd: String,
    },

    /// Deregister an app. Sent by CLI on shutdown.
    #[serde(rename = "deregister")]
    Deregister { name: String },

    /// Request a free port for an app name.
    #[serde(rename = "allocate_port")]
    AllocatePort { name: String },

    /// Ping — check if daemon is alive.
    #[serde(rename = "ping")]
    Ping,

    /// List all registered apps.
    #[serde(rename = "list")]
    List,

    /// Append a log line for an app.
    #[serde(rename = "log_append")]
    LogAppend {
        name: String,
        stream: String, // "stdout" or "stderr"
        line: String,
    },

    /// Get logs for an app.
    #[serde(rename = "get_logs")]
    GetLogs { name: String, lines: Option<usize> },

    /// Set a network simulation profile for an app.
    #[serde(rename = "set_network_profile")]
    SetNetworkProfile {
        app_name: String,
        profile: SetNetworkProfile,
    },

    /// Get the network simulation profile for an app.
    #[serde(rename = "get_network_profile")]
    GetNetworkProfile { app_name: String },

    /// Clear the network simulation profile for an app.
    #[serde(rename = "clear_network_profile")]
    ClearNetworkProfile { app_name: String },

    // -- Mocks --
    #[serde(rename = "list_mocks")]
    ListMocks,

    #[serde(rename = "create_mock")]
    CreateMock { rule: CreateMockRule },

    #[serde(rename = "update_mock")]
    UpdateMock { id: String, updates: UpdateMockRule },

    #[serde(rename = "delete_mock")]
    DeleteMock { id: String },

    #[serde(rename = "toggle_mock")]
    ToggleMock { id: String },

    // -- Tunnels --
    /// Start a tunnel for an app.
    #[serde(rename = "share")]
    Share {
        name: String,
        subdomain: Option<String>,
        relay: Option<String>,
    },

    /// Stop a tunnel for an app.
    #[serde(rename = "unshare")]
    Unshare { name: String },

    /// List active tunnels.
    #[serde(rename = "list_tunnels")]
    ListTunnels,

    /// Subscribe to real-time events. The daemon will stream `WsEvent` JSON
    /// lines until the client disconnects. No further request/response
    /// exchanges happen on this connection after subscribing.
    #[serde(rename = "subscribe")]
    Subscribe,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ControlResponse {
    #[serde(rename = "ok")]
    Ok { message: String },

    #[serde(rename = "port")]
    Port { port: u16 },

    #[serde(rename = "error")]
    Error { message: String },

    #[serde(rename = "pong")]
    Pong,

    #[serde(rename = "apps")]
    Apps { apps: Vec<AppEntry> },

    #[serde(rename = "logs")]
    Logs { lines: Vec<LogLine> },

    #[serde(rename = "network_profile")]
    NetworkProfileResp { profile: NetworkProfile },

    #[serde(rename = "mocks")]
    Mocks { mocks: Vec<MockRule> },

    #[serde(rename = "mock")]
    Mock { mock: MockRule },

    #[serde(rename = "tunnel")]
    Tunnel { tunnel: crate::types::TunnelInfo },

    #[serde(rename = "tunnels")]
    Tunnels {
        tunnels: Vec<crate::types::TunnelInfo>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppEntry {
    pub name: String,
    pub port: u16,
    pub pid: u32,
    pub url: String,
    pub command: Vec<String>,
    pub cwd: std::path::PathBuf,
    pub started_at: chrono::DateTime<chrono::Utc>,
}

// ---------------------------------------------------------------------------
// Server (runs in the daemon)
// ---------------------------------------------------------------------------

/// Start the control socket listener. Runs until the listener is dropped.
pub async fn serve_control_socket(
    state_dir: &Path,
    router: Arc<Router>,
    ws_hub: Arc<WsHub>,
    log_store: Arc<LogStore>,
    network_sim: Arc<NetworkSim>,
    mock_engine: Arc<MockEngine>,
    store: Arc<Store>,
    tunnel_manager: Arc<TunnelManager>,
    proxy_port: u16,
) -> anyhow::Result<()> {
    let sock_path = socket_path(state_dir);

    // Remove stale socket file if it exists
    if sock_path.exists() {
        std::fs::remove_file(&sock_path)?;
    }

    let listener = UnixListener::bind(&sock_path)?;
    tracing::info!("Control socket listening on {}", sock_path.display());

    loop {
        match listener.accept().await {
            Ok((stream, _addr)) => {
                let router = router.clone();
                let ws_hub = ws_hub.clone();
                let log_store = log_store.clone();
                let network_sim = network_sim.clone();
                let mock_engine = mock_engine.clone();
                let store = store.clone();
                let tunnel_manager = tunnel_manager.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_client(
                        stream,
                        &router,
                        &ws_hub,
                        &log_store,
                        &network_sim,
                        &mock_engine,
                        &store,
                        &tunnel_manager,
                        proxy_port,
                    )
                    .await
                    {
                        tracing::debug!("Control client error: {e}");
                    }
                });
            }
            Err(e) => {
                tracing::error!("Control socket accept error: {e}");
            }
        }
    }
}

/// Handle a single client connection.
async fn handle_client(
    stream: UnixStream,
    router: &Router,
    ws_hub: &WsHub,
    log_store: &LogStore,
    network_sim: &NetworkSim,
    mock_engine: &MockEngine,
    store: &Store,
    tunnel_manager: &TunnelManager,
    proxy_port: u16,
) -> anyhow::Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();

    while let Some(line) = lines.next_line().await? {
        match serde_json::from_str::<ControlRequest>(&line) {
            Ok(ControlRequest::Subscribe) => {
                // Switch to push mode: stream WsEvents until disconnect
                let mut rx = ws_hub.subscribe();
                // Acknowledge the subscription
                let mut ack = serde_json::to_string(&ControlResponse::Ok {
                    message: "subscribed".to_string(),
                })?;
                ack.push('\n');
                writer.write_all(ack.as_bytes()).await?;
                writer.flush().await?;

                // Stream events until the client disconnects or channel closes
                loop {
                    match rx.recv().await {
                        Ok(event) => {
                            let mut json = serde_json::to_string(&event)?;
                            json.push('\n');
                            if writer.write_all(json.as_bytes()).await.is_err() {
                                break; // Client disconnected
                            }
                            if writer.flush().await.is_err() {
                                break;
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            tracing::debug!("Event subscriber lagged, dropped {n} events");
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            break;
                        }
                    }
                }
                return Ok(()); // Connection is done after subscribe mode
            }
            Ok(req) => {
                let response = process_request(
                    req,
                    router,
                    ws_hub,
                    log_store,
                    network_sim,
                    mock_engine,
                    store,
                    tunnel_manager,
                    proxy_port,
                )
                .await;
                let mut json = serde_json::to_string(&response)?;
                json.push('\n');
                writer.write_all(json.as_bytes()).await?;
            }
            Err(e) => {
                let response = ControlResponse::Error {
                    message: format!("invalid request: {e}"),
                };
                let mut json = serde_json::to_string(&response)?;
                json.push('\n');
                writer.write_all(json.as_bytes()).await?;
            }
        }
    }

    Ok(())
}

/// Process a single control request.
async fn process_request(
    req: ControlRequest,
    router: &Router,
    ws_hub: &WsHub,
    log_store: &LogStore,
    network_sim: &NetworkSim,
    mock_engine: &MockEngine,
    store: &Store,
    tunnel_manager: &TunnelManager,
    proxy_port: u16,
) -> ControlResponse {
    match req {
        ControlRequest::Register {
            name,
            port,
            pid,
            command,
            cwd,
        } => {
            router.register(name.clone(), port, pid, command, PathBuf::from(cwd));

            ws_hub.broadcast(WsEvent::AppRegistered {
                name: name.clone(),
                port,
                pid,
                url: format!("http://{}.localhost:{}", name, proxy_port),
            });

            tracing::info!(name = %name, port = port, pid = pid, "App registered via control socket");

            ControlResponse::Ok {
                message: format!("registered {name}"),
            }
        }

        ControlRequest::Deregister { name } => {
            router.deregister(&name);

            ws_hub.broadcast(WsEvent::AppRemoved { name: name.clone() });

            tracing::info!(name = %name, "App deregistered via control socket");

            ControlResponse::Ok {
                message: format!("deregistered {name}"),
            }
        }

        ControlRequest::AllocatePort { name } => {
            // Return existing port if already allocated for this name
            let port = if let Some(existing) = router.get_port(&name) {
                existing
            } else {
                let port = router.find_free_port(&name);
                // Reserve the port in the router so subsequent calls return the same one
                router.register(
                    name,
                    port,
                    0, // PID not yet known
                    vec![],
                    std::path::PathBuf::new(),
                );
                port
            };
            ControlResponse::Port { port }
        }

        ControlRequest::Ping => ControlResponse::Pong,

        ControlRequest::List => {
            let routes = router.list();
            let apps = routes
                .iter()
                .filter(|r| r.status.is_running())
                .map(|r| AppEntry {
                    name: r.hostname.clone(),
                    port: r.port,
                    pid: r.pid,
                    url: format!("http://{}.localhost:{}", r.hostname, proxy_port),
                    command: r.command.clone(),
                    cwd: r.cwd.clone(),
                    started_at: r.started_at,
                })
                .collect();
            ControlResponse::Apps { apps }
        }

        ControlRequest::LogAppend { name, stream, line } => {
            let log_stream = match stream.as_str() {
                "stderr" => LogStream::Stderr,
                _ => LogStream::Stdout,
            };
            log_store.append(&name, log_stream, line.clone());

            // Also broadcast as a WsEvent so the desktop gets live updates
            ws_hub.broadcast(WsEvent::LogLine {
                app: name,
                stream: log_stream,
                line,
                timestamp: chrono::Utc::now(),
            });

            ControlResponse::Ok {
                message: "logged".to_string(),
            }
        }

        ControlRequest::GetLogs { name, lines } => {
            let log_lines = log_store.get_logs(&name, lines);
            ControlResponse::Logs { lines: log_lines }
        }

        ControlRequest::SetNetworkProfile { app_name, profile } => {
            let np = NetworkProfile {
                app_name: app_name.clone(),
                latency_ms: profile.latency_ms,
                jitter_ms: profile.jitter_ms,
                packet_loss_rate: profile.packet_loss_rate.unwrap_or(0.0),
                bandwidth_limit: profile.bandwidth_limit,
                path_filter: profile.path_filter,
            };
            network_sim.set_profile(np.clone());
            tracing::info!(app = %app_name, "Network profile set via control socket");
            ControlResponse::NetworkProfileResp { profile: np }
        }

        ControlRequest::GetNetworkProfile { app_name } => {
            let profile = network_sim
                .get_profile(&app_name)
                .unwrap_or_else(|| NetworkProfile {
                    app_name,
                    latency_ms: None,
                    jitter_ms: None,
                    packet_loss_rate: 0.0,
                    bandwidth_limit: None,
                    path_filter: None,
                });
            ControlResponse::NetworkProfileResp { profile }
        }

        ControlRequest::ClearNetworkProfile { app_name } => {
            network_sim.clear_profile(&app_name);
            tracing::info!(app = %app_name, "Network profile cleared via control socket");
            ControlResponse::Ok {
                message: format!("cleared network profile for {app_name}"),
            }
        }

        // -- Mocks --
        ControlRequest::ListMocks => ControlResponse::Mocks {
            mocks: mock_engine.list_mocks(),
        },

        ControlRequest::CreateMock { rule } => {
            let mock = mock_engine.add_mock(
                rule.app_name,
                rule.method,
                rule.path_pattern,
                rule.status_code,
                rule.response_headers,
                rule.response_body,
            );
            let _ = store.insert_mock(&mock);
            ControlResponse::Mock { mock }
        }

        ControlRequest::UpdateMock { id, updates } => {
            match mock_engine.update_mock(
                &id,
                updates.method,
                updates.path_pattern,
                updates.status_code,
                updates.response_headers,
                updates.response_body,
                updates.enabled,
            ) {
                Some(mock) => ControlResponse::Mock { mock },
                None => ControlResponse::Error {
                    message: format!("mock '{id}' not found"),
                },
            }
        }

        ControlRequest::DeleteMock { id } => {
            if mock_engine.remove_mock(&id) {
                let _ = store.delete_mock(&id);
                ControlResponse::Ok {
                    message: format!("deleted mock {id}"),
                }
            } else {
                ControlResponse::Error {
                    message: format!("mock '{id}' not found"),
                }
            }
        }

        ControlRequest::ToggleMock { id } => {
            mock_engine.toggle_mock(&id);
            match mock_engine.get_mock(&id) {
                Some(mock) => ControlResponse::Mock { mock },
                None => ControlResponse::Error {
                    message: format!("mock '{id}' not found"),
                },
            }
        }

        // -- Tunnels --
        ControlRequest::Share {
            name,
            subdomain,
            relay,
        } => {
            // Look up the app's port from the router
            let port = router.get_port(&name);
            match port {
                Some(port) => {
                    match tunnel_manager
                        .share(&name, port, subdomain.as_deref(), relay.as_deref())
                        .await
                    {
                        Ok(info) => ControlResponse::Tunnel { tunnel: info },
                        Err(e) => ControlResponse::Error {
                            message: format!("{}", e),
                        },
                    }
                }
                None => ControlResponse::Error {
                    message: format!("App '{}' not found or not running", name),
                },
            }
        }

        ControlRequest::Unshare { name } => match tunnel_manager.unshare(&name).await {
            Ok(()) => ControlResponse::Ok {
                message: format!("tunnel stopped for {name}"),
            },
            Err(e) => ControlResponse::Error {
                message: format!("{}", e),
            },
        },

        ControlRequest::ListTunnels => {
            let tunnels = tunnel_manager.list_tunnels();
            ControlResponse::Tunnels { tunnels }
        }

        // Subscribe is handled at the connection level, not here
        ControlRequest::Subscribe => ControlResponse::Error {
            message: "subscribe must be handled at connection level".to_string(),
        },
    }
}

// ---------------------------------------------------------------------------
// Client (used by CLI)
// ---------------------------------------------------------------------------

/// A client connection to the daemon's control socket.
pub struct ControlClient {
    stream: BufReader<UnixStream>,
}

impl ControlClient {
    /// Connect to the daemon. Returns `None` if the socket doesn't exist or
    /// the daemon isn't running.
    pub async fn connect(state_dir: &Path) -> Option<Self> {
        let path = socket_path(state_dir);
        let stream = UnixStream::connect(&path).await.ok()?;
        Some(Self {
            stream: BufReader::new(stream),
        })
    }

    /// Send a request and read the response.
    pub async fn request(&mut self, req: &ControlRequest) -> anyhow::Result<ControlResponse> {
        let writer = self.stream.get_mut();
        let mut json = serde_json::to_string(req)?;
        json.push('\n');
        writer.write_all(json.as_bytes()).await?;
        writer.flush().await?;

        let mut line = String::new();
        self.stream.read_line(&mut line).await?;
        if line.is_empty() {
            anyhow::bail!("daemon closed connection");
        }
        let resp: ControlResponse = serde_json::from_str(line.trim())?;
        Ok(resp)
    }

    /// Convenience: ping the daemon.
    pub async fn ping(&mut self) -> bool {
        matches!(
            self.request(&ControlRequest::Ping).await,
            Ok(ControlResponse::Pong)
        )
    }

    /// Convenience: allocate a port for an app.
    pub async fn allocate_port(&mut self, name: &str) -> anyhow::Result<u16> {
        match self
            .request(&ControlRequest::AllocatePort {
                name: name.to_string(),
            })
            .await?
        {
            ControlResponse::Port { port } => Ok(port),
            ControlResponse::Error { message } => anyhow::bail!("{message}"),
            other => anyhow::bail!("unexpected response: {:?}", other),
        }
    }

    /// Convenience: register an app.
    pub async fn register(
        &mut self,
        name: &str,
        port: u16,
        pid: u32,
        command: &[String],
        cwd: &Path,
    ) -> anyhow::Result<()> {
        match self
            .request(&ControlRequest::Register {
                name: name.to_string(),
                port,
                pid,
                command: command.to_vec(),
                cwd: cwd.display().to_string(),
            })
            .await?
        {
            ControlResponse::Ok { .. } => Ok(()),
            ControlResponse::Error { message } => anyhow::bail!("{message}"),
            other => anyhow::bail!("unexpected response: {:?}", other),
        }
    }

    /// Convenience: deregister an app.
    pub async fn deregister(&mut self, name: &str) -> anyhow::Result<()> {
        match self
            .request(&ControlRequest::Deregister {
                name: name.to_string(),
            })
            .await?
        {
            ControlResponse::Ok { .. } => Ok(()),
            ControlResponse::Error { message } => anyhow::bail!("{message}"),
            other => anyhow::bail!("unexpected response: {:?}", other),
        }
    }

    /// Convenience: append a log line for an app.
    pub async fn log_append(&mut self, name: &str, stream: &str, line: &str) -> anyhow::Result<()> {
        match self
            .request(&ControlRequest::LogAppend {
                name: name.to_string(),
                stream: stream.to_string(),
                line: line.to_string(),
            })
            .await?
        {
            ControlResponse::Ok { .. } => Ok(()),
            ControlResponse::Error { message } => anyhow::bail!("{message}"),
            other => anyhow::bail!("unexpected response: {:?}", other),
        }
    }

    /// Convenience: get logs for an app.
    pub async fn get_logs(
        &mut self,
        name: &str,
        lines: Option<usize>,
    ) -> anyhow::Result<Vec<LogLine>> {
        match self
            .request(&ControlRequest::GetLogs {
                name: name.to_string(),
                lines,
            })
            .await?
        {
            ControlResponse::Logs { lines } => Ok(lines),
            ControlResponse::Error { message } => anyhow::bail!("{message}"),
            other => anyhow::bail!("unexpected response: {:?}", other),
        }
    }

    /// Convenience: set a network simulation profile for an app.
    pub async fn set_network_profile(
        &mut self,
        app_name: &str,
        profile: SetNetworkProfile,
    ) -> anyhow::Result<NetworkProfile> {
        match self
            .request(&ControlRequest::SetNetworkProfile {
                app_name: app_name.to_string(),
                profile,
            })
            .await?
        {
            ControlResponse::NetworkProfileResp { profile } => Ok(profile),
            ControlResponse::Error { message } => anyhow::bail!("{message}"),
            other => anyhow::bail!("unexpected response: {:?}", other),
        }
    }

    /// Convenience: get the network simulation profile for an app.
    pub async fn get_network_profile(&mut self, app_name: &str) -> anyhow::Result<NetworkProfile> {
        match self
            .request(&ControlRequest::GetNetworkProfile {
                app_name: app_name.to_string(),
            })
            .await?
        {
            ControlResponse::NetworkProfileResp { profile } => Ok(profile),
            ControlResponse::Error { message } => anyhow::bail!("{message}"),
            other => anyhow::bail!("unexpected response: {:?}", other),
        }
    }

    /// Convenience: clear the network simulation profile for an app.
    pub async fn clear_network_profile(&mut self, app_name: &str) -> anyhow::Result<()> {
        match self
            .request(&ControlRequest::ClearNetworkProfile {
                app_name: app_name.to_string(),
            })
            .await?
        {
            ControlResponse::Ok { .. } => Ok(()),
            ControlResponse::Error { message } => anyhow::bail!("{message}"),
            other => anyhow::bail!("unexpected response: {:?}", other),
        }
    }

    // -- Mock convenience methods --

    pub async fn list_mocks(&mut self) -> anyhow::Result<Vec<MockRule>> {
        match self.request(&ControlRequest::ListMocks).await? {
            ControlResponse::Mocks { mocks } => Ok(mocks),
            ControlResponse::Error { message } => anyhow::bail!("{message}"),
            other => anyhow::bail!("unexpected response: {:?}", other),
        }
    }

    pub async fn create_mock(&mut self, rule: CreateMockRule) -> anyhow::Result<MockRule> {
        match self.request(&ControlRequest::CreateMock { rule }).await? {
            ControlResponse::Mock { mock } => Ok(mock),
            ControlResponse::Error { message } => anyhow::bail!("{message}"),
            other => anyhow::bail!("unexpected response: {:?}", other),
        }
    }

    pub async fn update_mock(
        &mut self,
        id: &str,
        updates: UpdateMockRule,
    ) -> anyhow::Result<MockRule> {
        match self
            .request(&ControlRequest::UpdateMock {
                id: id.to_string(),
                updates,
            })
            .await?
        {
            ControlResponse::Mock { mock } => Ok(mock),
            ControlResponse::Error { message } => anyhow::bail!("{message}"),
            other => anyhow::bail!("unexpected response: {:?}", other),
        }
    }

    pub async fn delete_mock(&mut self, id: &str) -> anyhow::Result<()> {
        match self
            .request(&ControlRequest::DeleteMock { id: id.to_string() })
            .await?
        {
            ControlResponse::Ok { .. } => Ok(()),
            ControlResponse::Error { message } => anyhow::bail!("{message}"),
            other => anyhow::bail!("unexpected response: {:?}", other),
        }
    }

    pub async fn toggle_mock(&mut self, id: &str) -> anyhow::Result<MockRule> {
        match self
            .request(&ControlRequest::ToggleMock { id: id.to_string() })
            .await?
        {
            ControlResponse::Mock { mock } => Ok(mock),
            ControlResponse::Error { message } => anyhow::bail!("{message}"),
            other => anyhow::bail!("unexpected response: {:?}", other),
        }
    }

    // -- Tunnel convenience methods --

    /// Start a tunnel for an app.
    pub async fn share(
        &mut self,
        name: &str,
        subdomain: Option<&str>,
        relay: Option<&str>,
    ) -> anyhow::Result<crate::types::TunnelInfo> {
        match self
            .request(&ControlRequest::Share {
                name: name.to_string(),
                subdomain: subdomain.map(|s| s.to_string()),
                relay: relay.map(|s| s.to_string()),
            })
            .await?
        {
            ControlResponse::Tunnel { tunnel } => Ok(tunnel),
            ControlResponse::Error { message } => anyhow::bail!("{message}"),
            other => anyhow::bail!("unexpected response: {:?}", other),
        }
    }

    /// Stop a tunnel for an app.
    pub async fn unshare(&mut self, name: &str) -> anyhow::Result<()> {
        match self
            .request(&ControlRequest::Unshare {
                name: name.to_string(),
            })
            .await?
        {
            ControlResponse::Ok { .. } => Ok(()),
            ControlResponse::Error { message } => anyhow::bail!("{message}"),
            other => anyhow::bail!("unexpected response: {:?}", other),
        }
    }

    /// List active tunnels.
    pub async fn list_tunnels(&mut self) -> anyhow::Result<Vec<crate::types::TunnelInfo>> {
        match self.request(&ControlRequest::ListTunnels).await? {
            ControlResponse::Tunnels { tunnels } => Ok(tunnels),
            ControlResponse::Error { message } => anyhow::bail!("{message}"),
            other => anyhow::bail!("unexpected response: {:?}", other),
        }
    }

    /// Subscribe to real-time events from the daemon.
    ///
    /// Sends the subscribe request, reads the ack, and returns self
    /// for the caller to call `next_event()` in a loop.
    pub async fn subscribe(mut self) -> anyhow::Result<EventSubscription> {
        // Send subscribe request
        let writer = self.stream.get_mut();
        let mut json = serde_json::to_string(&ControlRequest::Subscribe)?;
        json.push('\n');
        writer.write_all(json.as_bytes()).await?;
        writer.flush().await?;

        // Read ack
        let mut line = String::new();
        self.stream.read_line(&mut line).await?;
        if line.is_empty() {
            anyhow::bail!("daemon closed connection");
        }
        // Don't care about parsing the ack — if we got a line, we're subscribed

        Ok(EventSubscription {
            stream: self.stream,
        })
    }
}

/// A live event subscription from the daemon's control socket.
/// Read events with `next_event()` in a loop.
pub struct EventSubscription {
    stream: BufReader<UnixStream>,
}

impl EventSubscription {
    /// Read the next event. Returns `None` if the connection is closed.
    pub async fn next_event(&mut self) -> Option<WsEvent> {
        let mut line = String::new();
        match self.stream.read_line(&mut line).await {
            Ok(0) => None, // EOF
            Ok(_) => serde_json::from_str::<WsEvent>(line.trim()).ok(),
            Err(_) => None,
        }
    }
}
