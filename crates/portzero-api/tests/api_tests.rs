//! Integration tests for the PortZero API.
//!
//! Tests all endpoints using axum-test with in-memory SQLite.

use axum_test::TestServer;
use portzero_api::{build_router, AppState};
use portzero_core::types::*;
use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};

/// Create a test server with fresh in-memory state.
fn test_server() -> TestServer {
    let state = AppState::test();
    let app = build_router(state);
    TestServer::new(app).unwrap()
}

/// Create a test server and return it with the state for manipulation.
fn test_server_with_state() -> (TestServer, AppState) {
    let state = AppState::test();
    let app = build_router(state.clone());
    (TestServer::new(app).unwrap(), state)
}

/// Helper: insert a test request record into the store.
fn insert_test_request(
    state: &AppState,
    id: &str,
    app: &str,
    method: &str,
    path: &str,
    status: u16,
) {
    let record = portzero_core::types::RequestRecord {
        id: id.to_string(),
        app_name: app.to_string(),
        timestamp: chrono::Utc::now(),
        duration_ms: 42,
        method: method.to_string(),
        url: format!("http://{}.localhost:1337{}", app, path),
        path: path.to_string(),
        query_string: String::new(),
        request_headers: HashMap::new(),
        request_body: None,
        request_content_type: None,
        status_code: status,
        status_message: String::new(),
        response_headers: HashMap::new(),
        response_body: Some(b"test body".to_vec()),
        response_content_type: Some("text/plain".to_string()),
        mocked: false,
        parent_id: None,
    };
    state.store.insert_request(&record).unwrap();
}

/// Helper: register a test app in state.
fn register_test_app(state: &AppState, name: &str) {
    state.apps.insert(
        name.to_string(),
        AppInfo {
            name: name.to_string(),
            port: 4001,
            pid: Some(1234),
            command: vec!["node".to_string(), "server.js".to_string()],
            cwd: std::path::PathBuf::from("/tmp/test"),
            status: AppStatus::Running,
            started_at: Some(chrono::Utc::now()),
            restarts: 0,
            auto_restart: true,
            url: format!("http://{}.localhost:1337", name),
            cpu_percent: Some(5.2),
            memory_bytes: Some(50_000_000),
            tunnel_url: None,
        },
    );
}

// =========================================================================
// Status endpoint
// =========================================================================

#[tokio::test]
async fn test_get_status() {
    let server = test_server();
    let resp = server.get("/api/status").await;
    resp.assert_status_ok();

    let body: Value = resp.json();
    assert!(body["uptime_seconds"].is_number());
    assert_eq!(body["proxy_port"], 1337);
    assert!(body["version"].is_string());
    assert_eq!(body["app_count"], 0);
}

// =========================================================================
// Apps endpoints
// =========================================================================

#[tokio::test]
async fn test_list_apps_empty() {
    let server = test_server();
    let resp = server.get("/api/apps").await;
    resp.assert_status_ok();

    let body: Vec<Value> = resp.json();
    assert!(body.is_empty());
}

#[tokio::test]
async fn test_list_apps_with_data() {
    let (server, state) = test_server_with_state();
    register_test_app(&state, "web");
    register_test_app(&state, "api");

    let resp = server.get("/api/apps").await;
    resp.assert_status_ok();

    let body: Vec<Value> = resp.json();
    assert_eq!(body.len(), 2);
}

#[tokio::test]
async fn test_get_app_found() {
    let (server, state) = test_server_with_state();
    register_test_app(&state, "my-app");

    let resp = server.get("/api/apps/my-app").await;
    resp.assert_status_ok();

    let body: Value = resp.json();
    assert_eq!(body["name"], "my-app");
    assert_eq!(body["port"], 4001);
    assert_eq!(body["pid"], 1234);
}

#[tokio::test]
async fn test_get_app_not_found() {
    let server = test_server();
    let resp = server.get("/api/apps/nonexistent").await;
    resp.assert_status_not_found();
}

#[tokio::test]
async fn test_restart_app() {
    let (server, state) = test_server_with_state();
    register_test_app(&state, "web");

    let resp = server.post("/api/apps/web/restart").await;
    resp.assert_status_ok();

    let body: Value = resp.json();
    assert_eq!(body["status"], "restart_requested");
}

#[tokio::test]
async fn test_restart_app_not_found() {
    let server = test_server();
    let resp = server.post("/api/apps/nonexistent/restart").await;
    resp.assert_status_not_found();
}

#[tokio::test]
async fn test_stop_app() {
    let (server, state) = test_server_with_state();
    register_test_app(&state, "web");

    let resp = server.post("/api/apps/web/stop").await;
    resp.assert_status_ok();

    let body: Value = resp.json();
    assert_eq!(body["status"], "stop_requested");
}

#[tokio::test]
async fn test_get_app_logs_empty() {
    let (server, state) = test_server_with_state();
    register_test_app(&state, "web");

    let resp = server.get("/api/apps/web/logs").await;
    resp.assert_status_ok();

    let body: Vec<Value> = resp.json();
    assert!(body.is_empty());
}

#[tokio::test]
async fn test_get_app_logs_with_data() {
    let (server, state) = test_server_with_state();
    register_test_app(&state, "web");

    let mut logs = VecDeque::new();
    logs.push_back(LogLine {
        timestamp: chrono::Utc::now(),
        stream: LogStream::Stdout,
        content: "Server started on port 3000".to_string(),
    });
    logs.push_back(LogLine {
        timestamp: chrono::Utc::now(),
        stream: LogStream::Stderr,
        content: "Warning: deprecated API".to_string(),
    });
    state.logs.insert("web".to_string(), logs);

    let resp = server.get("/api/apps/web/logs").await;
    resp.assert_status_ok();

    let body: Vec<Value> = resp.json();
    assert_eq!(body.len(), 2);
    assert_eq!(body[0]["content"], "Server started on port 3000");
}

#[tokio::test]
async fn test_get_app_logs_with_limit() {
    let (server, state) = test_server_with_state();
    register_test_app(&state, "web");

    let mut logs = VecDeque::new();
    for i in 0..10 {
        logs.push_back(LogLine {
            timestamp: chrono::Utc::now(),
            stream: LogStream::Stdout,
            content: format!("Line {}", i),
        });
    }
    state.logs.insert("web".to_string(), logs);

    let resp = server.get("/api/apps/web/logs?lines=3").await;
    resp.assert_status_ok();

    let body: Vec<Value> = resp.json();
    assert_eq!(body.len(), 3);
}

// =========================================================================
// Request traffic endpoints
// =========================================================================

#[tokio::test]
async fn test_list_requests_empty() {
    let server = test_server();
    let resp = server.get("/api/requests").await;
    resp.assert_status_ok();

    let body: Vec<Value> = resp.json();
    assert!(body.is_empty());
}

#[tokio::test]
async fn test_list_requests_with_data() {
    let (server, state) = test_server_with_state();
    insert_test_request(&state, "r1", "web", "GET", "/", 200);
    insert_test_request(&state, "r2", "api", "POST", "/api/users", 201);
    insert_test_request(&state, "r3", "api", "GET", "/api/users/1", 404);

    let resp = server.get("/api/requests").await;
    resp.assert_status_ok();

    let body: Vec<Value> = resp.json();
    assert_eq!(body.len(), 3);
}

#[tokio::test]
async fn test_list_requests_filter_by_app() {
    let (server, state) = test_server_with_state();
    insert_test_request(&state, "r1", "web", "GET", "/", 200);
    insert_test_request(&state, "r2", "api", "POST", "/api/users", 201);

    let resp = server.get("/api/requests?app=api").await;
    resp.assert_status_ok();

    let body: Vec<Value> = resp.json();
    assert_eq!(body.len(), 1);
    assert_eq!(body[0]["app_name"], "api");
}

#[tokio::test]
async fn test_list_requests_filter_by_method() {
    let (server, state) = test_server_with_state();
    insert_test_request(&state, "r1", "web", "GET", "/", 200);
    insert_test_request(&state, "r2", "api", "POST", "/api/users", 201);

    let resp = server.get("/api/requests?method=POST").await;
    resp.assert_status_ok();

    let body: Vec<Value> = resp.json();
    assert_eq!(body.len(), 1);
    assert_eq!(body[0]["method"], "POST");
}

#[tokio::test]
async fn test_list_requests_pagination() {
    let (server, state) = test_server_with_state();
    for i in 0..10 {
        insert_test_request(&state, &format!("r{}", i), "web", "GET", "/", 200);
    }

    let resp = server.get("/api/requests?limit=3&offset=0").await;
    resp.assert_status_ok();

    let body: Vec<Value> = resp.json();
    assert_eq!(body.len(), 3);
}

#[tokio::test]
async fn test_get_request_detail() {
    let (server, state) = test_server_with_state();
    insert_test_request(&state, "req-123", "web", "GET", "/api/hello", 200);

    let resp = server.get("/api/requests/req-123").await;
    resp.assert_status_ok();

    let body: Value = resp.json();
    assert_eq!(body["id"], "req-123");
    assert_eq!(body["method"], "GET");
    assert_eq!(body["path"], "/api/hello");
    assert_eq!(body["status_code"], 200);
}

#[tokio::test]
async fn test_get_request_not_found() {
    let server = test_server();
    let resp = server.get("/api/requests/nonexistent").await;
    resp.assert_status_not_found();
}

#[tokio::test]
async fn test_replay_request() {
    let (server, state) = test_server_with_state();
    insert_test_request(&state, "orig-1", "web", "POST", "/api/submit", 200);

    let resp = server
        .post("/api/requests/orig-1/replay")
        .json(&json!({}))
        .await;
    resp.assert_status(axum::http::StatusCode::CREATED);

    let body: Value = resp.json();
    // The replay endpoint now returns the full RequestRecord
    assert!(body["id"].is_string());
    assert_eq!(body["parent_id"], "orig-1");
    assert_eq!(body["app_name"], "web");
    assert_eq!(body["method"], "POST");
}

#[tokio::test]
async fn test_replay_request_not_found() {
    let server = test_server();
    let resp = server
        .post("/api/requests/nonexistent/replay")
        .json(&json!({}))
        .await;
    resp.assert_status_not_found();
}

#[tokio::test]
async fn test_delete_requests() {
    let (server, state) = test_server_with_state();
    insert_test_request(&state, "r1", "web", "GET", "/", 200);
    insert_test_request(&state, "r2", "api", "GET", "/", 200);

    let resp = server.delete("/api/requests").await;
    resp.assert_status_ok();

    let body: Value = resp.json();
    assert_eq!(body["deleted"], 2);

    // Verify empty
    let resp2 = server.get("/api/requests").await;
    let body2: Vec<Value> = resp2.json();
    assert!(body2.is_empty());
}

#[tokio::test]
async fn test_delete_requests_by_app() {
    let (server, state) = test_server_with_state();
    insert_test_request(&state, "r1", "web", "GET", "/", 200);
    insert_test_request(&state, "r2", "api", "GET", "/", 200);

    let resp = server.delete("/api/requests?app=web").await;
    resp.assert_status_ok();

    let body: Value = resp.json();
    assert_eq!(body["deleted"], 1);
}

#[tokio::test]
async fn test_diff_requests() {
    let (server, state) = test_server_with_state();
    insert_test_request(&state, "left-1", "web", "GET", "/api/test", 200);
    insert_test_request(&state, "right-1", "web", "GET", "/api/test", 500);

    let resp = server.get("/api/requests/left-1/diff/right-1").await;
    resp.assert_status_ok();

    let body: Value = resp.json();
    assert_eq!(body["left"]["id"], "left-1");
    assert_eq!(body["left"]["status_code"], 200);
    assert_eq!(body["right"]["id"], "right-1");
    assert_eq!(body["right"]["status_code"], 500);
}

#[tokio::test]
async fn test_diff_request_not_found() {
    let (server, state) = test_server_with_state();
    insert_test_request(&state, "exists", "web", "GET", "/", 200);

    let resp = server.get("/api/requests/exists/diff/missing").await;
    resp.assert_status_not_found();
}

// =========================================================================
// Mock endpoints
// =========================================================================

#[tokio::test]
async fn test_list_mocks_empty() {
    let server = test_server();
    let resp = server.get("/api/mocks").await;
    resp.assert_status_ok();

    let body: Vec<Value> = resp.json();
    assert!(body.is_empty());
}

#[tokio::test]
async fn test_create_mock() {
    let server = test_server();
    let resp = server
        .post("/api/mocks")
        .json(&json!({
            "app_name": "api",
            "method": "POST",
            "path_pattern": "/api/payments",
            "status_code": 500,
            "response_body": "{\"error\":\"declined\"}",
            "enabled": true
        }))
        .await;

    resp.assert_status(axum::http::StatusCode::CREATED);

    let body: Value = resp.json();
    assert_eq!(body["app_name"], "api");
    assert_eq!(body["status_code"], 500);
    assert!(body["id"].is_string());
}

#[tokio::test]
async fn test_create_and_list_mocks() {
    let server = test_server();

    // Create two mocks
    server
        .post("/api/mocks")
        .json(&json!({
            "app_name": "api",
            "path_pattern": "/api/users",
            "status_code": 200,
            "response_body": "[]"
        }))
        .await;
    server
        .post("/api/mocks")
        .json(&json!({
            "app_name": "web",
            "path_pattern": "/health",
            "status_code": 200,
            "response_body": "ok"
        }))
        .await;

    let resp = server.get("/api/mocks").await;
    resp.assert_status_ok();

    let body: Vec<Value> = resp.json();
    assert_eq!(body.len(), 2);
}

#[tokio::test]
async fn test_update_mock() {
    let server = test_server();

    // Create a mock
    let create_resp = server
        .post("/api/mocks")
        .json(&json!({
            "app_name": "api",
            "path_pattern": "/api/users",
            "status_code": 200,
            "response_body": "[]"
        }))
        .await;
    let created: Value = create_resp.json();
    let mock_id = created["id"].as_str().unwrap();

    // Update it
    let resp = server
        .put(&format!("/api/mocks/{}", mock_id))
        .json(&json!({
            "status_code": 404,
            "response_body": "{\"error\":\"not found\"}"
        }))
        .await;
    resp.assert_status_ok();

    let body: Value = resp.json();
    assert_eq!(body["status_code"], 404);
    assert_eq!(body["response_body"], "{\"error\":\"not found\"}");
}

#[tokio::test]
async fn test_delete_mock() {
    let server = test_server();

    let create_resp = server
        .post("/api/mocks")
        .json(&json!({
            "app_name": "api",
            "path_pattern": "/test",
            "status_code": 200,
            "response_body": "ok"
        }))
        .await;
    let created: Value = create_resp.json();
    let mock_id = created["id"].as_str().unwrap();

    let resp = server.delete(&format!("/api/mocks/{}", mock_id)).await;
    resp.assert_status(axum::http::StatusCode::NO_CONTENT);

    // Verify it's gone
    let list_resp = server.get("/api/mocks").await;
    let mocks: Vec<Value> = list_resp.json();
    assert!(mocks.is_empty());
}

#[tokio::test]
async fn test_delete_mock_not_found() {
    let server = test_server();
    let resp = server.delete("/api/mocks/nonexistent").await;
    resp.assert_status_not_found();
}

#[tokio::test]
async fn test_toggle_mock() {
    let server = test_server();

    let create_resp = server
        .post("/api/mocks")
        .json(&json!({
            "app_name": "api",
            "path_pattern": "/test",
            "status_code": 200,
            "response_body": "ok",
            "enabled": true
        }))
        .await;
    let created: Value = create_resp.json();
    let mock_id = created["id"].as_str().unwrap();

    // Toggle: true → false
    let resp = server
        .patch(&format!("/api/mocks/{}/toggle", mock_id))
        .await;
    resp.assert_status_ok();

    let body: Value = resp.json();
    assert_eq!(body["enabled"], false);

    // Toggle back: false → true
    let resp2 = server
        .patch(&format!("/api/mocks/{}/toggle", mock_id))
        .await;
    resp2.assert_status_ok();

    let body2: Value = resp2.json();
    assert_eq!(body2["enabled"], true);
}

// =========================================================================
// Network simulation endpoints
// =========================================================================

#[tokio::test]
async fn test_get_network_profile_empty() {
    let server = test_server();
    let resp = server.get("/api/network/my-app").await;
    resp.assert_status_ok();

    let body: Value = resp.json();
    assert!(body.is_null());
}

#[tokio::test]
async fn test_set_network_profile() {
    let server = test_server();
    let resp = server
        .put("/api/network/my-app")
        .json(&json!({
            "latency_ms": 200,
            "packet_loss_rate": 0.1,
            "bandwidth_limit": 50000
        }))
        .await;
    resp.assert_status_ok();

    let body: Value = resp.json();
    assert_eq!(body["app_name"], "my-app");
    assert_eq!(body["latency_ms"], 200);
    assert_eq!(body["packet_loss_rate"], 0.1);
}

#[tokio::test]
async fn test_get_network_profile_after_set() {
    let (server, state) = test_server_with_state();

    state.network_profiles.insert(
        "my-app".to_string(),
        NetworkProfile {
            app_name: "my-app".to_string(),
            latency_ms: Some(500),
            jitter_ms: Some(50),
            packet_loss_rate: 0.0,
            bandwidth_limit: None,
            path_filter: None,
        },
    );

    let resp = server.get("/api/network/my-app").await;
    resp.assert_status_ok();

    let body: Value = resp.json();
    assert_eq!(body["latency_ms"], 500);
    assert_eq!(body["latency_jitter_ms"], 50);
}

#[tokio::test]
async fn test_clear_network_profile() {
    let (server, state) = test_server_with_state();

    state.network_profiles.insert(
        "my-app".to_string(),
        NetworkProfile {
            app_name: "my-app".to_string(),
            latency_ms: Some(200),
            jitter_ms: None,
            packet_loss_rate: 0.0,
            bandwidth_limit: None,
            path_filter: None,
        },
    );

    let resp = server.delete("/api/network/my-app").await;
    resp.assert_status(axum::http::StatusCode::NO_CONTENT);

    // Verify cleared
    let resp2 = server.get("/api/network/my-app").await;
    let body: Value = resp2.json();
    assert!(body.is_null());
}

// =========================================================================
// Schema endpoint
// =========================================================================

#[tokio::test]
async fn test_get_schema_app_not_found() {
    let server = test_server();
    let resp = server.get("/api/apps/nonexistent/schema").await;
    resp.assert_status_not_found();
}

#[tokio::test]
async fn test_get_schema_no_data() {
    let (server, state) = test_server_with_state();
    register_test_app(&state, "web");

    let resp = server.get("/api/apps/web/schema").await;
    resp.assert_status_ok();

    let body: Value = resp.json();
    assert_eq!(body["app_name"], "web");
    assert!(body["endpoints"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_get_schema_with_data() {
    let (server, state) = test_server_with_state();
    register_test_app(&state, "api");

    state.schemas.insert(
        "api".to_string(),
        InferredSchema {
            app_name: "api".to_string(),
            endpoints: vec![InferredEndpoint {
                method: "GET".to_string(),
                path_template: "/api/users/:id".to_string(),
                query_params: vec![],
                request_body_schema: None,
                response_schemas: HashMap::new(),
                sample_count: 15,
            }],
            last_updated: chrono::Utc::now(),
        },
    );

    let resp = server.get("/api/apps/api/schema").await;
    resp.assert_status_ok();

    let body: Value = resp.json();
    assert_eq!(body["endpoints"].as_array().unwrap().len(), 1);
    assert_eq!(body["endpoints"][0]["path_template"], "/api/users/:id");
}

// =========================================================================
// Tunnel endpoints (behind "tunnel" feature flag)
// =========================================================================

#[cfg(feature = "tunnel")]
#[tokio::test]
async fn test_start_tunnel_app_not_found() {
    let server = test_server();
    let resp = server
        .post("/api/apps/nonexistent/share")
        .json(&json!({}))
        .await;
    resp.assert_status_not_found();
}

#[cfg(feature = "tunnel")]
#[tokio::test]
async fn test_start_tunnel() {
    let (server, state) = test_server_with_state();
    register_test_app(&state, "web");

    let resp = server.post("/api/apps/web/share").json(&json!({})).await;
    resp.assert_status(axum::http::StatusCode::CREATED);

    let body: Value = resp.json();
    assert_eq!(body["app_name"], "web");
    assert!(body["public_url"].as_str().unwrap().contains("web"));
}

#[cfg(feature = "tunnel")]
#[tokio::test]
async fn test_start_tunnel_custom_subdomain() {
    let (server, state) = test_server_with_state();
    register_test_app(&state, "web");

    let resp = server
        .post("/api/apps/web/share")
        .json(&json!({ "subdomain": "my-custom" }))
        .await;
    resp.assert_status(axum::http::StatusCode::CREATED);

    let body: Value = resp.json();
    assert!(body["public_url"].as_str().unwrap().contains("my-custom"));
}

#[cfg(feature = "tunnel")]
#[tokio::test]
async fn test_start_tunnel_conflict() {
    let (server, state) = test_server_with_state();
    register_test_app(&state, "web");

    // First share succeeds
    server.post("/api/apps/web/share").json(&json!({})).await;

    // Second share conflicts
    let resp = server.post("/api/apps/web/share").json(&json!({})).await;
    resp.assert_status(axum::http::StatusCode::CONFLICT);
}

#[cfg(feature = "tunnel")]
#[tokio::test]
async fn test_stop_tunnel() {
    let (server, state) = test_server_with_state();
    register_test_app(&state, "web");

    // Start tunnel
    server.post("/api/apps/web/share").json(&json!({})).await;

    // Stop tunnel
    let resp = server.delete("/api/apps/web/share").await;
    resp.assert_status(axum::http::StatusCode::NO_CONTENT);

    // Verify it's gone
    assert!(!state.tunnels.contains_key("web"));
}

#[cfg(feature = "tunnel")]
#[tokio::test]
async fn test_stop_tunnel_not_found() {
    let server = test_server();
    let resp = server.delete("/api/apps/nonexistent/share").await;
    resp.assert_status_not_found();
}

// =========================================================================
// Dashboard fallback
// =========================================================================

#[tokio::test]
async fn test_dashboard_index() {
    let server = test_server();
    let resp = server.get("/").await;
    resp.assert_status_ok();

    let body = resp.text();
    assert!(body.contains("PortZero Dashboard"));
}

#[tokio::test]
async fn test_dashboard_spa_fallback() {
    let server = test_server();
    // Unknown path should return index.html (SPA client-side routing)
    let resp = server.get("/traffic/some-id").await;
    resp.assert_status_ok();

    let body = resp.text();
    assert!(body.contains("PortZero Dashboard"));
}
