//! Credential storage for LocalUp tunnel authentication.
//!
//! Stores auth tokens and session info in `~/.portzero/credentials.json`.
//! The auth_token (JWT) is what's needed for tunnel connections.
//! The session_token is for API access (managing tokens, account, etc.)

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Filename within the state directory.
const CREDENTIALS_FILE: &str = "credentials.json";

/// Default relay API base URL.
pub const DEFAULT_RELAY_API: &str = "https://tunnel.kfs.es";

/// Stored credentials for LocalUp tunnel service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credentials {
    /// The relay API base URL (e.g. "https://tunnel.kfs.es")
    pub relay_api: String,
    /// Session token for API access (short-lived, from login)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_token: Option<String>,
    /// Auth token JWT for tunnel connections (long-lived)
    pub auth_token: String,
    /// User email
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// User ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

/// Path to the credentials file.
pub fn credentials_path(state_dir: &Path) -> PathBuf {
    state_dir.join(CREDENTIALS_FILE)
}

/// Load credentials from disk. Returns None if the file doesn't exist.
pub fn load_credentials(state_dir: &Path) -> Result<Option<Credentials>> {
    let path = credentials_path(state_dir);
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)?;
    let creds: Credentials = serde_json::from_str(&content)?;
    Ok(Some(creds))
}

/// Save credentials to disk. Creates the file with restricted permissions.
pub fn save_credentials(state_dir: &Path, creds: &Credentials) -> Result<()> {
    let path = credentials_path(state_dir);
    std::fs::create_dir_all(state_dir)?;

    let json = serde_json::to_string_pretty(creds)?;
    std::fs::write(&path, &json)?;

    // Restrict permissions on Unix (owner read/write only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(&path, perms)?;
    }

    Ok(())
}

/// Delete credentials file.
pub fn delete_credentials(state_dir: &Path) -> Result<()> {
    let path = credentials_path(state_dir);
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    Ok(())
}

/// Get the stored auth token, if any.
pub fn get_auth_token(state_dir: &Path) -> Option<String> {
    load_credentials(state_dir)
        .ok()
        .flatten()
        .map(|c| c.auth_token)
}

/// Default relay address for tunnel connections (QUIC control plane).
pub const DEFAULT_RELAY: &str = "tunnel.kfs.es:4443";

/// Environment variable names.
const ENV_TUNNEL_TOKEN: &str = "PORTZERO_TUNNEL_TOKEN";
const ENV_TUNNEL_RELAY: &str = "PORTZERO_TUNNEL_RELAY";

/// Resolved tunnel configuration from all sources.
#[derive(Debug, Clone)]
pub struct ResolvedTunnelConfig {
    /// JWT auth token for the relay.
    pub auth_token: Option<String>,
    /// Relay address (host:port).
    pub relay: String,
}

/// Resolve tunnel configuration from all sources.
///
/// Token resolution order (first wins):
///   1. `PORTZERO_TUNNEL_TOKEN` environment variable
///   2. `[tunnel] token = "..."` in portzero.toml
///   3. `~/.portzero/credentials.json` (from `portzero login`)
///
/// Relay resolution order (first wins):
///   1. `PORTZERO_TUNNEL_RELAY` environment variable
///   2. `[tunnel] relay = "..."` in portzero.toml
///   3. Relay stored in credentials.json
///   4. Default: `tunnel.kfs.es:4443`
///
/// This allows:
///   - CI/CD: set env vars
///   - Self-hosted: put token+relay in portzero.toml
///   - Hosted service: `portzero login` (stores in credentials.json)
pub fn resolve_tunnel_config(
    state_dir: &Path,
    config_tunnel: Option<&crate::config::TunnelConfig>,
) -> ResolvedTunnelConfig {
    // Token
    let auth_token = std::env::var(ENV_TUNNEL_TOKEN).ok()
        .or_else(|| config_tunnel.and_then(|c| c.token.clone()))
        .or_else(|| get_auth_token(state_dir));

    // Relay
    let relay = std::env::var(ENV_TUNNEL_RELAY).ok()
        .or_else(|| config_tunnel.and_then(|c| c.relay.clone()))
        .unwrap_or_else(|| DEFAULT_RELAY.to_string());

    ResolvedTunnelConfig { auth_token, relay }
}

/// Simple HTTP client for the LocalUp API.
/// Uses minimal dependencies (just tokio + serde_json over raw HTTP).
pub struct LocalUpApi {
    base_url: String,
}

/// Login response from the LocalUp API.
#[derive(Debug, Deserialize)]
pub struct LoginResponse {
    pub user: UserInfo,
    pub token: String,
    #[serde(default)]
    pub expires_at: Option<String>,
}

/// Registration response from the LocalUp API.
#[derive(Debug, Deserialize)]
pub struct RegisterResponse {
    pub user: UserInfo,
    pub token: String,
    #[serde(default)]
    pub auth_token: String,
}

/// User info from API responses.
#[derive(Debug, Deserialize)]
pub struct UserInfo {
    pub id: String,
    pub email: String,
    #[serde(default)]
    pub full_name: Option<String>,
}

/// Auth token info from API.
#[derive(Debug, Deserialize)]
pub struct AuthTokenInfo {
    pub id: String,
    pub name: String,
    pub token: String,
    #[serde(default)]
    pub expires_at: Option<String>,
}

/// Auth token list item (without the token value).
#[derive(Debug, Deserialize)]
pub struct AuthTokenListItem {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub last_used_at: Option<String>,
    #[serde(default)]
    pub expires_at: Option<String>,
}

/// Error response from the API.
#[derive(Debug, Deserialize)]
pub struct ApiError {
    #[serde(alias = "detail")]
    pub message: Option<String>,
    #[serde(alias = "title")]
    pub error: Option<String>,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ref msg) = self.message {
            write!(f, "{}", msg)
        } else if let Some(ref err) = self.error {
            write!(f, "{}", err)
        } else {
            write!(f, "Unknown error")
        }
    }
}

impl LocalUpApi {
    /// Create a new API client for the given relay URL.
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// Login with email and password. Returns session token + user info.
    pub async fn login(&self, email: &str, password: &str) -> Result<LoginResponse> {
        let url = format!("{}/api/auth/login", self.base_url);
        let body = serde_json::json!({
            "email": email,
            "password": password,
        });

        let response = self.post_json(&url, &body, None).await?;
        Ok(response)
    }

    /// Register a new account.
    pub async fn register(
        &self,
        email: &str,
        password: &str,
        full_name: Option<&str>,
    ) -> Result<RegisterResponse> {
        let url = format!("{}/api/auth/register", self.base_url);
        let mut body = serde_json::json!({
            "email": email,
            "password": password,
        });
        if let Some(name) = full_name {
            body["full_name"] = serde_json::Value::String(name.to_string());
        }

        let response = self.post_json(&url, &body, None).await?;
        Ok(response)
    }

    /// Create a new auth token (for tunnel connections).
    pub async fn create_auth_token(
        &self,
        session_token: &str,
        name: &str,
    ) -> Result<AuthTokenInfo> {
        let url = format!("{}/api/auth-tokens", self.base_url);
        let body = serde_json::json!({
            "name": name,
            "description": "Auto-created by PortZero",
        });

        let response = self.post_json(&url, &body, Some(session_token)).await?;
        Ok(response)
    }

    /// List existing auth tokens.
    pub async fn list_auth_tokens(
        &self,
        session_token: &str,
    ) -> Result<Vec<AuthTokenListItem>> {
        let url = format!("{}/api/auth-tokens", self.base_url);
        let response = self.get_json(&url, Some(session_token)).await?;
        Ok(response)
    }

    /// Internal: POST JSON and parse response.
    async fn post_json<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
        body: &serde_json::Value,
        bearer_token: Option<&str>,
    ) -> Result<T> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpStream;

        let parsed_url = url::Url::parse(url)?;
        let host = parsed_url
            .host_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid URL: no host"))?;
        let port = parsed_url.port_or_known_default().unwrap_or(443);
        let use_tls = parsed_url.scheme() == "https";

        let body_bytes = serde_json::to_vec(body)?;

        let mut request = format!(
            "POST {} HTTP/1.1\r\n\
             Host: {}\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             Accept: application/json\r\n",
            parsed_url.path(),
            host,
            body_bytes.len(),
        );

        if let Some(token) = bearer_token {
            request.push_str(&format!("Authorization: Bearer {}\r\n", token));
        }
        request.push_str("\r\n");

        let response_bytes = if use_tls {
            let tcp = TcpStream::connect(format!("{}:{}", host, port)).await?;
            let config = make_tls_config();
            let connector = tokio_rustls::TlsConnector::from(std::sync::Arc::new(config));
            let server_name = rustls::pki_types::ServerName::try_from(host.to_string())?;
            let mut tls = connector.connect(server_name, tcp).await?;
            tls.write_all(request.as_bytes()).await?;
            tls.write_all(&body_bytes).await?;
            tls.flush().await?;
            let mut buf = Vec::new();
            tls.read_to_end(&mut buf).await?;
            buf
        } else {
            let mut tcp = TcpStream::connect(format!("{}:{}", host, port)).await?;
            tcp.write_all(request.as_bytes()).await?;
            tcp.write_all(&body_bytes).await?;
            tcp.flush().await?;
            let mut buf = Vec::new();
            tcp.read_to_end(&mut buf).await?;
            buf
        };

        parse_http_response(&response_bytes)
    }

    /// Internal: GET JSON and parse response.
    async fn get_json<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
        bearer_token: Option<&str>,
    ) -> Result<T> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpStream;

        let parsed_url = url::Url::parse(url)?;
        let host = parsed_url
            .host_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid URL: no host"))?;
        let port = parsed_url.port_or_known_default().unwrap_or(443);
        let use_tls = parsed_url.scheme() == "https";

        let mut request = format!(
            "GET {} HTTP/1.1\r\n\
             Host: {}\r\n\
             Accept: application/json\r\n",
            parsed_url.path(),
            host,
        );

        if let Some(token) = bearer_token {
            request.push_str(&format!("Authorization: Bearer {}\r\n", token));
        }
        request.push_str("\r\n");

        let response_bytes = if use_tls {
            let tcp = TcpStream::connect(format!("{}:{}", host, port)).await?;
            let config = make_tls_config();
            let connector = tokio_rustls::TlsConnector::from(std::sync::Arc::new(config));
            let server_name = rustls::pki_types::ServerName::try_from(host.to_string())?;
            let mut tls = connector.connect(server_name, tcp).await?;
            tls.write_all(request.as_bytes()).await?;
            tls.flush().await?;
            let mut buf = Vec::new();
            tls.read_to_end(&mut buf).await?;
            buf
        } else {
            let mut tcp = TcpStream::connect(format!("{}:{}", host, port)).await?;
            tcp.write_all(request.as_bytes()).await?;
            tcp.flush().await?;
            let mut buf = Vec::new();
            tcp.read_to_end(&mut buf).await?;
            buf
        };

        parse_http_response(&response_bytes)
    }
}

/// Build a rustls ClientConfig with native root certificates.
fn make_tls_config() -> rustls::ClientConfig {
    let mut root_store = rustls::RootCertStore::empty();
    let native_certs = rustls_native_certs::load_native_certs();
    for cert in native_certs.certs {
        let _ = root_store.add(cert);
    }
    rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth()
}

/// Parse an HTTP/1.1 response, extract the JSON body.
fn parse_http_response<T: serde::de::DeserializeOwned>(raw: &[u8]) -> Result<T> {
    let response_str = String::from_utf8_lossy(raw);

    // Find the blank line separating headers from body
    let body_start = response_str
        .find("\r\n\r\n")
        .map(|i| i + 4)
        .or_else(|| response_str.find("\n\n").map(|i| i + 2))
        .ok_or_else(|| anyhow::anyhow!("Malformed HTTP response"))?;

    // Parse status line
    let status_line = response_str
        .lines()
        .next()
        .ok_or_else(|| anyhow::anyhow!("Empty response"))?;

    let status_code: u16 = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let body = &response_str[body_start..];

    // Handle chunked transfer encoding
    let body = if response_str[..body_start]
        .to_lowercase()
        .contains("transfer-encoding: chunked")
    {
        decode_chunked(body)?
    } else {
        body.to_string()
    };

    if status_code >= 400 {
        if let Ok(err) = serde_json::from_str::<ApiError>(&body) {
            anyhow::bail!("{}", err);
        }
        anyhow::bail!("HTTP {} - {}", status_code, body.trim());
    }

    serde_json::from_str(&body)
        .map_err(|e| anyhow::anyhow!("Failed to parse response: {} (body: {})", e, body.trim()))
}

/// Decode HTTP chunked transfer encoding.
fn decode_chunked(input: &str) -> Result<String> {
    let mut result = String::new();
    let mut remaining = input;

    loop {
        // Find the chunk size line
        let line_end = remaining
            .find("\r\n")
            .or_else(|| remaining.find('\n'))
            .unwrap_or(remaining.len());

        let size_str = remaining[..line_end].trim();
        if size_str.is_empty() {
            remaining = &remaining[line_end + if remaining[line_end..].starts_with("\r\n") { 2 } else { 1 }..];
            continue;
        }

        let chunk_size = usize::from_str_radix(size_str, 16).unwrap_or(0);
        if chunk_size == 0 {
            break;
        }

        let data_start = line_end + if remaining[line_end..].starts_with("\r\n") { 2 } else { 1 };
        let data_end = (data_start + chunk_size).min(remaining.len());
        result.push_str(&remaining[data_start..data_end]);

        remaining = &remaining[data_end..];
        // Skip trailing \r\n after chunk data
        if remaining.starts_with("\r\n") {
            remaining = &remaining[2..];
        } else if remaining.starts_with('\n') {
            remaining = &remaining[1..];
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_save_and_load_credentials() {
        let tmp = TempDir::new().unwrap();
        let creds = Credentials {
            relay_api: "https://tunnel.kfs.es".to_string(),
            session_token: Some("session-123".to_string()),
            auth_token: "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.test".to_string(),
            email: Some("user@example.com".to_string()),
            user_id: Some("user-uuid".to_string()),
        };

        save_credentials(tmp.path(), &creds).unwrap();

        let loaded = load_credentials(tmp.path()).unwrap().unwrap();
        assert_eq!(loaded.auth_token, creds.auth_token);
        assert_eq!(loaded.email, creds.email);
        assert_eq!(loaded.relay_api, creds.relay_api);
    }

    #[test]
    fn test_load_missing_credentials() {
        let tmp = TempDir::new().unwrap();
        assert!(load_credentials(tmp.path()).unwrap().is_none());
    }

    #[test]
    fn test_delete_credentials() {
        let tmp = TempDir::new().unwrap();
        let creds = Credentials {
            relay_api: "https://tunnel.kfs.es".to_string(),
            session_token: None,
            auth_token: "token".to_string(),
            email: None,
            user_id: None,
        };

        save_credentials(tmp.path(), &creds).unwrap();
        assert!(load_credentials(tmp.path()).unwrap().is_some());

        delete_credentials(tmp.path()).unwrap();
        assert!(load_credentials(tmp.path()).unwrap().is_none());
    }

    #[test]
    fn test_get_auth_token() {
        let tmp = TempDir::new().unwrap();
        assert!(get_auth_token(tmp.path()).is_none());

        let creds = Credentials {
            relay_api: "https://tunnel.kfs.es".to_string(),
            session_token: None,
            auth_token: "my-jwt-token".to_string(),
            email: None,
            user_id: None,
        };
        save_credentials(tmp.path(), &creds).unwrap();

        assert_eq!(get_auth_token(tmp.path()), Some("my-jwt-token".to_string()));
    }
}
