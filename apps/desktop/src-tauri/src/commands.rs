use crate::state::DesktopState;
use portzero_core::*;
use serde::Serialize;
use std::collections::HashMap;
use tauri::State;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Query CPU (%) and memory (bytes) for a process and all its descendants.
///
/// Uses `ps` on macOS because `sysinfo` cannot read per-process CPU usage
/// without elevated privileges on recent macOS versions.
fn get_process_stats(pid: u32) -> (Option<f64>, Option<u64>) {
    // `ps -e -o pid=,ppid=,%cpu=,rss=` gives us every process in a parseable
    // format. We then walk the tree from `pid` to collect descendants.
    let output = match std::process::Command::new("ps")
        .args(["-e", "-o", "pid=,ppid=,%cpu=,rss="])
        .output()
    {
        Ok(o) if o.status.success() => o,
        _ => return (None, None),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse into (pid, ppid, cpu, rss_kb) tuples.
    struct Row {
        pid: u32,
        ppid: u32,
        cpu: f64,
        rss_kb: u64,
    }
    let rows: Vec<Row> = stdout
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let pid: u32 = parts.next()?.parse().ok()?;
            let ppid: u32 = parts.next()?.parse().ok()?;
            let cpu: f64 = parts.next()?.parse().ok()?;
            let rss_kb: u64 = parts.next()?.parse().ok()?;
            Some(Row {
                pid,
                ppid,
                cpu,
                rss_kb,
            })
        })
        .collect();

    // BFS to collect the root PID + all descendants.
    let mut tree: Vec<u32> = vec![pid];
    let mut i = 0;
    while i < tree.len() {
        let parent = tree[i];
        for row in &rows {
            if row.ppid == parent && !tree.contains(&row.pid) {
                tree.push(row.pid);
            }
        }
        i += 1;
    }

    // Sum CPU and memory for the tree.
    let mut total_cpu: f64 = 0.0;
    let mut total_mem: u64 = 0;
    let mut found = false;
    for row in &rows {
        if tree.contains(&row.pid) {
            total_cpu += row.cpu;
            total_mem += row.rss_kb * 1024; // rss is in KB, convert to bytes
            found = true;
        }
    }

    if found {
        (Some(total_cpu), Some(total_mem))
    } else {
        (None, None)
    }
}

fn route_to_app_info(route: &Route, pm: &ProcessManager) -> AppInfo {
    let proc = pm.get(&route.hostname);
    let (restarts, auto_restart) = match proc {
        Some(p) => {
            // Use try_lock since we're in a sync context; fall back to defaults if locked
            match p.try_lock() {
                Ok(p) => (p.restarts, p.auto_restart),
                Err(_) => (0, false),
            }
        }
        None => (0, false),
    };
    let (cpu_percent, memory_bytes) = get_process_stats(route.pid);
    AppInfo {
        name: route.hostname.clone(),
        port: route.port,
        pid: Some(route.pid),
        command: route.command.clone(),
        cwd: route.cwd.clone(),
        status: route.status.clone(),
        started_at: Some(route.started_at),
        restarts,
        auto_restart,
        url: format!("http://{}.localhost:{}", route.hostname, DEFAULT_PROXY_PORT),
        cpu_percent,
        memory_bytes,
        tunnel_url: None,
    }
}

#[allow(dead_code)]
fn record_to_summary(r: &RequestRecord) -> RequestSummary {
    RequestSummary {
        id: r.id.clone(),
        app_name: r.app_name.clone(),
        timestamp: r.timestamp,
        method: r.method.clone(),
        path: r.path.clone(),
        status_code: r.status_code,
        duration_ms: r.duration_ms,
        mocked: r.mocked,
    }
}

fn build_store_filter(f: &RequestFilters) -> portzero_core::store::RequestFilter {
    portzero_core::store::RequestFilter {
        app_name: f.app.clone(),
        method: f.method.clone(),
        status_code: f.status,
        path_prefix: f.path.clone(),
        search: f.search.clone(),
        limit: f.limit.map(|v| v as usize),
        offset: f.offset.map(|v| v as usize),
    }
}

// ---------------------------------------------------------------------------
// Frontend filter params
// ---------------------------------------------------------------------------

#[derive(Debug, Default, serde::Deserialize)]
pub struct RequestFilters {
    pub app: Option<String>,
    pub method: Option<String>,
    pub status: Option<u16>,
    pub status_range: Option<String>,
    pub path: Option<String>,
    pub search: Option<String>,
    pub from: Option<u64>,
    pub to: Option<u64>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

// ---------------------------------------------------------------------------
// Apps
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn list_apps(state: State<'_, DesktopState>) -> Result<Vec<AppInfo>, String> {
    // First check the local router (for apps managed by the desktop app)
    // Filter out the internal _portzero dashboard route.
    let mut apps: Vec<AppInfo> = state
        .router
        .list()
        .iter()
        .filter(|r| r.hostname != portzero_core::types::RESERVED_SUBDOMAIN)
        .map(|r| route_to_app_info(r, &state.process_manager))
        .collect();

    // Also query the daemon's control socket (for apps managed by CLI)
    if let Some(mut client) = portzero_core::control::ControlClient::connect(&state.state_dir).await
    {
        if let Ok(portzero_core::control::ControlResponse::Apps { apps: daemon_apps }) = client
            .request(&portzero_core::control::ControlRequest::List)
            .await
        {
            for entry in daemon_apps {
                // Avoid duplicates (if the same app is somehow in both)
                if !apps.iter().any(|a| a.name == entry.name) {
                    let (cpu_percent, memory_bytes) = get_process_stats(entry.pid);
                    apps.push(AppInfo {
                        name: entry.name.clone(),
                        port: entry.port,
                        pid: Some(entry.pid),
                        command: entry.command.clone(),
                        cwd: entry.cwd.clone(),
                        status: portzero_core::types::AppStatus::Running,
                        started_at: Some(entry.started_at),
                        restarts: 0,
                        auto_restart: false,
                        url: entry.url,
                        cpu_percent,
                        memory_bytes,
                        tunnel_url: None,
                    });
                }
            }
        }
    }

    Ok(apps)
}

#[tauri::command]
pub async fn get_app(state: State<'_, DesktopState>, name: String) -> Result<AppInfo, String> {
    // Check local router first
    if let Some(route) = state.router.get(&name) {
        return Ok(route_to_app_info(&route, &state.process_manager));
    }

    // Try daemon
    if let Some(mut client) = portzero_core::control::ControlClient::connect(&state.state_dir).await
    {
        if let Ok(portzero_core::control::ControlResponse::Apps { apps }) = client
            .request(&portzero_core::control::ControlRequest::List)
            .await
        {
            if let Some(entry) = apps.iter().find(|a| a.name == name) {
                let (cpu_percent, memory_bytes) = get_process_stats(entry.pid);
                return Ok(AppInfo {
                    name: entry.name.clone(),
                    port: entry.port,
                    pid: Some(entry.pid),
                    command: entry.command.clone(),
                    cwd: entry.cwd.clone(),
                    status: portzero_core::types::AppStatus::Running,
                    started_at: Some(entry.started_at),
                    restarts: 0,
                    auto_restart: false,
                    url: entry.url.clone(),
                    cpu_percent,
                    memory_bytes,
                    tunnel_url: None,
                });
            }
        }
    }

    Err(format!("App '{}' not found", name))
}

#[tauri::command]
pub async fn restart_app(state: State<'_, DesktopState>, name: String) -> Result<(), String> {
    state
        .process_manager
        .restart(&name)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn stop_app(state: State<'_, DesktopState>, name: String) -> Result<(), String> {
    state
        .process_manager
        .stop(&name)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_app_logs(
    state: State<'_, DesktopState>,
    name: String,
    lines: Option<usize>,
) -> Result<Vec<LogLine>, String> {
    // Try local process manager first
    match state.process_manager.get_logs(&name, lines).await {
        Ok(logs) if !logs.is_empty() => return Ok(logs),
        _ => {}
    }

    // Fall back to daemon's log store (for CLI-started apps)
    if let Some(mut client) = portzero_core::control::ControlClient::connect(&state.state_dir).await
    {
        match client.get_logs(&name, lines).await {
            Ok(logs) => return Ok(logs),
            Err(e) => {
                tracing::debug!("Failed to get logs from daemon: {e}");
            }
        }
    }

    Ok(vec![])
}

#[tauri::command]
pub fn get_app_schema(
    state: State<'_, DesktopState>,
    name: String,
) -> Result<InferredSchema, String> {
    let schema = state.schema_inference.get_schema(&name);
    match schema {
        Some(s) => Ok(s),
        None => Ok(InferredSchema {
            app_name: name,
            endpoints: vec![],
            last_updated: chrono::Utc::now(),
        }),
    }
}

// ---------------------------------------------------------------------------
// Requests / Traffic
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn list_requests(
    state: State<'_, DesktopState>,
    filters: Option<RequestFilters>,
) -> Result<Vec<RequestSummary>, String> {
    let f = filters.unwrap_or_default();
    let store_filter = build_store_filter(&f);
    state
        .store
        .list_request_summaries(&store_filter)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_request(state: State<'_, DesktopState>, id: String) -> Result<RequestRecord, String> {
    state
        .store
        .get_request(&id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Request '{}' not found", id))
}

#[tauri::command]
pub async fn replay_request(
    state: State<'_, DesktopState>,
    id: String,
    options: Option<ReplayRequest>,
) -> Result<RequestRecord, String> {
    let original = state
        .store
        .get_request(&id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Request '{}' not found", id))?;

    let method_str = options
        .as_ref()
        .and_then(|o| o.method.as_deref())
        .unwrap_or(&original.method);

    // Build the replay URL. The stored `url` may be a full URL (http://host/path)
    // or just a path (/path) from older recordings. If it's just a path,
    // reconstruct the full URL using the app name and proxy port.
    let raw_url = options
        .as_ref()
        .and_then(|o| o.url.as_deref())
        .unwrap_or(&original.url);
    let url_str = if raw_url.starts_with("http://") || raw_url.starts_with("https://") {
        raw_url.to_string()
    } else {
        // Fallback: route through the proxy using <app>.localhost:<port>
        format!(
            "http://{}.localhost:{}{}",
            original.app_name, state.proxy_port, raw_url
        )
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let method: reqwest::Method = method_str
        .parse()
        .map_err(|e: http::method::InvalidMethod| e.to_string())?;
    let mut req = client.request(method, &url_str);

    let mut headers = original.request_headers.clone();
    if let Some(ref opts) = options {
        if let Some(ref h) = opts.headers {
            for (k, v) in h {
                headers.insert(k.clone(), v.clone());
            }
        }
    }
    for (k, v) in &headers {
        if let (Ok(name), Ok(val)) = (
            reqwest::header::HeaderName::from_bytes(k.as_bytes()),
            reqwest::header::HeaderValue::from_str(v),
        ) {
            req = req.header(name, val);
        }
    }

    if let Some(ref opts) = options {
        if let Some(ref body) = opts.body {
            req = req.body(body.clone());
        }
    } else if let Some(ref body) = original.request_body {
        req = req.body(body.clone());
    }

    let start = std::time::Instant::now();
    let resp = req.send().await.map_err(|e| e.to_string())?;
    let duration = start.elapsed();

    let status_code = resp.status().as_u16();
    let status_message = resp.status().canonical_reason().unwrap_or("").to_string();
    let resp_headers: HashMap<String, String> = resp
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();
    let content_type = resp_headers.get("content-type").cloned();
    let body_bytes = resp.bytes().await.map_err(|e| e.to_string())?;

    let record = RequestRecord {
        id: uuid::Uuid::new_v4().to_string(),
        app_name: original.app_name.clone(),
        timestamp: chrono::Utc::now(),
        duration_ms: duration.as_millis() as u64,
        method: method_str.to_string(),
        url: url_str.to_string(),
        path: original.path.clone(),
        query_string: original.query_string.clone(),
        request_headers: headers,
        request_body: original.request_body.clone(),
        request_content_type: original.request_content_type.clone(),
        status_code,
        status_message,
        response_headers: resp_headers,
        response_body: Some(body_bytes.to_vec()),
        response_content_type: content_type,
        mocked: false,
        parent_id: Some(original.id.clone()),
    };

    state
        .store
        .insert_request(&record)
        .map_err(|e| e.to_string())?;
    Ok(record)
}

#[tauri::command]
pub fn clear_requests(state: State<'_, DesktopState>, app: Option<String>) -> Result<(), String> {
    state
        .store
        .clear_requests(app.as_deref())
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn diff_requests(
    state: State<'_, DesktopState>,
    id1: String,
    id2: String,
) -> Result<RequestDiff, String> {
    let left = state
        .store
        .get_request(&id1)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Request '{}' not found", id1))?;
    let right = state
        .store
        .get_request(&id2)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Request '{}' not found", id2))?;
    Ok(RequestDiff { left, right })
}

// ---------------------------------------------------------------------------
// Mocks (forwarded to daemon where the proxy's MockEngine lives)
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn list_mocks(state: State<'_, DesktopState>) -> Result<Vec<MockRule>, String> {
    if let Some(mut client) = portzero_core::control::ControlClient::connect(&state.state_dir).await
    {
        if let Ok(mocks) = client.list_mocks().await {
            return Ok(mocks);
        }
    }
    Ok(state.mock_engine.list_mocks())
}

#[tauri::command]
pub async fn create_mock(
    state: State<'_, DesktopState>,
    rule: CreateMockRule,
) -> Result<MockRule, String> {
    if let Some(mut client) = portzero_core::control::ControlClient::connect(&state.state_dir).await
    {
        if let Ok(mock) = client.create_mock(rule.clone()).await {
            return Ok(mock);
        }
    }
    // Fallback to local
    let mock = state.mock_engine.add_mock(
        rule.app_name,
        rule.method,
        rule.path_pattern,
        rule.status_code,
        rule.response_headers,
        rule.response_body,
    );
    let _ = state.store.insert_mock(&mock);
    Ok(mock)
}

#[tauri::command]
pub async fn update_mock(
    state: State<'_, DesktopState>,
    id: String,
    updates: UpdateMockRule,
) -> Result<MockRule, String> {
    if let Some(mut client) = portzero_core::control::ControlClient::connect(&state.state_dir).await
    {
        if let Ok(mock) = client.update_mock(&id, updates.clone()).await {
            return Ok(mock);
        }
    }
    state
        .mock_engine
        .update_mock(
            &id,
            updates.method,
            updates.path_pattern,
            updates.status_code,
            updates.response_headers,
            updates.response_body,
            updates.enabled,
        )
        .ok_or_else(|| format!("Mock '{}' not found", id))
}

#[tauri::command]
pub async fn delete_mock(state: State<'_, DesktopState>, id: String) -> Result<(), String> {
    if let Some(mut client) = portzero_core::control::ControlClient::connect(&state.state_dir).await
    {
        if let Ok(()) = client.delete_mock(&id).await {
            return Ok(());
        }
    }
    if state.mock_engine.remove_mock(&id) {
        let _ = state.store.delete_mock(&id);
        Ok(())
    } else {
        Err(format!("Mock '{}' not found", id))
    }
}

#[tauri::command]
pub async fn toggle_mock(state: State<'_, DesktopState>, id: String) -> Result<MockRule, String> {
    if let Some(mut client) = portzero_core::control::ControlClient::connect(&state.state_dir).await
    {
        if let Ok(mock) = client.toggle_mock(&id).await {
            return Ok(mock);
        }
    }
    state.mock_engine.toggle_mock(&id);
    state
        .mock_engine
        .get_mock(&id)
        .ok_or_else(|| format!("Mock '{}' not found", id))
}

// ---------------------------------------------------------------------------
// Network Simulation
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn get_network_profile(
    state: State<'_, DesktopState>,
    app: String,
) -> Result<NetworkProfile, String> {
    // Try forwarding to the daemon (where the proxy's NetworkSim lives)
    if let Some(mut client) = portzero_core::control::ControlClient::connect(&state.state_dir).await
    {
        match client.get_network_profile(&app).await {
            Ok(profile) => return Ok(profile),
            Err(e) => tracing::debug!("Failed to get network profile from daemon: {e}"),
        }
    }

    // Fallback to local (for when daemon isn't running)
    Ok(state
        .network_sim
        .get_profile(&app)
        .unwrap_or_else(|| NetworkProfile {
            app_name: app,
            latency_ms: None,
            jitter_ms: None,
            packet_loss_rate: 0.0,
            bandwidth_limit: None,
            path_filter: None,
        }))
}

#[tauri::command]
pub async fn update_network_profile(
    state: State<'_, DesktopState>,
    app: String,
    profile: SetNetworkProfile,
) -> Result<NetworkProfile, String> {
    // Forward to the daemon so the proxy applies the simulation
    if let Some(mut client) = portzero_core::control::ControlClient::connect(&state.state_dir).await
    {
        match client.set_network_profile(&app, profile.clone()).await {
            Ok(np) => return Ok(np),
            Err(e) => tracing::debug!("Failed to set network profile on daemon: {e}"),
        }
    }

    // Fallback to local
    let np = NetworkProfile {
        app_name: app,
        latency_ms: profile.latency_ms,
        jitter_ms: profile.jitter_ms,
        packet_loss_rate: profile.packet_loss_rate.unwrap_or(0.0),
        bandwidth_limit: profile.bandwidth_limit,
        path_filter: profile.path_filter,
    };
    state.network_sim.set_profile(np.clone());
    Ok(np)
}

#[tauri::command]
pub async fn clear_network_profile(
    state: State<'_, DesktopState>,
    app: String,
) -> Result<(), String> {
    // Forward to the daemon
    if let Some(mut client) = portzero_core::control::ControlClient::connect(&state.state_dir).await
    {
        match client.clear_network_profile(&app).await {
            Ok(()) => return Ok(()),
            Err(e) => tracing::debug!("Failed to clear network profile on daemon: {e}"),
        }
    }

    // Fallback to local
    state.network_sim.clear_profile(&app);
    Ok(())
}

// ---------------------------------------------------------------------------
// Tunnels (stubs — tunnels require the Pingora proxy running)
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn start_tunnel(
    _state: State<'_, DesktopState>,
    _app: String,
    _subdomain: Option<String>,
) -> Result<TunnelInfo, String> {
    Err("Tunnels are not available in this build. This feature will be enabled in a future release.".to_string())
}

#[tauri::command]
pub async fn stop_tunnel(_state: State<'_, DesktopState>, _app: String) -> Result<(), String> {
    Err("Tunnels are not available in this build. This feature will be enabled in a future release.".to_string())
}

// ---------------------------------------------------------------------------
// System / Status
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn get_status(state: State<'_, DesktopState>) -> Result<DaemonStatus, String> {
    let uptime = state.started_at.elapsed().as_secs();
    let mut total_apps = state.router.list().len();
    let total_requests = state.store.request_count().unwrap_or(0);

    // Also count daemon apps (CLI-registered) that aren't in the local router
    if let Some(mut client) = portzero_core::control::ControlClient::connect(&state.state_dir).await
    {
        if let Ok(portzero_core::control::ControlResponse::Apps { apps: daemon_apps }) = client
            .request(&portzero_core::control::ControlRequest::List)
            .await
        {
            let local_names: Vec<String> = state
                .router
                .list()
                .iter()
                .map(|r| r.hostname.clone())
                .collect();
            let extra = daemon_apps
                .iter()
                .filter(|a| !local_names.contains(&a.name))
                .count();
            total_apps += extra;
        }
    }

    Ok(DaemonStatus {
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_secs: uptime,
        proxy_port: state.proxy_port,
        total_apps,
        total_requests,
    })
}

// ---------------------------------------------------------------------------
// Daemon Management
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn get_daemon_info(
    state: State<'_, DesktopState>,
) -> Result<crate::daemon_bridge::DaemonRunInfo, String> {
    Ok(crate::daemon_bridge::get_daemon_info(&state.state_dir).await)
}

#[tauri::command]
pub async fn start_daemon(state: State<'_, DesktopState>) -> Result<(), String> {
    crate::daemon_bridge::start_daemon(&state.state_dir)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn stop_daemon(state: State<'_, DesktopState>) -> Result<(), String> {
    crate::daemon_bridge::stop_daemon(&state.state_dir)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn restart_daemon(state: State<'_, DesktopState>) -> Result<(), String> {
    crate::daemon_bridge::restart_daemon(&state.state_dir)
        .await
        .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Certificates
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct CertStatus {
    pub certs_exist: bool,
    pub ca_trusted: bool,
    pub ca_cert_path: String,
    pub trust_command: String,
}

#[derive(Debug, Serialize)]
pub struct TrustResponse {
    pub status: String,
    pub message: String,
}

#[tauri::command]
pub fn get_cert_status(state: State<'_, DesktopState>) -> Result<CertStatus, String> {
    let ca_cert_path = state.state_dir.join("certs").join("ca.crt");
    let certs_exist = ca_cert_path.exists();
    let ca_trusted = if certs_exist {
        portzero_core::certs::is_ca_trusted(&state.state_dir).unwrap_or(false)
    } else {
        false
    };
    let trust_command = if certs_exist {
        portzero_core::certs::trust_ca_command(&state.state_dir)
    } else {
        "Certificates not generated yet. Start the proxy first.".to_string()
    };

    Ok(CertStatus {
        certs_exist,
        ca_trusted,
        ca_cert_path: ca_cert_path.display().to_string(),
        trust_command,
    })
}

#[tauri::command]
pub async fn trust_ca(state: State<'_, DesktopState>) -> Result<TrustResponse, String> {
    match portzero_core::certs::trust_ca(&state.state_dir, true).map_err(|e| e.to_string())? {
        portzero_core::certs::TrustResult::Trusted => Ok(TrustResponse {
            status: "trusted".to_string(),
            message: "CA certificate trusted successfully.".to_string(),
        }),
        portzero_core::certs::TrustResult::AlreadyTrusted => Ok(TrustResponse {
            status: "already_trusted".to_string(),
            message: "CA certificate is already trusted.".to_string(),
        }),
        portzero_core::certs::TrustResult::NeedsSudo => {
            Err("Elevated privileges required. Use the manual command.".to_string())
        }
        portzero_core::certs::TrustResult::Failed(msg) => Err(msg),
        portzero_core::certs::TrustResult::Unsupported => {
            Err("Trust not supported on this platform.".to_string())
        }
    }
}

#[tauri::command]
pub async fn untrust_ca(state: State<'_, DesktopState>) -> Result<TrustResponse, String> {
    match portzero_core::certs::untrust_ca(&state.state_dir, true).map_err(|e| e.to_string())? {
        portzero_core::certs::TrustResult::Trusted => Ok(TrustResponse {
            status: "untrusted".to_string(),
            message: "CA certificate removed from trust store.".to_string(),
        }),
        portzero_core::certs::TrustResult::AlreadyTrusted => Ok(TrustResponse {
            status: "not_trusted".to_string(),
            message: "CA certificate was not in the trust store.".to_string(),
        }),
        portzero_core::certs::TrustResult::NeedsSudo => {
            Err("Elevated privileges required. Use the manual command.".to_string())
        }
        portzero_core::certs::TrustResult::Failed(msg) => Err(msg),
        portzero_core::certs::TrustResult::Unsupported => {
            Err("Untrust not supported on this platform.".to_string())
        }
    }
}

// ---------------------------------------------------------------------------
// CLI Installation
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct CliStatus {
    /// Whether `portzero` is found somewhere in PATH
    pub installed: bool,
    /// The path where the current `portzero` binary lives (if found)
    pub current_path: Option<String>,
    /// The path to the CLI binary built from this workspace
    pub binary_path: Option<String>,
    /// Whether the binary has been built yet
    pub binary_exists: bool,
    /// The default install location
    pub install_dir: String,
}

#[derive(Debug, Serialize)]
pub struct CliInstallResult {
    pub success: bool,
    pub message: String,
    pub installed_path: Option<String>,
}

/// Find the workspace target directory by walking up from the executable
/// or from a compile-time known path.
fn find_cli_binary() -> Option<std::path::PathBuf> {
    // During development, CARGO_MANIFEST_DIR points to src-tauri/
    // The workspace root is 3 levels up: src-tauri -> desktop -> apps -> root
    // The binary is at <workspace-root>/target/{debug,release}/portzero
    let manifest_dir = option_env!("CARGO_MANIFEST_DIR");
    if let Some(dir) = manifest_dir {
        let workspace_root = std::path::Path::new(dir)
            .parent() // apps/desktop
            .and_then(|p| p.parent()) // apps
            .and_then(|p| p.parent()); // workspace root
        if let Some(root) = workspace_root {
            // Check release first, then debug
            let release = root.join("target/release/portzero");
            if release.exists() {
                return Some(release);
            }
            let debug = root.join("target/debug/portzero");
            if debug.exists() {
                return Some(debug);
            }
        }
    }

    // Fallback: check if the app bundle contains a sidecar binary
    // (for production builds using Tauri's externalBin feature)
    None
}

/// Default directory to install the CLI symlink
fn default_install_dir() -> String {
    "/usr/local/bin".to_string()
}

#[tauri::command]
pub fn get_cli_status() -> Result<CliStatus, String> {
    use std::process::Command;

    // Check if `portzero` is already in PATH
    let which_output = Command::new("which").arg("portzero").output().ok();

    let (installed, current_path) = match which_output {
        Some(output) if output.status.success() => {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            (true, Some(path))
        }
        _ => (false, None),
    };

    let binary = find_cli_binary();
    let binary_exists = binary.is_some();
    let binary_path = binary.map(|p| p.display().to_string());

    Ok(CliStatus {
        installed,
        current_path,
        binary_path,
        binary_exists,
        install_dir: default_install_dir(),
    })
}

#[tauri::command]
pub async fn install_cli(install_dir: Option<String>) -> Result<CliInstallResult, String> {
    use std::process::Command;

    let binary = find_cli_binary().ok_or_else(|| {
        "CLI binary not found. Build it first with `cargo build -p portzero-cli`.".to_string()
    })?;

    let dir = install_dir.unwrap_or_else(default_install_dir);
    let target = std::path::PathBuf::from(&dir).join("portzero");

    // Check if the install directory exists
    if !std::path::Path::new(&dir).exists() {
        return Err(format!("Install directory '{}' does not exist.", dir));
    }

    // If a file/symlink already exists at the target, remove it first
    if target.exists() || target.is_symlink() {
        // Try removing directly first (works if user owns the dir)
        if std::fs::remove_file(&target).is_err() {
            // Need elevated privileges — use osascript on macOS
            #[cfg(target_os = "macos")]
            {
                let script = format!(
                    r#"do shell script "rm -f '{}'" with administrator privileges"#,
                    target.display()
                );
                let output = Command::new("osascript")
                    .args(["-e", &script])
                    .output()
                    .map_err(|e| e.to_string())?;
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    if stderr.contains("User canceled") || stderr.contains("-128") {
                        return Ok(CliInstallResult {
                            success: false,
                            message: "Installation cancelled by user.".to_string(),
                            installed_path: None,
                        });
                    }
                    return Err(format!("Failed to remove existing binary: {}", stderr));
                }
            }
            #[cfg(not(target_os = "macos"))]
            {
                return Err(format!(
                    "Cannot remove existing file at '{}'. Try with sudo.",
                    target.display()
                ));
            }
        }
    }

    // Try to create a symlink directly first
    #[cfg(unix)]
    {
        if std::os::unix::fs::symlink(&binary, &target).is_ok() {
            return Ok(CliInstallResult {
                success: true,
                message: format!("Symlinked portzero to {}", target.display()),
                installed_path: Some(target.display().to_string()),
            });
        }
    }

    // Direct symlink failed (permission denied) — use osascript on macOS
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            r#"do shell script "ln -sf '{}' '{}'" with administrator privileges"#,
            binary.display(),
            target.display()
        );
        let output = Command::new("osascript")
            .args(["-e", &script])
            .output()
            .map_err(|e| e.to_string())?;

        if output.status.success() {
            return Ok(CliInstallResult {
                success: true,
                message: format!("Symlinked portzero to {}", target.display()),
                installed_path: Some(target.display().to_string()),
            });
        }

        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("User canceled") || stderr.contains("-128") {
            return Ok(CliInstallResult {
                success: false,
                message: "Installation cancelled by user.".to_string(),
                installed_path: None,
            });
        }

        return Err(format!("Failed to create symlink: {}", stderr));
    }

    #[cfg(not(target_os = "macos"))]
    {
        Err(format!(
            "Cannot create symlink at '{}'. Try running: sudo ln -sf '{}' '{}'",
            target.display(),
            binary.display(),
            target.display()
        ))
    }
}

#[tauri::command]
pub async fn uninstall_cli() -> Result<CliInstallResult, String> {
    use std::process::Command;

    // Find where portzero is currently installed
    let which_output = Command::new("which")
        .arg("portzero")
        .output()
        .map_err(|e| e.to_string())?;

    if !which_output.status.success() {
        return Ok(CliInstallResult {
            success: true,
            message: "portzero is not installed in PATH.".to_string(),
            installed_path: None,
        });
    }

    let installed_path = String::from_utf8_lossy(&which_output.stdout)
        .trim()
        .to_string();
    let path = std::path::Path::new(&installed_path);

    // Try to remove directly
    if std::fs::remove_file(path).is_ok() {
        return Ok(CliInstallResult {
            success: true,
            message: format!("Removed {}", installed_path),
            installed_path: None,
        });
    }

    // Need elevated privileges
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            r#"do shell script "rm -f '{}'" with administrator privileges"#,
            installed_path
        );
        let output = Command::new("osascript")
            .args(["-e", &script])
            .output()
            .map_err(|e| e.to_string())?;

        if output.status.success() {
            return Ok(CliInstallResult {
                success: true,
                message: format!("Removed {}", installed_path),
                installed_path: None,
            });
        }

        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("User canceled") || stderr.contains("-128") {
            return Ok(CliInstallResult {
                success: false,
                message: "Uninstall cancelled by user.".to_string(),
                installed_path: Some(installed_path),
            });
        }
        return Err(format!("Failed to remove: {}", stderr));
    }

    #[cfg(not(target_os = "macos"))]
    {
        Err(format!(
            "Cannot remove '{}'. Try: sudo rm '{}'",
            installed_path, installed_path
        ))
    }
}

// ---------------------------------------------------------------------------
// Utility: open URL in browser
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn open_in_browser(app: tauri::AppHandle, url: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url(&url, None::<&str>)
        .map_err(|e| e.to_string())
}
