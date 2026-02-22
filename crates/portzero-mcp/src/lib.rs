//! PortZero MCP (Model Context Protocol) server.
//!
//! Exposes PortZero as an MCP server so AI coding agents can programmatically
//! inspect traffic, manage apps, replay requests, and read logs.
//!
//! # Transport
//!
//! Uses stdio (stdin/stdout) with JSON-RPC 2.0 messages, per the MCP spec.
//!
//! # Configuration
//!
//! Add to your agent's MCP settings:
//!
//! ```json
//! {
//!   "mcpServers": {
//!     "portzero": {
//!       "command": "portzero",
//!       "args": ["mcp"]
//!     }
//!   }
//! }
//! ```

pub mod tools;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing;

pub use tools::{ProcessManagerOps, StubProcessManager, ToolContext};

// ---------------------------------------------------------------------------
// JSON-RPC 2.0 types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

impl JsonRpcResponse {
    fn success(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    fn error(id: Value, code: i32, message: String) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message,
                data: None,
            }),
        }
    }

    fn method_not_found(id: Value, method: &str) -> Self {
        Self::error(id, -32601, format!("Method not found: {}", method))
    }
}

// ---------------------------------------------------------------------------
// MCP protocol constants
// ---------------------------------------------------------------------------

const MCP_PROTOCOL_VERSION: &str = "2024-11-05";
const SERVER_NAME: &str = "portzero";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

// ---------------------------------------------------------------------------
// MCP Server
// ---------------------------------------------------------------------------

/// The MCP server. Call [`McpServer::run`] to start handling stdio messages.
pub struct McpServer {
    tool_ctx: Arc<ToolContext>,
    process_manager: Arc<dyn ProcessManagerOps>,
}

impl McpServer {
    /// Create a new MCP server with the given context.
    pub fn new(tool_ctx: ToolContext, process_manager: Arc<dyn ProcessManagerOps>) -> Self {
        Self {
            tool_ctx: Arc::new(tool_ctx),
            process_manager,
        }
    }

    /// Run the MCP server, reading JSON-RPC messages from stdin and writing
    /// responses to stdout. Runs until stdin is closed.
    pub async fn run(&self) -> anyhow::Result<()> {
        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let reader = BufReader::new(stdin);
        let mut lines = reader.lines();

        tracing::info!("MCP server started, waiting for messages on stdin");

        while let Some(line) = lines.next_line().await? {
            let line = line.trim().to_string();
            if line.is_empty() {
                continue;
            }

            tracing::debug!(message = %line, "Received MCP message");

            let request: JsonRpcRequest = match serde_json::from_str(&line) {
                Ok(req) => req,
                Err(e) => {
                    let response = JsonRpcResponse::error(
                        Value::Null,
                        -32700,
                        format!("Parse error: {}", e),
                    );
                    let json = serde_json::to_string(&response)?;
                    stdout.write_all(json.as_bytes()).await?;
                    stdout.write_all(b"\n").await?;
                    stdout.flush().await?;
                    continue;
                }
            };

            let response = self.handle_request(request).await;

            if let Some(response) = response {
                let json = serde_json::to_string(&response)?;
                tracing::debug!(response = %json, "Sending MCP response");
                stdout.write_all(json.as_bytes()).await?;
                stdout.write_all(b"\n").await?;
                stdout.flush().await?;
            }
        }

        tracing::info!("MCP server stdin closed, shutting down");
        Ok(())
    }

    /// Handle a single JSON-RPC request.
    async fn handle_request(&self, request: JsonRpcRequest) -> Option<JsonRpcResponse> {
        let id = request.id.clone().unwrap_or(Value::Null);

        // Notifications (no id) don't get responses per JSON-RPC spec
        if request.id.is_none() {
            // Handle notification methods
            match request.method.as_str() {
                "notifications/initialized" => {
                    tracing::info!("MCP client initialized");
                    return None;
                }
                "notifications/cancelled" => {
                    tracing::info!("MCP request cancelled");
                    return None;
                }
                _ => {
                    tracing::debug!(method = %request.method, "Unknown notification");
                    return None;
                }
            }
        }

        let response = match request.method.as_str() {
            "initialize" => self.handle_initialize(id).await,
            "tools/list" => self.handle_tools_list(id).await,
            "tools/call" => self.handle_tools_call(id, &request.params).await,
            "ping" => JsonRpcResponse::success(id, serde_json::json!({})),
            _ => JsonRpcResponse::method_not_found(id, &request.method),
        };

        Some(response)
    }

    /// Handle `initialize` -- return server capabilities.
    async fn handle_initialize(&self, id: Value) -> JsonRpcResponse {
        JsonRpcResponse::success(
            id,
            serde_json::json!({
                "protocolVersion": MCP_PROTOCOL_VERSION,
                "capabilities": {
                    "tools": {
                        "listChanged": false
                    }
                },
                "serverInfo": {
                    "name": SERVER_NAME,
                    "version": SERVER_VERSION
                }
            }),
        )
    }

    /// Handle `tools/list` -- return all available tools.
    async fn handle_tools_list(&self, id: Value) -> JsonRpcResponse {
        let definitions = tools::tool_definitions();
        JsonRpcResponse::success(
            id,
            serde_json::json!({
                "tools": definitions
            }),
        )
    }

    /// Handle `tools/call` -- execute a tool.
    async fn handle_tools_call(&self, id: Value, params: &Value) -> JsonRpcResponse {
        let tool_name = match params.get("name").and_then(|v| v.as_str()) {
            Some(name) => name,
            None => {
                return JsonRpcResponse::error(
                    id,
                    -32602,
                    "Missing 'name' parameter".to_string(),
                );
            }
        };

        let arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or(serde_json::json!({}));

        let result = tools::execute_tool(
            tool_name,
            &arguments,
            &self.tool_ctx,
            self.process_manager.as_ref(),
        )
        .await;

        let response_value = serde_json::to_value(&result)
            .unwrap_or_else(|e| serde_json::json!({"error": e.to_string()}));

        JsonRpcResponse::success(id, response_value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use portzero_core::store::Store;
    use portzero_core::SchemaInference;

    fn make_test_server() -> McpServer {
        let store = Store::in_memory().unwrap();
        let tool_ctx = ToolContext {
            store: Arc::new(store),
            schema_inference: Arc::new(SchemaInference::new()),
        };
        McpServer::new(tool_ctx, Arc::new(StubProcessManager))
    }

    #[tokio::test]
    async fn test_initialize() {
        let server = make_test_server();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(Value::Number(1.into())),
            method: "initialize".to_string(),
            params: serde_json::json!({}),
        };

        let response = server.handle_request(request).await.unwrap();
        assert!(response.error.is_none());

        let result = response.result.unwrap();
        assert_eq!(result["protocolVersion"], MCP_PROTOCOL_VERSION);
        assert_eq!(result["serverInfo"]["name"], SERVER_NAME);
        assert!(result["capabilities"]["tools"].is_object());
    }

    #[tokio::test]
    async fn test_tools_list() {
        let server = make_test_server();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(Value::Number(2.into())),
            method: "tools/list".to_string(),
            params: serde_json::json!({}),
        };

        let response = server.handle_request(request).await.unwrap();
        assert!(response.error.is_none());

        let result = response.result.unwrap();
        let tools = result["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 7);
    }

    #[tokio::test]
    async fn test_tools_call_list_apps() {
        let server = make_test_server();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(Value::Number(3.into())),
            method: "tools/call".to_string(),
            params: serde_json::json!({
                "name": "list_apps",
                "arguments": {}
            }),
        };

        let response = server.handle_request(request).await.unwrap();
        assert!(response.error.is_none());

        let result = response.result.unwrap();
        // StubProcessManager returns empty list
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("No apps"));
    }

    #[tokio::test]
    async fn test_tools_call_unknown_tool() {
        let server = make_test_server();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(Value::Number(4.into())),
            method: "tools/call".to_string(),
            params: serde_json::json!({
                "name": "nonexistent_tool",
                "arguments": {}
            }),
        };

        let response = server.handle_request(request).await.unwrap();
        assert!(response.error.is_none());

        let result = response.result.unwrap();
        assert_eq!(result["isError"], true);
    }

    #[tokio::test]
    async fn test_method_not_found() {
        let server = make_test_server();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(Value::Number(5.into())),
            method: "nonexistent/method".to_string(),
            params: serde_json::json!({}),
        };

        let response = server.handle_request(request).await.unwrap();
        assert!(response.error.is_some());
        assert_eq!(response.error.unwrap().code, -32601);
    }

    #[tokio::test]
    async fn test_notification_no_response() {
        let server = make_test_server();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: None, // Notification -- no id
            method: "notifications/initialized".to_string(),
            params: serde_json::json!({}),
        };

        let response = server.handle_request(request).await;
        assert!(response.is_none());
    }

    #[tokio::test]
    async fn test_ping() {
        let server = make_test_server();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(Value::Number(6.into())),
            method: "ping".to_string(),
            params: serde_json::json!({}),
        };

        let response = server.handle_request(request).await.unwrap();
        assert!(response.error.is_none());
        assert_eq!(response.result.unwrap(), serde_json::json!({}));
    }
}
