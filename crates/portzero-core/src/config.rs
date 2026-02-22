//! Configuration file loader for `portzero.toml`.

use crate::types::DEFAULT_PROXY_PORT;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Top-level configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub proxy: ProxyConfig,
    #[serde(default)]
    pub tunnel: TunnelConfig,
    #[serde(default)]
    pub apps: HashMap<String, AppConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            proxy: ProxyConfig::default(),
            tunnel: TunnelConfig::default(),
            apps: HashMap::new(),
        }
    }
}

/// Proxy server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// Port to listen on (default: 1337)
    #[serde(default = "default_proxy_port")]
    pub port: u16,
    /// Enable HTTPS with auto-generated certs
    #[serde(default)]
    pub https: bool,
    /// Custom TLS certificate path
    pub cert: Option<PathBuf>,
    /// Custom TLS key path
    pub key: Option<PathBuf>,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            port: DEFAULT_PROXY_PORT,
            https: false,
            cert: None,
            key: None,
        }
    }
}

fn default_proxy_port() -> u16 {
    DEFAULT_PROXY_PORT
}

/// Tunnel configuration for LocalUp integration.
///
/// All fields are optional. They can also be set via environment variables:
///   - `PORTZERO_TUNNEL_TOKEN` overrides `token`
///   - `PORTZERO_TUNNEL_RELAY` overrides `relay`
///
/// Or via `portzero login` (stores credentials in ~/.portzero/credentials.json).
///
/// # Example (portzero.toml)
///
/// ```toml
/// [tunnel]
/// relay = "tunnel.kfs.es:4443"      # Default hosted relay
/// # relay = "my-relay.example.com:4443"  # Self-hosted relay
/// # token = "eyJ..."                     # JWT from `localup generate-token`
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TunnelConfig {
    /// LocalUp relay server address (host:port for the QUIC control plane).
    /// Default: tunnel.kfs.es:4443
    pub relay: Option<String>,
    /// JWT auth token for tunnel connections.
    /// Can be generated with `localup generate-token` for self-hosted relays,
    /// or obtained automatically via `portzero login` for the hosted service.
    pub token: Option<String>,
    /// Transport protocol: quic, websocket, h2 (default: auto-detect)
    pub transport: Option<String>,
}

/// Per-app configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Command to run (e.g. "pnpm dev")
    pub command: String,
    /// Working directory (relative to config file)
    pub cwd: Option<PathBuf>,
    /// Custom subdomain (defaults to app key name)
    pub subdomain: Option<String>,
    /// Auto-restart on crash
    #[serde(default)]
    pub auto_restart: bool,
    /// Environment variables
    #[serde(default)]
    pub env: HashMap<String, String>,
}

impl Config {
    /// Load configuration from the given path.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read config file {}: {}", path.display(), e))?;
        let config: Config = toml::from_str(&content).map_err(|e| {
            anyhow::anyhow!("Failed to parse config file {}: {}", path.display(), e)
        })?;
        Ok(config)
    }

    /// Try to find and load a config file by searching upward from the given directory.
    /// Looks for `portzero.toml` in the given dir and its parents.
    pub fn discover(start_dir: &Path) -> Option<(PathBuf, Config)> {
        let mut dir = start_dir.to_path_buf();
        loop {
            let candidate = dir.join("portzero.toml");
            if candidate.exists() {
                match Config::load(&candidate) {
                    Ok(config) => return Some((candidate, config)),
                    Err(e) => {
                        tracing::warn!(
                            "Found config at {} but failed to parse: {}",
                            candidate.display(),
                            e
                        );
                        return None;
                    }
                }
            }
            if !dir.pop() {
                return None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_config() {
        let toml = r#"
[proxy]
port = 8080
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.proxy.port, 8080);
        assert!(!config.proxy.https);
        assert!(config.apps.is_empty());
    }

    #[test]
    fn test_parse_full_config() {
        let toml = r#"
[proxy]
port = 1337
https = true

[tunnel]
relay = "relay.portzero.dev:4443"

[apps.web]
command = "pnpm dev"
cwd = "./apps/web"
auto_restart = true

[apps.web.env]
NODE_ENV = "development"

[apps.api]
command = "pnpm start"
cwd = "./apps/api"
subdomain = "api.myapp"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.proxy.port, 1337);
        assert!(config.proxy.https);
        assert_eq!(
            config.tunnel.relay.as_deref(),
            Some("relay.portzero.dev:4443")
        );
        assert_eq!(config.apps.len(), 2);

        let web = &config.apps["web"];
        assert_eq!(web.command, "pnpm dev");
        assert!(web.auto_restart);
        assert_eq!(
            web.env.get("NODE_ENV").map(|s| s.as_str()),
            Some("development")
        );

        let api = &config.apps["api"];
        assert_eq!(api.subdomain.as_deref(), Some("api.myapp"));
    }

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.proxy.port, DEFAULT_PROXY_PORT);
        assert!(!config.proxy.https);
    }
}
