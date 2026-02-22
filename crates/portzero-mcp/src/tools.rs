//! MCP tool definitions for PortZero.
//!
//! Each tool corresponds to an action an AI agent can take:
//! - `list_apps` -- List all running apps with status, port, URL
//! - `get_app_logs` -- Get last N log lines for an app
//! - `get_recent_requests` -- Get last N requests with optional filters
//! - `get_request_detail` -- Get full request/response for a request ID
//! - `replay_request` -- Re-send a captured request with optional overrides
//! - `restart_app` -- Restart a crashed or running app
//! - `get_app_schema` -- Get the inferred API schema for an app

use portzero_core::store::{RequestFilter, Store};
use portzero_core::types::*;
use portzero_core::SchemaInference;
use serde::Serialize;
use serde_json::Value;
use std::sync::Arc;

/// Shared state that tools have access to.
pub struct ToolContext {
    pub store: Arc<Store>,
    pub schema_inference: Arc<SchemaInference>,
    // Process manager and router would go here, but they are owned by Task 1.
    // We define trait-based interfaces for them.
}

/// Trait for process manager operations needed by MCP tools.
/// Task 1 implements this; Task 4 codes against the interface.
#[async_trait::async_trait]
pub trait ProcessManagerOps: Send + Sync {
    /// List all running apps.
    fn list_apps(&self) -> Vec<AppInfo>;
    /// Get logs for an app.
    fn get_logs(&self, app_name: &str, lines: usize) -> Option<Vec<LogLine>>;
    /// Restart an app.
    async fn restart_app(&self, app_name: &str) -> anyhow::Result<()>;
}

/// A no-op process manager for when the real one isn't available.
pub struct StubProcessManager;

#[async_trait::async_trait]
impl ProcessManagerOps for StubProcessManager {
    fn list_apps(&self) -> Vec<AppInfo> {
        Vec::new()
    }
    fn get_logs(&self, _app_name: &str, _lines: usize) -> Option<Vec<LogLine>> {
        None
    }
    async fn restart_app(&self, app_name: &str) -> anyhow::Result<()> {
        anyhow::bail!("Process manager not available (app: {})", app_name)
    }
}

// ---------------------------------------------------------------------------
// Tool metadata
// ---------------------------------------------------------------------------

/// MCP tool definition (as returned in `tools/list`).
#[derive(Debug, Clone, Serialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

/// Return the list of all MCP tools with their schemas.
pub fn tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "list_apps".to_string(),
            description: "List all running apps with status, port, and URL".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        ToolDefinition {
            name: "get_app_logs".to_string(),
            description: "Get the last N log lines for a specific app".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "app": {
                        "type": "string",
                        "description": "App name"
                    },
                    "lines": {
                        "type": "integer",
                        "description": "Number of log lines to return (default: 50)",
                        "default": 50
                    }
                },
                "required": ["app"]
            }),
        },
        ToolDefinition {
            name: "get_recent_requests".to_string(),
            description: "Get recent HTTP requests captured by the proxy, optionally filtered by app, status, or method".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "app": {
                        "type": "string",
                        "description": "Filter by app name"
                    },
                    "method": {
                        "type": "string",
                        "description": "Filter by HTTP method (GET, POST, etc.)"
                    },
                    "status": {
                        "type": "integer",
                        "description": "Filter by exact status code"
                    },
                    "status_range": {
                        "type": "string",
                        "description": "Filter by status range: '2xx', '3xx', '4xx', '5xx'"
                    },
                    "path": {
                        "type": "string",
                        "description": "Filter by path prefix"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of requests to return (default: 20)",
                        "default": 20
                    }
                },
                "required": []
            }),
        },
        ToolDefinition {
            name: "get_request_detail".to_string(),
            description: "Get the full request and response details for a specific request ID, including headers and body".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "Request ID"
                    }
                },
                "required": ["id"]
            }),
        },
        ToolDefinition {
            name: "replay_request".to_string(),
            description: "Re-send a previously captured request with optional overrides to method, URL, headers, or body".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "ID of the request to replay"
                    },
                    "method": {
                        "type": "string",
                        "description": "Override the HTTP method"
                    },
                    "url": {
                        "type": "string",
                        "description": "Override the URL"
                    },
                    "headers": {
                        "type": "object",
                        "description": "Override or add headers (merged with original)",
                        "additionalProperties": { "type": "string" }
                    },
                    "body": {
                        "type": "string",
                        "description": "Override the request body"
                    }
                },
                "required": ["id"]
            }),
        },
        ToolDefinition {
            name: "restart_app".to_string(),
            description: "Restart a running or crashed app".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "app": {
                        "type": "string",
                        "description": "App name to restart"
                    }
                },
                "required": ["app"]
            }),
        },
        ToolDefinition {
            name: "get_app_schema".to_string(),
            description: "Get the inferred API schema for an app, built from observed traffic. Shows endpoints, methods, parameters, and status codes.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "app": {
                        "type": "string",
                        "description": "App name"
                    }
                },
                "required": ["app"]
            }),
        },
    ]
}

// ---------------------------------------------------------------------------
// Tool execution
// ---------------------------------------------------------------------------

/// Result of executing a tool.
#[derive(Debug, Serialize)]
pub struct ToolResult {
    pub content: Vec<ToolContent>,
    #[serde(rename = "isError", skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct ToolContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}

impl ToolResult {
    pub fn text(s: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text: s.into(),
            }],
            is_error: None,
        }
    }

    pub fn json(value: &impl Serialize) -> Self {
        Self::text(serde_json::to_string_pretty(value).unwrap_or_else(|e| format!("Serialization error: {}", e)))
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text: msg.into(),
            }],
            is_error: Some(true),
        }
    }
}

/// Execute a tool by name with the given arguments.
pub async fn execute_tool(
    name: &str,
    args: &Value,
    ctx: &ToolContext,
    pm: &dyn ProcessManagerOps,
) -> ToolResult {
    match name {
        "list_apps" => tool_list_apps(pm).await,
        "get_app_logs" => tool_get_app_logs(args, pm).await,
        "get_recent_requests" => tool_get_recent_requests(args, ctx).await,
        "get_request_detail" => tool_get_request_detail(args, ctx).await,
        "replay_request" => tool_replay_request(args, ctx).await,
        "restart_app" => tool_restart_app(args, pm).await,
        "get_app_schema" => tool_get_app_schema(args, ctx).await,
        _ => ToolResult::error(format!("Unknown tool: {}", name)),
    }
}

// ---------------------------------------------------------------------------
// Individual tool implementations
// ---------------------------------------------------------------------------

async fn tool_list_apps(pm: &dyn ProcessManagerOps) -> ToolResult {
    let apps = pm.list_apps();
    if apps.is_empty() {
        return ToolResult::text("No apps are currently running.");
    }

    #[derive(Serialize)]
    struct AppSummary {
        name: String,
        status: String,
        port: u16,
        url: String,
        pid: Option<u32>,
        restarts: u32,
    }

    let summaries: Vec<AppSummary> = apps
        .iter()
        .map(|app| AppSummary {
            name: app.name.clone(),
            status: match &app.status {
                AppStatus::Running => "running".to_string(),
                AppStatus::Crashed { exit_code, .. } => format!("crashed (exit code {})", exit_code),
                AppStatus::Stopped => "stopped".to_string(),
            },
            port: app.port,
            url: app.url.clone(),
            pid: app.pid,
            restarts: app.restarts,
        })
        .collect();

    ToolResult::json(&summaries)
}

async fn tool_get_app_logs(args: &Value, pm: &dyn ProcessManagerOps) -> ToolResult {
    let app_name = match args.get("app").and_then(|v| v.as_str()) {
        Some(name) => name,
        None => return ToolResult::error("Missing required parameter: app"),
    };

    let lines = args
        .get("lines")
        .and_then(|v| v.as_u64())
        .unwrap_or(50) as usize;

    match pm.get_logs(app_name, lines) {
        Some(logs) => {
            if logs.is_empty() {
                return ToolResult::text(format!("No logs available for app '{}'.", app_name));
            }

            let formatted: Vec<String> = logs
                .iter()
                .map(|line| {
                    let stream = match line.stream {
                        LogStream::Stdout => "stdout",
                        LogStream::Stderr => "stderr",
                    };
                    format!(
                        "[{}] [{}] {}",
                        line.timestamp.format("%H:%M:%S%.3f"),
                        stream,
                        line.content
                    )
                })
                .collect();

            ToolResult::text(formatted.join("\n"))
        }
        None => ToolResult::error(format!("App '{}' not found.", app_name)),
    }
}

async fn tool_get_recent_requests(args: &Value, ctx: &ToolContext) -> ToolResult {
    let mut filter = RequestFilter::default();

    if let Some(app) = args.get("app").and_then(|v| v.as_str()) {
        filter.app_name = Some(app.to_string());
    }
    if let Some(method) = args.get("method").and_then(|v| v.as_str()) {
        filter.method = Some(method.to_string());
    }
    if let Some(status) = args.get("status").and_then(|v| v.as_u64()) {
        filter.status_code = Some(status as u16);
    }
    if let Some(path) = args.get("path").and_then(|v| v.as_str()) {
        filter.path_prefix = Some(path.to_string());
    }
    if let Some(search) = args.get("search").and_then(|v| v.as_str()) {
        filter.search = Some(search.to_string());
    }

    let limit = args
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(20) as usize;
    filter.limit = Some(limit);

    // Handle status_range filter (e.g. "5xx")
    if let Some(range) = args.get("status_range").and_then(|v| v.as_str()) {
        match range {
            "2xx" => {
                // We'll filter post-query since the store only supports exact status
                // For now, just return all and filter
            }
            "4xx" | "5xx" => {}
            _ => {}
        }
    }

    match ctx.store.list_requests(&filter) {
        Ok(requests) => {
            let mut requests = requests;

            // Apply status_range filter if specified
            if let Some(range) = args.get("status_range").and_then(|v| v.as_str()) {
                let range_start = match range {
                    "2xx" => 200,
                    "3xx" => 300,
                    "4xx" => 400,
                    "5xx" => 500,
                    _ => 0,
                };
                if range_start > 0 {
                    requests.retain(|r| r.status_code >= range_start && r.status_code < range_start + 100);
                }
            }

            if requests.is_empty() {
                return ToolResult::text("No matching requests found.");
            }

            #[derive(Serialize)]
            struct RequestSummary {
                id: String,
                app: String,
                method: String,
                path: String,
                status: u16,
                duration_ms: u64,
                mocked: bool,
            }

            let summaries: Vec<RequestSummary> = requests
                .iter()
                .map(|r| RequestSummary {
                    id: r.id.clone(),
                    app: r.app_name.clone(),
                    method: r.method.clone(),
                    path: r.path.clone(),
                    status: r.status_code,
                    duration_ms: r.duration_ms,
                    mocked: r.mocked,
                })
                .collect();

            ToolResult::json(&summaries)
        }
        Err(e) => ToolResult::error(format!("Failed to query requests: {}", e)),
    }
}

async fn tool_get_request_detail(args: &Value, ctx: &ToolContext) -> ToolResult {
    let id = match args.get("id").and_then(|v| v.as_str()) {
        Some(id) => id,
        None => return ToolResult::error("Missing required parameter: id"),
    };

    match ctx.store.get_request(id) {
        Ok(Some(record)) => {
            #[derive(Serialize)]
            struct RequestDetail {
                id: String,
                app: String,
                timestamp: String,
                duration_ms: u64,
                request: RequestPart,
                response: ResponsePart,
                mocked: bool,
            }

            #[derive(Serialize)]
            struct RequestPart {
                method: String,
                url: String,
                path: String,
                query_string: String,
                headers: std::collections::HashMap<String, String>,
                body: Option<String>,
                content_type: Option<String>,
            }

            #[derive(Serialize)]
            struct ResponsePart {
                status_code: u16,
                headers: std::collections::HashMap<String, String>,
                body: Option<String>,
                content_type: Option<String>,
            }

            let request_body = record.request_body.as_ref().map(|b| {
                String::from_utf8(b.clone())
                    .unwrap_or_else(|_| format!("<binary, {} bytes>", b.len()))
            });

            let response_body = record.response_body.as_ref().map(|b| {
                String::from_utf8(b.clone())
                    .unwrap_or_else(|_| format!("<binary, {} bytes>", b.len()))
            });

            let detail = RequestDetail {
                id: record.id.clone(),
                app: record.app_name.clone(),
                timestamp: record.timestamp.to_rfc3339(),
                duration_ms: record.duration_ms,
                request: RequestPart {
                    method: record.method.clone(),
                    url: record.url.clone(),
                    path: record.path.clone(),
                    query_string: record.query_string.clone(),
                    headers: record.request_headers.clone(),
                    body: request_body,
                    content_type: record.request_content_type.clone(),
                },
                response: ResponsePart {
                    status_code: record.status_code,
                    headers: record.response_headers.clone(),
                    body: response_body,
                    content_type: record.response_content_type.clone(),
                },
                mocked: record.mocked,
            };

            ToolResult::json(&detail)
        }
        Ok(None) => ToolResult::error(format!("Request '{}' not found.", id)),
        Err(e) => ToolResult::error(format!("Failed to get request: {}", e)),
    }
}

async fn tool_replay_request(args: &Value, ctx: &ToolContext) -> ToolResult {
    let id = match args.get("id").and_then(|v| v.as_str()) {
        Some(id) => id,
        None => return ToolResult::error("Missing required parameter: id"),
    };

    let original = match ctx.store.get_request(id) {
        Ok(Some(record)) => record,
        Ok(None) => return ToolResult::error(format!("Request '{}' not found.", id)),
        Err(e) => return ToolResult::error(format!("Failed to get request: {}", e)),
    };

    // Build the replay request
    let method = args
        .get("method")
        .and_then(|v| v.as_str())
        .unwrap_or(&original.method);

    let url = args
        .get("url")
        .and_then(|v| v.as_str())
        .unwrap_or(&original.url);

    let mut headers = original.request_headers.clone();
    if let Some(override_headers) = args.get("headers").and_then(|v| v.as_object()) {
        for (key, val) in override_headers {
            if let Some(val_str) = val.as_str() {
                headers.insert(key.clone(), val_str.to_string());
            }
        }
    }

    let body = args
        .get("body")
        .and_then(|v| v.as_str())
        .map(|s| s.as_bytes().to_vec())
        .or(original.request_body.clone());

    // For now, we can't actually send HTTP requests from the MCP server
    // (that would require an HTTP client dependency). Instead, we record
    // the replay intent and return the details.
    // In the full integration, the proxy or API server would handle the actual replay.

    #[derive(Serialize)]
    struct ReplayInfo {
        message: String,
        original_id: String,
        method: String,
        url: String,
        headers: std::collections::HashMap<String, String>,
        has_body: bool,
    }

    let info = ReplayInfo {
        message: format!(
            "Replay prepared for {} {}. In full integration, this would be sent through the proxy.",
            method, url
        ),
        original_id: id.to_string(),
        method: method.to_string(),
        url: url.to_string(),
        headers,
        has_body: body.is_some(),
    };

    ToolResult::json(&info)
}

async fn tool_restart_app(args: &Value, pm: &dyn ProcessManagerOps) -> ToolResult {
    let app_name = match args.get("app").and_then(|v| v.as_str()) {
        Some(name) => name,
        None => return ToolResult::error("Missing required parameter: app"),
    };

    match pm.restart_app(app_name).await {
        Ok(()) => ToolResult::text(format!("App '{}' has been restarted.", app_name)),
        Err(e) => ToolResult::error(format!("Failed to restart '{}': {}", app_name, e)),
    }
}

async fn tool_get_app_schema(args: &Value, ctx: &ToolContext) -> ToolResult {
    let app_name = match args.get("app").and_then(|v| v.as_str()) {
        Some(name) => name,
        None => return ToolResult::error("Missing required parameter: app"),
    };

    match ctx.schema_inference.get_schema(app_name) {
        Some(schema) => {
            if schema.endpoints.is_empty() {
                return ToolResult::text(format!(
                    "No API schema available for '{}'. Send some traffic first.",
                    app_name
                ));
            }
            ToolResult::json(&schema)
        }
        None => ToolResult::text(format!(
            "No traffic has been observed for app '{}' yet.",
            app_name
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::collections::HashMap;
    use std::path::PathBuf;

    /// Test process manager with pre-loaded data.
    struct TestProcessManager {
        apps: Vec<AppInfo>,
        logs: HashMap<String, Vec<LogLine>>,
    }

    #[async_trait::async_trait]
    impl ProcessManagerOps for TestProcessManager {
        fn list_apps(&self) -> Vec<AppInfo> {
            self.apps.clone()
        }

        fn get_logs(&self, app_name: &str, lines: usize) -> Option<Vec<LogLine>> {
            self.logs.get(app_name).map(|logs| {
                let start = logs.len().saturating_sub(lines);
                logs[start..].to_vec()
            })
        }

        async fn restart_app(&self, app_name: &str) -> anyhow::Result<()> {
            if self.apps.iter().any(|a| a.name == app_name) {
                Ok(())
            } else {
                anyhow::bail!("App '{}' not found", app_name)
            }
        }
    }

    fn make_test_pm() -> TestProcessManager {
        let mut logs = HashMap::new();
        logs.insert(
            "api".to_string(),
            vec![
                LogLine {
                    timestamp: Utc::now(),
                    stream: LogStream::Stdout,
                    content: "Server started on port 4001".to_string(),
                },
                LogLine {
                    timestamp: Utc::now(),
                    stream: LogStream::Stderr,
                    content: "Warning: deprecated API used".to_string(),
                },
            ],
        );

        TestProcessManager {
            apps: vec![
                AppInfo {
                    name: "web".to_string(),
                    port: 4000,
                    pid: Some(1234),
                    command: vec!["pnpm".to_string(), "dev".to_string()],
                    cwd: PathBuf::from("/app/web"),
                    status: AppStatus::Running,
                    started_at: Some(Utc::now()),
                    restarts: 0,
                    auto_restart: true,
                    url: "http://web.localhost:1337".to_string(),
                    cpu_percent: None,
                    memory_bytes: None,
                    tunnel_url: None,
                },
                AppInfo {
                    name: "api".to_string(),
                    port: 4001,
                    pid: Some(5678),
                    command: vec!["pnpm".to_string(), "start".to_string()],
                    cwd: PathBuf::from("/app/api"),
                    status: AppStatus::Running,
                    started_at: Some(Utc::now()),
                    restarts: 1,
                    auto_restart: true,
                    url: "http://api.localhost:1337".to_string(),
                    cpu_percent: None,
                    memory_bytes: None,
                    tunnel_url: None,
                },
            ],
            logs,
        }
    }

    fn make_test_ctx() -> ToolContext {
        let store = Store::in_memory().unwrap();
        // Populate with some test data
        store
            .insert_request(&RequestRecord {
                id: "req-1".to_string(),
                app_name: "api".to_string(),
                timestamp: Utc::now(),
                duration_ms: 45,
                method: "POST".to_string(),
                url: "http://api.localhost:1337/api/users".to_string(),
                path: "/api/users".to_string(),
                query_string: String::new(),
                request_headers: {
                    let mut h = HashMap::new();
                    h.insert("Content-Type".to_string(), "application/json".to_string());
                    h
                },
                request_body: Some(br#"{"name":"John"}"#.to_vec()),
                request_content_type: Some("application/json".to_string()),
                status_code: 500,
                status_message: String::new(),
                response_headers: HashMap::new(),
                response_body: Some(br#"{"error":"duplicate key: email"}"#.to_vec()),
                response_content_type: Some("application/json".to_string()),
                mocked: false,
                parent_id: None,
            })
            .unwrap();

        store
            .insert_request(&RequestRecord {
                id: "req-2".to_string(),
                app_name: "web".to_string(),
                timestamp: Utc::now(),
                duration_ms: 12,
                method: "GET".to_string(),
                url: "http://web.localhost:1337/".to_string(),
                path: "/".to_string(),
                query_string: String::new(),
                request_headers: HashMap::new(),
                request_body: None,
                request_content_type: None,
                status_code: 200,
                status_message: String::new(),
                response_headers: HashMap::new(),
                response_body: Some(b"<html>Hello</html>".to_vec()),
                response_content_type: Some("text/html".to_string()),
                mocked: false,
                parent_id: None,
            })
            .unwrap();

        let schema_inference = Arc::new(SchemaInference::new());

        ToolContext {
            store: Arc::new(store),
            schema_inference,
        }
    }

    #[tokio::test]
    async fn test_list_apps() {
        let pm = make_test_pm();
        let result = tool_list_apps(&pm).await;
        assert!(result.is_error.is_none());

        let text = &result.content[0].text;
        assert!(text.contains("web"));
        assert!(text.contains("api"));
        assert!(text.contains("running"));
    }

    #[tokio::test]
    async fn test_list_apps_empty() {
        let pm = TestProcessManager {
            apps: Vec::new(),
            logs: HashMap::new(),
        };
        let result = tool_list_apps(&pm).await;
        assert!(result.content[0].text.contains("No apps"));
    }

    #[tokio::test]
    async fn test_get_app_logs() {
        let pm = make_test_pm();
        let args = serde_json::json!({"app": "api", "lines": 10});
        let result = tool_get_app_logs(&args, &pm).await;
        assert!(result.is_error.is_none());
        assert!(result.content[0].text.contains("Server started"));
        assert!(result.content[0].text.contains("deprecated"));
    }

    #[tokio::test]
    async fn test_get_app_logs_not_found() {
        let pm = make_test_pm();
        let args = serde_json::json!({"app": "nonexistent"});
        let result = tool_get_app_logs(&args, &pm).await;
        assert_eq!(result.is_error, Some(true));
    }

    #[tokio::test]
    async fn test_get_app_logs_missing_param() {
        let pm = make_test_pm();
        let args = serde_json::json!({});
        let result = tool_get_app_logs(&args, &pm).await;
        assert_eq!(result.is_error, Some(true));
        assert!(result.content[0].text.contains("Missing"));
    }

    #[tokio::test]
    async fn test_get_recent_requests() {
        let ctx = make_test_ctx();
        let args = serde_json::json!({"limit": 10});
        let result = tool_get_recent_requests(&args, &ctx).await;
        assert!(result.is_error.is_none());

        let text = &result.content[0].text;
        assert!(text.contains("req-1"));
        assert!(text.contains("req-2"));
    }

    #[tokio::test]
    async fn test_get_recent_requests_filtered_by_app() {
        let ctx = make_test_ctx();
        let args = serde_json::json!({"app": "api"});
        let result = tool_get_recent_requests(&args, &ctx).await;
        assert!(result.is_error.is_none());

        let text = &result.content[0].text;
        assert!(text.contains("req-1"));
        assert!(!text.contains("req-2"));
    }

    #[tokio::test]
    async fn test_get_recent_requests_filtered_by_status_range() {
        let ctx = make_test_ctx();
        let args = serde_json::json!({"status_range": "5xx"});
        let result = tool_get_recent_requests(&args, &ctx).await;
        assert!(result.is_error.is_none());

        let text = &result.content[0].text;
        assert!(text.contains("req-1"));
        assert!(!text.contains("req-2"));
    }

    #[tokio::test]
    async fn test_get_request_detail() {
        let ctx = make_test_ctx();
        let args = serde_json::json!({"id": "req-1"});
        let result = tool_get_request_detail(&args, &ctx).await;
        assert!(result.is_error.is_none());

        let text = &result.content[0].text;
        assert!(text.contains("POST"));
        assert!(text.contains("/api/users"));
        assert!(text.contains("500"));
        assert!(text.contains("duplicate key"));
    }

    #[tokio::test]
    async fn test_get_request_detail_not_found() {
        let ctx = make_test_ctx();
        let args = serde_json::json!({"id": "nonexistent"});
        let result = tool_get_request_detail(&args, &ctx).await;
        assert_eq!(result.is_error, Some(true));
    }

    #[tokio::test]
    async fn test_replay_request() {
        let ctx = make_test_ctx();
        let args = serde_json::json!({"id": "req-1"});
        let result = tool_replay_request(&args, &ctx).await;
        assert!(result.is_error.is_none());

        let text = &result.content[0].text;
        assert!(text.contains("POST"));
        assert!(text.contains("/api/users"));
    }

    #[tokio::test]
    async fn test_replay_request_with_overrides() {
        let ctx = make_test_ctx();
        let args = serde_json::json!({
            "id": "req-1",
            "method": "PUT",
            "headers": {"Authorization": "Bearer test-token"}
        });
        let result = tool_replay_request(&args, &ctx).await;
        assert!(result.is_error.is_none());

        let text = &result.content[0].text;
        assert!(text.contains("PUT"));
        assert!(text.contains("Authorization"));
    }

    #[tokio::test]
    async fn test_restart_app() {
        let pm = make_test_pm();
        let args = serde_json::json!({"app": "api"});
        let result = tool_restart_app(&args, &pm).await;
        assert!(result.is_error.is_none());
        assert!(result.content[0].text.contains("restarted"));
    }

    #[tokio::test]
    async fn test_restart_app_not_found() {
        let pm = make_test_pm();
        let args = serde_json::json!({"app": "nonexistent"});
        let result = tool_restart_app(&args, &pm).await;
        assert_eq!(result.is_error, Some(true));
    }

    #[tokio::test]
    async fn test_get_app_schema() {
        let ctx = make_test_ctx();

        // First observe some traffic
        let record = RequestRecord {
            id: "s1".to_string(),
            app_name: "api".to_string(),
            timestamp: Utc::now(),
            duration_ms: 10,
            method: "GET".to_string(),
            url: "http://api.localhost:1337/api/users".to_string(),
            path: "/api/users".to_string(),
            query_string: "page=1".to_string(),
            request_headers: HashMap::new(),
            request_body: None,
            request_content_type: None,
            status_code: 200,
            status_message: String::new(),
            response_headers: HashMap::new(),
            response_body: None,
            response_content_type: None,
            mocked: false,
            parent_id: None,
        };
        ctx.schema_inference.observe(&record);

        let args = serde_json::json!({"app": "api"});
        let result = tool_get_app_schema(&args, &ctx).await;
        assert!(result.is_error.is_none());

        let text = &result.content[0].text;
        assert!(text.contains("/api/users"));
        assert!(text.contains("GET"));
    }

    #[tokio::test]
    async fn test_get_app_schema_no_traffic() {
        let ctx = make_test_ctx();
        let args = serde_json::json!({"app": "unknown"});
        let result = tool_get_app_schema(&args, &ctx).await;
        assert!(result.content[0].text.contains("No traffic"));
    }

    #[test]
    fn test_tool_definitions() {
        let defs = tool_definitions();
        assert_eq!(defs.len(), 7);

        let names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"list_apps"));
        assert!(names.contains(&"get_app_logs"));
        assert!(names.contains(&"get_recent_requests"));
        assert!(names.contains(&"get_request_detail"));
        assert!(names.contains(&"replay_request"));
        assert!(names.contains(&"restart_app"));
        assert!(names.contains(&"get_app_schema"));
    }
}
