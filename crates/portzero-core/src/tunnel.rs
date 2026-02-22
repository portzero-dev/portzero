//! Tunnel manager: exposes local apps to the internet via public tunnels.
//!
//! This module wraps tunnel connectivity (eventually via `localup-lib`) and
//! provides a clean interface for starting/stopping tunnels per app.
//!
//! # Current Status
//!
//! The `localup-lib` crate is not yet published, so this module implements a
//! **trait-based interface** that can be swapped in when the dependency is available.
//! For now, tunnels are managed as metadata only, with the actual tunnel
//! connection stubbed out.

use crate::types::{TunnelInfo, WsEvent};
use crate::ws::WsHub;
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;
use tracing;

/// Error type for tunnel operations.
#[derive(Debug, thiserror::Error)]
pub enum TunnelError {
    #[error("App '{0}' not found")]
    AppNotFound(String),
    #[error("Tunnel already active for app '{0}'")]
    AlreadyActive(String),
    #[error("No active tunnel for app '{0}'")]
    NotActive(String),
    #[error("Tunnel connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Tunnel not available: localup-lib not integrated")]
    NotAvailable,
}

/// Trait for tunnel backends. This allows swapping in different tunnel
/// implementations (localup-lib, bore, cloudflared, etc.)
#[async_trait::async_trait]
pub trait TunnelBackend: Send + Sync {
    /// Connect to a tunnel relay and expose a local port.
    async fn connect(
        &self,
        local_port: u16,
        subdomain: Option<&str>,
        relay: &str,
    ) -> Result<TunnelConnection, TunnelError>;
}

/// An active tunnel connection handle.
pub struct TunnelConnection {
    /// The public URL assigned by the relay.
    pub public_url: String,
    /// The relay server used.
    pub relay: String,
    /// A handle that keeps the tunnel alive. Drop to disconnect.
    _handle: Box<dyn std::any::Any + Send + Sync>,
}

impl TunnelConnection {
    /// Create a new tunnel connection with the given public URL.
    pub fn new(public_url: String, relay: String) -> Self {
        Self {
            public_url,
            relay,
            _handle: Box::new(()),
        }
    }
}

/// Default tunnel backend that returns NotAvailable.
/// Replace with a real implementation when localup-lib is available.
pub struct StubTunnelBackend;

#[async_trait::async_trait]
impl TunnelBackend for StubTunnelBackend {
    async fn connect(
        &self,
        _local_port: u16,
        _subdomain: Option<&str>,
        _relay: &str,
    ) -> Result<TunnelConnection, TunnelError> {
        Err(TunnelError::NotAvailable)
    }
}

// ---------------------------------------------------------------------------
// LocalUp tunnel backend (real implementation via localup-client)
// ---------------------------------------------------------------------------

/// Real tunnel backend using localup-client.
///
/// Requires a valid JWT auth_token obtained via `portzero login`.
/// Connects to the LocalUp relay server and exposes a local port via HTTPS.
#[cfg(feature = "tunnel")]
pub struct LocalUpBackend {
    /// JWT auth token for the relay.
    auth_token: String,
}

#[cfg(feature = "tunnel")]
impl LocalUpBackend {
    /// Create a new LocalUp backend with the given auth token.
    pub fn new(auth_token: String) -> Self {
        Self { auth_token }
    }
}

#[cfg(feature = "tunnel")]
#[async_trait::async_trait]
impl TunnelBackend for LocalUpBackend {
    async fn connect(
        &self,
        local_port: u16,
        subdomain: Option<&str>,
        relay: &str,
    ) -> Result<TunnelConnection, TunnelError> {
        use localup_client::ExitNodeConfig as LuExitNodeConfig;
        use localup_client::{
            ProtocolConfig as LuProtocolConfig, TunnelClient, TunnelConfig as LuTunnelConfig,
        };

        // Build the localup tunnel config
        let config = LuTunnelConfig::builder()
            .local_host("127.0.0.1".to_string())
            .protocol(LuProtocolConfig::Https {
                local_port,
                subdomain: subdomain.map(|s| s.to_string()),
                custom_domain: None,
            })
            .auth_token(self.auth_token.clone())
            .exit_node(LuExitNodeConfig::Custom(relay.to_string()))
            .build()
            .map_err(|e| TunnelError::ConnectionFailed(format!("Config error: {}", e)))?;

        // Connect to the relay
        let client = TunnelClient::connect(config).await.map_err(|e| {
            let msg = format!("{}", e);
            if msg.contains("Authentication") || msg.contains("auth") {
                TunnelError::ConnectionFailed(
                    "Authentication failed. Run `portzero login` to refresh your credentials."
                        .to_string(),
                )
            } else {
                TunnelError::ConnectionFailed(msg)
            }
        })?;

        let public_url = client.public_url().unwrap_or("unknown").to_string();

        tracing::info!(
            public_url = %public_url,
            relay = %relay,
            local_port = %local_port,
            "LocalUp tunnel connected"
        );

        // The TunnelClient must stay alive to keep the tunnel open.
        // We store it in the _handle field — dropping it disconnects.
        Ok(TunnelConnection {
            public_url,
            relay: relay.to_string(),
            _handle: Box::new(LocalUpHandle::new(client)),
        })
    }
}

/// Handle that keeps a LocalUp tunnel alive.
/// When dropped, sends a graceful disconnect.
#[cfg(feature = "tunnel")]
struct LocalUpHandle {
    /// We use an Option so we can take the client for graceful shutdown.
    /// The Drop impl will abort the background task if it's still running.
    _task: tokio::task::JoinHandle<()>,
}

#[cfg(feature = "tunnel")]
impl LocalUpHandle {
    fn new(client: localup_client::TunnelClient) -> Self {
        // Run the tunnel client in a background task.
        // `client.wait()` consumes the client and runs until disconnect.
        let task = tokio::spawn(async move {
            if let Err(e) = client.wait().await {
                tracing::warn!("LocalUp tunnel closed: {}", e);
            }
        });
        Self { _task: task }
    }
}

#[cfg(feature = "tunnel")]
impl Drop for LocalUpHandle {
    fn drop(&mut self) {
        // Abort the background task — this will close the QUIC connection
        self._task.abort();
    }
}

/// Manages active tunnels for all apps.
pub struct TunnelManager {
    /// Active tunnel info keyed by app name.
    active: DashMap<String, TunnelInfo>,
    /// Active tunnel connections keyed by app name.
    connections: DashMap<String, TunnelConnection>,
    /// The tunnel backend.
    backend: Arc<dyn TunnelBackend>,
    /// Default relay server URL.
    default_relay: String,
    /// WebSocket hub for broadcasting tunnel events.
    ws_hub: Option<WsHub>,
}

impl TunnelManager {
    /// Create a new tunnel manager with the given backend.
    pub fn new(
        backend: Arc<dyn TunnelBackend>,
        default_relay: String,
        ws_hub: Option<WsHub>,
    ) -> Self {
        Self {
            active: DashMap::new(),
            connections: DashMap::new(),
            backend,
            default_relay,
            ws_hub,
        }
    }

    /// Create a tunnel manager with the stub backend (no actual tunnels).
    pub fn stub(ws_hub: Option<WsHub>) -> Self {
        Self::new(
            Arc::new(StubTunnelBackend),
            crate::credentials::DEFAULT_RELAY.to_string(),
            ws_hub,
        )
    }

    /// Create a tunnel manager from a resolved tunnel config.
    ///
    /// Uses `LocalUpBackend` if an auth token is available, otherwise
    /// falls back to `StubTunnelBackend` (tunnels disabled).
    ///
    /// Token sources (in priority order):
    ///   1. `PORTZERO_TUNNEL_TOKEN` env var
    ///   2. `[tunnel] token` in portzero.toml
    ///   3. `~/.portzero/credentials.json` (from `portzero login`)
    #[cfg(feature = "tunnel")]
    pub fn from_resolved_config(
        config: &crate::credentials::ResolvedTunnelConfig,
        ws_hub: Option<WsHub>,
    ) -> Self {
        match config.auth_token {
            Some(ref token) => {
                tracing::info!("Tunnel backend enabled (LocalUp)");
                Self::new(
                    Arc::new(LocalUpBackend::new(token.clone())),
                    config.relay.clone(),
                    ws_hub,
                )
            }
            None => {
                tracing::debug!("No tunnel auth token found. Tunnels disabled.");
                tracing::debug!("To enable: `portzero login`, set PORTZERO_TUNNEL_TOKEN, or add [tunnel] token in portzero.toml");
                Self::new(Arc::new(StubTunnelBackend), config.relay.clone(), ws_hub)
            }
        }
    }

    /// Convenience: create from state_dir + optional config.
    #[cfg(feature = "tunnel")]
    pub fn from_state_dir(
        state_dir: &std::path::Path,
        tunnel_config: Option<&crate::config::TunnelConfig>,
        ws_hub: Option<WsHub>,
    ) -> Self {
        let resolved = crate::credentials::resolve_tunnel_config(state_dir, tunnel_config);
        Self::from_resolved_config(&resolved, ws_hub)
    }

    /// Start a tunnel for an app, exposing the given local port.
    pub async fn share(
        &self,
        app_name: &str,
        port: u16,
        subdomain: Option<&str>,
        relay: Option<&str>,
    ) -> Result<TunnelInfo, TunnelError> {
        if self.active.contains_key(app_name) {
            return Err(TunnelError::AlreadyActive(app_name.to_string()));
        }

        let relay = relay.unwrap_or(&self.default_relay);
        let subdomain = subdomain.or(Some(app_name));

        tracing::info!(
            app = %app_name,
            port = %port,
            relay = %relay,
            subdomain = ?subdomain,
            "Starting tunnel"
        );

        let connection = self.backend.connect(port, subdomain, relay).await?;

        let info = TunnelInfo {
            app_name: app_name.to_string(),
            public_url: connection.public_url.clone(),
            relay: connection.relay.clone(),
            transport: "quic".to_string(),
            started_at: Utc::now(),
        };

        // Broadcast event
        if let Some(ref hub) = self.ws_hub {
            hub.broadcast(WsEvent::TunnelStarted {
                app: app_name.to_string(),
                public_url: info.public_url.clone(),
            });
        }

        self.active.insert(app_name.to_string(), info.clone());
        self.connections.insert(app_name.to_string(), connection);

        tracing::info!(
            app = %app_name,
            public_url = %info.public_url,
            "Tunnel started"
        );

        Ok(info)
    }

    /// Stop a tunnel for an app.
    pub async fn unshare(&self, app_name: &str) -> Result<(), TunnelError> {
        if !self.active.contains_key(app_name) {
            return Err(TunnelError::NotActive(app_name.to_string()));
        }

        // Remove the connection (dropping it disconnects)
        self.connections.remove(app_name);
        self.active.remove(app_name);

        // Broadcast event
        if let Some(ref hub) = self.ws_hub {
            hub.broadcast(WsEvent::TunnelStopped {
                app: app_name.to_string(),
            });
        }

        tracing::info!(app = %app_name, "Tunnel stopped");
        Ok(())
    }

    /// Get info about an active tunnel.
    pub fn get_tunnel(&self, app_name: &str) -> Option<TunnelInfo> {
        self.active.get(app_name).map(|entry| entry.value().clone())
    }

    /// List all active tunnels.
    pub fn list_tunnels(&self) -> Vec<TunnelInfo> {
        self.active.iter().map(|e| e.value().clone()).collect()
    }

    /// Check if a tunnel is active for an app.
    pub fn is_active(&self, app_name: &str) -> bool {
        self.active.contains_key(app_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A mock tunnel backend that always succeeds.
    struct MockTunnelBackend;

    #[async_trait::async_trait]
    impl TunnelBackend for MockTunnelBackend {
        async fn connect(
            &self,
            _local_port: u16,
            subdomain: Option<&str>,
            relay: &str,
        ) -> Result<TunnelConnection, TunnelError> {
            let subdomain = subdomain.unwrap_or("test");
            let public_url = format!("https://{}.{}", subdomain, relay);
            Ok(TunnelConnection::new(public_url, relay.to_string()))
        }
    }

    fn make_manager() -> TunnelManager {
        TunnelManager::new(
            Arc::new(MockTunnelBackend),
            "relay.example.com".to_string(),
            None,
        )
    }

    #[tokio::test]
    async fn test_share_and_unshare() {
        let manager = make_manager();

        let info = manager.share("my-app", 4001, None, None).await.unwrap();
        assert_eq!(info.app_name, "my-app");
        assert_eq!(info.public_url, "https://my-app.relay.example.com");
        assert!(manager.is_active("my-app"));

        manager.unshare("my-app").await.unwrap();
        assert!(!manager.is_active("my-app"));
    }

    #[tokio::test]
    async fn test_share_custom_subdomain() {
        let manager = make_manager();

        let info = manager
            .share("my-app", 4001, Some("custom-name"), None)
            .await
            .unwrap();
        assert_eq!(info.public_url, "https://custom-name.relay.example.com");
    }

    #[tokio::test]
    async fn test_share_custom_relay() {
        let manager = make_manager();

        let info = manager
            .share("my-app", 4001, None, Some("my-relay.example.com"))
            .await
            .unwrap();
        assert_eq!(info.relay, "my-relay.example.com");
        assert_eq!(info.public_url, "https://my-app.my-relay.example.com");
    }

    #[tokio::test]
    async fn test_share_already_active() {
        let manager = make_manager();
        manager.share("my-app", 4001, None, None).await.unwrap();

        let err = manager.share("my-app", 4001, None, None).await.unwrap_err();
        assert!(matches!(err, TunnelError::AlreadyActive(_)));
    }

    #[tokio::test]
    async fn test_unshare_not_active() {
        let manager = make_manager();
        let err = manager.unshare("my-app").await.unwrap_err();
        assert!(matches!(err, TunnelError::NotActive(_)));
    }

    #[tokio::test]
    async fn test_list_tunnels() {
        let manager = make_manager();
        manager.share("app-a", 4001, None, None).await.unwrap();
        manager.share("app-b", 4002, None, None).await.unwrap();

        let tunnels = manager.list_tunnels();
        assert_eq!(tunnels.len(), 2);
    }

    #[tokio::test]
    async fn test_stub_backend_returns_not_available() {
        let manager = TunnelManager::stub(None);
        let err = manager.share("my-app", 4001, None, None).await.unwrap_err();
        assert!(matches!(err, TunnelError::NotAvailable));
    }

    #[tokio::test]
    async fn test_get_tunnel() {
        let manager = make_manager();
        assert!(manager.get_tunnel("my-app").is_none());

        manager.share("my-app", 4001, None, None).await.unwrap();
        let info = manager.get_tunnel("my-app").unwrap();
        assert_eq!(info.app_name, "my-app");
    }
}
