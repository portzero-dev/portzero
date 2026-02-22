//! Shared types used across all PortZero crates.
//!
//! This module defines the core data structures and trait interfaces that
//! `portzero-api`, `portzero-mcp`, and `portzero-dashboard` code against.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Route / App types
// ---------------------------------------------------------------------------

/// A registered app route: maps a subdomain to a local port.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    /// Subdomain name (e.g. "my-app", "api.my-app")
    pub hostname: String,
    /// Local port the app is listening on
    pub port: u16,
    /// OS process ID of the managed child
    pub pid: u32,
    /// The command used to start the app
    pub command: Vec<String>,
    /// Working directory of the app
    pub cwd: PathBuf,
    /// When the app was started
    pub started_at: DateTime<Utc>,
    /// Current app status
    pub status: AppStatus,
}

/// Current status of a managed application.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum AppStatus {
    Running,
    Crashed { exit_code: i32, at: DateTime<Utc> },
    Stopped,
}

impl AppStatus {
    pub fn is_running(&self) -> bool {
        matches!(self, AppStatus::Running)
    }
}

// ---------------------------------------------------------------------------
// Request recording types
// ---------------------------------------------------------------------------

/// A fully captured HTTP request/response pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestRecord {
    /// Unique ID for this record
    pub id: String,
    /// Which app handled this request
    pub app_name: String,
    /// When the request was received
    pub timestamp: DateTime<Utc>,
    /// Total duration from request start to response end
    pub duration_ms: u64,

    // -- Request --
    pub method: String,
    pub url: String,
    pub path: String,
    pub query_string: String,
    pub request_headers: HashMap<String, String>,
    /// Request body (None if empty or too large)
    pub request_body: Option<Vec<u8>>,
    pub request_content_type: Option<String>,

    // -- Response --
    pub status_code: u16,
    /// HTTP status text (e.g. "OK", "Not Found")
    #[serde(default)]
    pub status_message: String,
    pub response_headers: HashMap<String, String>,
    /// Response body (None if empty or too large)
    pub response_body: Option<Vec<u8>>,
    pub response_content_type: Option<String>,

    // -- Metadata --
    /// Whether this response was served by the mock engine
    pub mocked: bool,
    /// If this is a replayed request, the ID of the original
    pub parent_id: Option<String>,
}

// ---------------------------------------------------------------------------
// Mock types (Task 4 implements, Task 1 defines interface)
// ---------------------------------------------------------------------------

/// A mock response rule for a specific route.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockRule {
    pub id: String,
    pub app_name: String,
    pub method: Option<String>,
    pub path_pattern: String,
    pub status_code: u16,
    pub response_headers: HashMap<String, String>,
    pub response_body: String,
    pub enabled: bool,
    /// Number of times this mock has been matched.
    #[serde(default)]
    pub hit_count: u64,
}

/// A mock response to send back to the client (produced by the mock engine).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockResponse {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
}

// ---------------------------------------------------------------------------
// Network simulation types (Task 4 implements, Task 1 defines interface)
// ---------------------------------------------------------------------------

/// Network conditions to simulate for an app.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkProfile {
    pub app_name: String,
    /// Fixed latency added to every response (milliseconds)
    pub latency_ms: Option<u64>,
    /// Random jitter range +/- (milliseconds)
    #[serde(rename = "latency_jitter_ms")]
    pub jitter_ms: Option<u64>,
    /// Probability of dropping a request (0.0 - 1.0)
    pub packet_loss_rate: f64,
    /// Bandwidth limit in bytes per second
    #[serde(rename = "bandwidth_limit_bytes")]
    pub bandwidth_limit: Option<u64>,
    /// Only apply to matching paths (glob pattern)
    pub path_filter: Option<String>,
}

// ---------------------------------------------------------------------------
// Schema inference types (Task 4 implements, Task 1 defines interface)
// ---------------------------------------------------------------------------

/// An inferred API endpoint from observed traffic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferredEndpoint {
    pub method: String,
    /// Parameterized path template (e.g. "/api/users/:id")
    pub path_template: String,
    pub query_params: Vec<ParamInfo>,
    pub request_body_schema: Option<JsonSchema>,
    pub response_schemas: HashMap<u16, JsonSchema>,
    pub sample_count: u64,
}

/// Information about a query parameter or path parameter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParamInfo {
    pub name: String,
    #[serde(rename = "type")]
    pub param_type: String,
    pub required: bool,
    pub example_values: Vec<String>,
}

/// A simplified JSON schema representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSchema {
    #[serde(rename = "type")]
    pub schema_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, JsonSchema>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<JsonSchema>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "enum")]
    pub enum_values: Option<Vec<String>>,
}

/// Full inferred schema for an app.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferredSchema {
    pub app_name: String,
    pub endpoints: Vec<InferredEndpoint>,
    pub last_updated: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Tunnel types (Task 4 implements via localup-lib)
// ---------------------------------------------------------------------------

/// Status of an active tunnel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelInfo {
    pub app_name: String,
    pub public_url: String,
    pub relay: String,
    /// Transport protocol (e.g. "quic", "tcp", "websocket")
    #[serde(default = "default_transport")]
    pub transport: String,
    pub started_at: DateTime<Utc>,
}

fn default_transport() -> String {
    "quic".to_string()
}

// ---------------------------------------------------------------------------
// Process log types
// ---------------------------------------------------------------------------

/// A single log line from a managed process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogLine {
    pub timestamp: DateTime<Utc>,
    pub stream: LogStream,
    pub content: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogStream {
    Stdout,
    Stderr,
}

// ---------------------------------------------------------------------------
// WebSocket event types (shared contract with dashboard)
// ---------------------------------------------------------------------------

/// Events broadcast over WebSocket to the dashboard.
///
/// Serializes with `"type": "request:start"` style tags (internally tagged)
/// matching the dashboard's expected format.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsEvent {
    #[serde(rename = "request:start")]
    RequestStart {
        id: String,
        app: String,
        method: String,
        url: String,
        timestamp: DateTime<Utc>,
    },
    #[serde(rename = "request:complete")]
    RequestComplete {
        id: String,
        app: String,
        /// Serialized as "status" to match dashboard expectation.
        #[serde(rename = "status")]
        status_code: u16,
        duration_ms: u64,
    },
    #[serde(rename = "app:registered")]
    AppRegistered {
        name: String,
        port: u16,
        pid: u32,
        url: String,
    },
    #[serde(rename = "app:removed")]
    AppRemoved { name: String },
    #[serde(rename = "app:crashed")]
    AppCrashed { name: String, exit_code: i32 },
    #[serde(rename = "app:restarted")]
    AppRestarted { name: String, pid: u32 },
    #[serde(rename = "log:line")]
    LogLine {
        app: String,
        stream: LogStream,
        line: String,
        timestamp: DateTime<Utc>,
    },
    #[serde(rename = "mock:hit")]
    MockHit { mock_id: String, request_id: String },
    #[serde(rename = "tunnel:started")]
    TunnelStarted { app: String, public_url: String },
    #[serde(rename = "tunnel:stopped")]
    TunnelStopped { app: String },
}

// ---------------------------------------------------------------------------
// Daemon status
// ---------------------------------------------------------------------------

/// High-level daemon status info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonStatus {
    #[serde(rename = "uptime_seconds")]
    pub uptime_secs: u64,
    pub proxy_port: u16,
    #[serde(rename = "app_count")]
    pub total_apps: usize,
    pub total_requests: u64,
    pub version: String,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Default proxy port.
pub const DEFAULT_PROXY_PORT: u16 = 1337;

/// Reserved subdomain for the dashboard/API.
pub const RESERVED_SUBDOMAIN: &str = "_portzero";

/// Port range base for auto-assigned app ports.
pub const PORT_RANGE_BASE: u16 = 4000;

/// Port range size.
pub const PORT_RANGE_SIZE: u16 = 1000;

/// Maximum request/response body size to capture (1MB).
pub const MAX_BODY_CAPTURE_SIZE: usize = 1_048_576;

/// Maximum number of request records to retain.
pub const MAX_REQUEST_RECORDS: usize = 10_000;

/// Maximum log lines per app ring buffer.
pub const MAX_LOG_LINES: usize = 5_000;

// ---------------------------------------------------------------------------
// API request/response types (used by portzero-api)
// ---------------------------------------------------------------------------

/// Summary view of a request (for list endpoints — lighter than full RequestRecord).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestSummary {
    pub id: String,
    pub app_name: String,
    pub timestamp: DateTime<Utc>,
    pub method: String,
    pub path: String,
    pub status_code: u16,
    pub duration_ms: u64,
    pub mocked: bool,
}

/// Filters for querying captured requests.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RequestFilter {
    pub app: Option<String>,
    pub method: Option<String>,
    pub status: Option<u16>,
    pub status_range: Option<String>,
    pub path: Option<String>,
    pub search: Option<String>,
    pub from: Option<i64>,
    pub to: Option<i64>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// Request body for replaying a captured request.
#[derive(Debug, Clone, Deserialize)]
pub struct ReplayRequest {
    #[serde(default)]
    pub headers: Option<HashMap<String, String>>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
}

/// Side-by-side diff of two requests.
#[derive(Debug, Clone, Serialize)]
pub struct RequestDiff {
    pub left: RequestRecord,
    pub right: RequestRecord,
}

/// Request body for creating a mock rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMockRule {
    pub app_name: String,
    pub method: Option<String>,
    pub path_pattern: String,
    pub status_code: u16,
    #[serde(default)]
    pub response_headers: HashMap<String, String>,
    #[serde(default)]
    pub response_body: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Request body for updating a mock rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMockRule {
    pub method: Option<Option<String>>,
    pub path_pattern: Option<String>,
    pub status_code: Option<u16>,
    pub response_headers: Option<HashMap<String, String>>,
    pub response_body: Option<String>,
    pub enabled: Option<bool>,
}

fn default_true() -> bool {
    true
}

/// Request body for setting a network profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetNetworkProfile {
    pub latency_ms: Option<u64>,
    #[serde(alias = "latency_jitter_ms")]
    pub jitter_ms: Option<u64>,
    pub packet_loss_rate: Option<f64>,
    #[serde(alias = "bandwidth_limit_bytes")]
    pub bandwidth_limit: Option<u64>,
    pub path_filter: Option<String>,
}

/// Request body for starting a tunnel.
#[derive(Debug, Clone, Deserialize)]
pub struct ShareRequest {
    pub subdomain: Option<String>,
    pub relay: Option<String>,
}

/// A paginated list response.
#[derive(Debug, Clone, Serialize)]
pub struct PaginatedResponse<T: Serialize> {
    pub items: Vec<T>,
    pub total: u64,
    pub limit: u32,
    pub offset: u32,
}

/// Standard API error response body.
#[derive(Debug, Clone, Serialize)]
pub struct ApiError {
    pub error: String,
    pub message: String,
}

impl ApiError {
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self {
            error: "not_found".to_string(),
            message: msg.into(),
        }
    }

    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self {
            error: "bad_request".to_string(),
            message: msg.into(),
        }
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self {
            error: "internal_error".to_string(),
            message: msg.into(),
        }
    }
}

/// Info about an app exposed via the API (richer than Route).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    pub name: String,
    pub port: u16,
    pub pid: Option<u32>,
    pub command: Vec<String>,
    pub cwd: PathBuf,
    pub status: AppStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub restarts: u32,
    pub auto_restart: bool,
    pub url: String,
    #[serde(rename = "cpu_usage")]
    pub cpu_percent: Option<f64>,
    pub memory_bytes: Option<u64>,
    pub tunnel_url: Option<String>,
}
