//! Integration tests for the control socket protocol.
//!
//! Spins up a real control socket server in a temp directory and tests
//! the full client ↔ server flow over Unix sockets.

use portzero_core::control::{self, ControlClient, ControlRequest, ControlResponse};
use portzero_core::log_store::LogStore;
use portzero_core::mock_engine::MockEngine;
use portzero_core::network_sim::NetworkSim;
use portzero_core::router::Router;
use portzero_core::store::Store;
use portzero_core::tunnel::TunnelManager;
use portzero_core::types::{CreateMockRule, SetNetworkProfile, UpdateMockRule, DEFAULT_PROXY_PORT};
use portzero_core::ws::WsHub;
use std::collections::HashMap;
use std::sync::Arc;
use tempfile::TempDir;

/// Spin up a control socket server in a temp directory and return
/// (temp_dir, client). The server runs in a background task.
async fn setup() -> (TempDir, ControlClient) {
    let tmp = TempDir::new().unwrap();
    let state_dir = tmp.path().to_path_buf();

    let db_path = state_dir.join("portzero.db");
    let router = Arc::new(Router::new());
    let ws_hub = Arc::new(WsHub::new());
    let log_store = Arc::new(LogStore::new());
    let network_sim = Arc::new(NetworkSim::new());
    let mock_engine = Arc::new(MockEngine::new(None));
    let store = Arc::new(Store::open(&db_path).unwrap());
    let tunnel_manager = Arc::new(TunnelManager::stub(Some((*ws_hub).clone())));

    let sd = state_dir.clone();
    tokio::spawn(async move {
        control::serve_control_socket(
            &sd,
            router,
            ws_hub,
            log_store,
            network_sim,
            mock_engine,
            store,
            tunnel_manager,
            DEFAULT_PROXY_PORT,
        )
        .await
        .unwrap();
    });

    // Wait a beat for the listener to bind
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let client = ControlClient::connect(tmp.path()).await.unwrap();
    (tmp, client)
}

// ---------------------------------------------------------------------------
// Ping
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_ping() {
    let (_tmp, mut client) = setup().await;
    assert!(client.ping().await);
}

// ---------------------------------------------------------------------------
// Register / Deregister / List
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_list_empty() {
    let (_tmp, mut client) = setup().await;

    let resp = client.request(&ControlRequest::List).await.unwrap();
    match resp {
        ControlResponse::Apps { apps } => {
            assert!(apps.is_empty());
        }
        other => panic!("expected Apps, got {:?}", other),
    }
}

#[tokio::test]
async fn test_register_and_list() {
    let (_tmp, mut client) = setup().await;

    client
        .register(
            "my-app",
            3000,
            1234,
            &["node".into(), "server.js".into()],
            std::path::Path::new("/tmp"),
        )
        .await
        .unwrap();

    let resp = client.request(&ControlRequest::List).await.unwrap();
    match resp {
        ControlResponse::Apps { apps } => {
            assert_eq!(apps.len(), 1);
            assert_eq!(apps[0].name, "my-app");
            assert_eq!(apps[0].port, 3000);
            assert_eq!(apps[0].pid, 1234);
            assert!(apps[0].url.contains("my-app"));
        }
        other => panic!("expected Apps, got {:?}", other),
    }
}

#[tokio::test]
async fn test_register_multiple_and_list() {
    let (_tmp, mut client) = setup().await;

    client
        .register(
            "app-a",
            3000,
            100,
            &["cmd-a".into()],
            std::path::Path::new("/tmp"),
        )
        .await
        .unwrap();
    client
        .register(
            "app-b",
            3001,
            101,
            &["cmd-b".into()],
            std::path::Path::new("/tmp"),
        )
        .await
        .unwrap();

    let resp = client.request(&ControlRequest::List).await.unwrap();
    match resp {
        ControlResponse::Apps { apps } => {
            assert_eq!(apps.len(), 2);
            let names: Vec<&str> = apps.iter().map(|a| a.name.as_str()).collect();
            assert!(names.contains(&"app-a"));
            assert!(names.contains(&"app-b"));
        }
        other => panic!("expected Apps, got {:?}", other),
    }
}

#[tokio::test]
async fn test_deregister() {
    let (_tmp, mut client) = setup().await;

    client
        .register(
            "my-app",
            3000,
            1234,
            &["cmd".into()],
            std::path::Path::new("/tmp"),
        )
        .await
        .unwrap();
    client.deregister("my-app").await.unwrap();

    let resp = client.request(&ControlRequest::List).await.unwrap();
    match resp {
        ControlResponse::Apps { apps } => {
            assert!(apps.is_empty());
        }
        other => panic!("expected Apps, got {:?}", other),
    }
}

#[tokio::test]
async fn test_deregister_nonexistent() {
    let (_tmp, mut client) = setup().await;
    // Should not error — just no-op
    client.deregister("nonexistent").await.unwrap();
}

// ---------------------------------------------------------------------------
// Port allocation
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_allocate_port() {
    let (_tmp, mut client) = setup().await;
    let port = client.allocate_port("my-app").await.unwrap();
    assert!(port > 0);
}

#[tokio::test]
async fn test_allocate_port_deterministic() {
    let (_tmp, mut client) = setup().await;
    let port1 = client.allocate_port("my-app").await.unwrap();
    let port2 = client.allocate_port("my-app").await.unwrap();
    // Same name should get the same port
    assert_eq!(port1, port2);
}

#[tokio::test]
async fn test_allocate_port_different_names() {
    let (_tmp, mut client) = setup().await;
    let port_a = client.allocate_port("app-a").await.unwrap();
    let port_b = client.allocate_port("app-b").await.unwrap();
    // Different names should get different ports
    assert_ne!(port_a, port_b);
}

// ---------------------------------------------------------------------------
// Logs
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_log_append_and_get() {
    let (_tmp, mut client) = setup().await;

    client
        .log_append("my-app", "stdout", "hello world")
        .await
        .unwrap();
    client
        .log_append("my-app", "stderr", "error msg")
        .await
        .unwrap();

    let logs = client.get_logs("my-app", None).await.unwrap();
    assert_eq!(logs.len(), 2);
    assert_eq!(logs[0].content, "hello world");
    assert_eq!(logs[1].content, "error msg");
}

#[tokio::test]
async fn test_log_get_with_limit() {
    let (_tmp, mut client) = setup().await;

    for i in 0..10 {
        client
            .log_append("my-app", "stdout", &format!("line {}", i))
            .await
            .unwrap();
    }

    let logs = client.get_logs("my-app", Some(3)).await.unwrap();
    assert_eq!(logs.len(), 3);
    // Should return the last 3 lines
    assert_eq!(logs[0].content, "line 7");
    assert_eq!(logs[1].content, "line 8");
    assert_eq!(logs[2].content, "line 9");
}

#[tokio::test]
async fn test_log_empty() {
    let (_tmp, mut client) = setup().await;
    let logs = client.get_logs("nonexistent", None).await.unwrap();
    assert!(logs.is_empty());
}

// ---------------------------------------------------------------------------
// Mock rules
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_create_and_list_mocks() {
    let (_tmp, mut client) = setup().await;

    let rule = CreateMockRule {
        app_name: "my-app".to_string(),
        method: Some("GET".to_string()),
        path_pattern: "/api/health".to_string(),
        status_code: 200,
        response_headers: HashMap::new(),
        response_body: r#"{"status":"ok"}"#.to_string(),
        enabled: true,
    };

    let mock = client.create_mock(rule).await.unwrap();
    assert_eq!(mock.app_name, "my-app");
    assert_eq!(mock.status_code, 200);
    assert!(mock.enabled);

    let mocks = client.list_mocks().await.unwrap();
    assert_eq!(mocks.len(), 1);
    assert_eq!(mocks[0].id, mock.id);
}

#[tokio::test]
async fn test_update_mock() {
    let (_tmp, mut client) = setup().await;

    let rule = CreateMockRule {
        app_name: "my-app".to_string(),
        method: Some("POST".to_string()),
        path_pattern: "/api/test".to_string(),
        status_code: 201,
        response_headers: HashMap::new(),
        response_body: "".to_string(),
        enabled: true,
    };

    let mock = client.create_mock(rule).await.unwrap();

    let updated = client
        .update_mock(
            &mock.id,
            UpdateMockRule {
                method: None,
                path_pattern: None,
                status_code: Some(500),
                response_headers: None,
                response_body: None,
                enabled: Some(false),
            },
        )
        .await
        .unwrap();

    assert_eq!(updated.status_code, 500);
    assert!(!updated.enabled);
}

#[tokio::test]
async fn test_delete_mock() {
    let (_tmp, mut client) = setup().await;

    let rule = CreateMockRule {
        app_name: "my-app".to_string(),
        method: Some("GET".to_string()),
        path_pattern: "/test".to_string(),
        status_code: 200,
        response_headers: HashMap::new(),
        response_body: "".to_string(),
        enabled: true,
    };

    let mock = client.create_mock(rule).await.unwrap();
    client.delete_mock(&mock.id).await.unwrap();

    let mocks = client.list_mocks().await.unwrap();
    assert!(mocks.is_empty());
}

#[tokio::test]
async fn test_toggle_mock() {
    let (_tmp, mut client) = setup().await;

    let rule = CreateMockRule {
        app_name: "my-app".to_string(),
        method: Some("GET".to_string()),
        path_pattern: "/test".to_string(),
        status_code: 200,
        response_headers: HashMap::new(),
        response_body: "".to_string(),
        enabled: true,
    };

    let mock = client.create_mock(rule).await.unwrap();
    assert!(mock.enabled);

    let toggled = client.toggle_mock(&mock.id).await.unwrap();
    assert!(!toggled.enabled);

    let toggled2 = client.toggle_mock(&mock.id).await.unwrap();
    assert!(toggled2.enabled);
}

// ---------------------------------------------------------------------------
// Network simulation
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_set_and_get_network_profile() {
    let (_tmp, mut client) = setup().await;

    let profile = SetNetworkProfile {
        latency_ms: Some(500),
        jitter_ms: Some(50),
        packet_loss_rate: Some(0.1),
        bandwidth_limit: None,
        path_filter: None,
    };

    let result = client.set_network_profile("my-app", profile).await.unwrap();
    assert_eq!(result.app_name, "my-app");
    assert_eq!(result.latency_ms, Some(500));
    assert_eq!(result.jitter_ms, Some(50));

    let got = client.get_network_profile("my-app").await.unwrap();
    assert_eq!(got.latency_ms, Some(500));
}

#[tokio::test]
async fn test_clear_network_profile() {
    let (_tmp, mut client) = setup().await;

    let profile = SetNetworkProfile {
        latency_ms: Some(1000),
        jitter_ms: None,
        packet_loss_rate: None,
        bandwidth_limit: None,
        path_filter: None,
    };
    client.set_network_profile("my-app", profile).await.unwrap();
    client.clear_network_profile("my-app").await.unwrap();

    let got = client.get_network_profile("my-app").await.unwrap();
    assert_eq!(got.latency_ms, None);
}

#[tokio::test]
async fn test_get_network_profile_default() {
    let (_tmp, mut client) = setup().await;

    // Getting profile for app with no profile set should return defaults
    let got = client.get_network_profile("no-profile-app").await.unwrap();
    assert_eq!(got.latency_ms, None);
    assert_eq!(got.jitter_ms, None);
    assert_eq!(got.bandwidth_limit, None);
}

// ---------------------------------------------------------------------------
// Tunnels (stub backend)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_list_tunnels_empty() {
    let (_tmp, mut client) = setup().await;
    let tunnels = client.list_tunnels().await.unwrap();
    assert!(tunnels.is_empty());
}

// ---------------------------------------------------------------------------
// Subscribe (basic handshake)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_subscribe_handshake() {
    let (_tmp, client) = setup().await;
    // Just verify the subscribe handshake works without error
    let _sub = client.subscribe().await.unwrap();
}

// ---------------------------------------------------------------------------
// Multiple clients on same socket
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_multiple_concurrent_clients() {
    let (tmp, mut client1) = setup().await;

    // Connect a second client
    let mut client2 = ControlClient::connect(tmp.path()).await.unwrap();

    // Both should be able to ping
    assert!(client1.ping().await);
    assert!(client2.ping().await);

    // Client 1 registers, client 2 sees it
    client1
        .register(
            "shared-app",
            4000,
            999,
            &["cmd".into()],
            std::path::Path::new("/tmp"),
        )
        .await
        .unwrap();

    let resp = client2.request(&ControlRequest::List).await.unwrap();
    match resp {
        ControlResponse::Apps { apps } => {
            assert_eq!(apps.len(), 1);
            assert_eq!(apps[0].name, "shared-app");
        }
        other => panic!("expected Apps, got {:?}", other),
    }
}
