//! Pingora Server lifecycle management.
//!
//! Starts the PortZero proxy as a Pingora `Server`, with the `PortZeroProxy`
//! `ProxyHttp` implementation handling all traffic.
//! Also starts the control socket for CLI ↔ daemon communication,
//! and the axum API server that powers the embedded web dashboard.

use anyhow::Result;
use pingora_core::server::Server;
use pingora_proxy::http_proxy_service;
use portzero_core::certs;
use portzero_core::control;
use portzero_core::log_store::LogStore;
use portzero_core::mock_engine::MockEngine;
use portzero_core::network_sim::NetworkSim;
use portzero_core::proxy::PortZeroProxy;
use portzero_core::recorder::Recorder;
use portzero_core::router::Router;
use portzero_core::store::Store;
use portzero_core::types::{DEFAULT_PROXY_PORT, RESERVED_SUBDOMAIN};
use portzero_core::ws::WsHub;
use std::path::Path;
use std::sync::Arc;

/// Shared application state that's passed to commands.
pub struct AppState {
    pub router: Arc<Router>,
    pub store: Arc<Store>,
    pub recorder: Arc<Recorder>,
    pub ws_hub: Arc<WsHub>,
    pub network_sim: Arc<NetworkSim>,
    pub mock_engine: Arc<MockEngine>,
    pub proxy_port: u16,
}

impl AppState {
    /// Initialize all shared state.
    pub fn new(state_dir: &Path, proxy_port: u16) -> Result<Self> {
        let db_path = state_dir.join("portzero.db");
        let store = Arc::new(Store::open(&db_path)?);
        let ws_hub = Arc::new(WsHub::new());
        let router = Arc::new(Router::new());
        let recorder = Arc::new(Recorder::new(store.clone(), ws_hub.clone()));
        let network_sim = Arc::new(NetworkSim::new());
        let mock_engine = Arc::new(MockEngine::new(Some((*ws_hub).clone())));

        // Load persisted mocks from the database
        if let Ok(mocks) = store.list_mocks(None) {
            for mock in mocks {
                mock_engine.add_mock_raw(mock);
            }
            tracing::info!(
                "Loaded {} mocks from database",
                mock_engine.list_mocks().len()
            );
        }

        Ok(Self {
            router,
            store,
            recorder,
            ws_hub,
            network_sim,
            mock_engine,
            proxy_port,
        })
    }
}

/// Guard that removes PID file and control socket on drop.
/// Ensures cleanup happens regardless of how the daemon exits (signal, crash, normal).
struct CleanupGuard {
    pid_file: std::path::PathBuf,
    socket_path: std::path::PathBuf,
}

impl Drop for CleanupGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.pid_file);
        let _ = std::fs::remove_file(&self.socket_path);
    }
}

/// Check whether a process with the given PID is still alive.
#[cfg(unix)]
pub fn is_process_alive(pid: i32) -> bool {
    // kill with signal 0 checks existence without sending a signal
    unsafe { libc::kill(pid, 0) == 0 }
}

/// Start the proxy daemon.
///
/// Runs four things concurrently:
/// 1. Pingora proxy on port 1337 (on a dedicated thread — it creates its own runtime)
/// 2. Control socket on ~/.portzero/portzero.sock (for CLI registration)
/// 3. Axum API server on a random local port (dashboard + REST API + WebSocket)
/// 4. PID file management
pub async fn start(daemonize: bool, state_dir: &Path) -> Result<()> {
    if daemonize {
        return start_background(state_dir).await;
    }

    // Ensure certs exist
    let generated = certs::ensure_certs(state_dir)?;
    if generated {
        println!(
            "Generated TLS certificates in {}",
            state_dir.join("certs").display()
        );
        println!(
            "To trust the CA certificate, run: portzero trust\n\
             Or manually: {}",
            certs::trust_ca_command(state_dir)
        );
    }

    let proxy_port = DEFAULT_PROXY_PORT;

    // Initialize shared state
    let app_state = AppState::new(state_dir, proxy_port)?;

    // Write PID file
    let pid_file = state_dir.join("portzero.pid");
    std::fs::write(&pid_file, std::process::id().to_string())?;

    // Register cleanup guard — removes PID file and socket when dropped,
    // regardless of how the function exits (normal return, panic, signal).
    let _cleanup = CleanupGuard {
        pid_file: pid_file.clone(),
        socket_path: control::socket_path(state_dir),
    };

    // ── Start the axum API server on a random port ─────────────────────
    let api_state = portzero_api::AppState::new_with_state_dir(
        app_state.store.clone(),
        app_state.ws_hub.clone(),
        state_dir.to_path_buf(),
    );
    let api_router = portzero_api::build_router(api_state);
    let api_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let api_port = api_listener.local_addr()?.port();

    tracing::info!(port = api_port, "API server listening");

    tokio::spawn(async move {
        if let Err(e) = axum::serve(api_listener, api_router).await {
            tracing::error!("API server error: {e}");
        }
    });

    // Register the dashboard/API as a route so Pingora proxies to it.
    // This makes `_portzero.localhost:1337` → `127.0.0.1:<api_port>`.
    app_state.router.register(
        RESERVED_SUBDOMAIN.to_string(),
        api_port,
        std::process::id(),
        vec!["portzero-api".to_string()],
        state_dir.to_path_buf(),
    );

    // Create the Pingora proxy
    let proxy = PortZeroProxy::new(
        app_state.router.clone(),
        app_state.recorder.clone(),
        app_state.ws_hub.clone(),
        app_state.network_sim.clone(),
        app_state.mock_engine.clone(),
    );

    println!("PortZero proxy starting on http://localhost:{}", proxy_port);
    println!(
        "Dashboard: http://{}.localhost:{}",
        RESERVED_SUBDOMAIN, proxy_port
    );
    println!(
        "Control socket: {}",
        control::socket_path(state_dir).display()
    );

    // Start the control socket listener (runs on tokio)
    let ctrl_router = app_state.router.clone();
    let ctrl_ws_hub = app_state.ws_hub.clone();
    let ctrl_network_sim = app_state.network_sim.clone();
    let ctrl_mock_engine = app_state.mock_engine.clone();
    let ctrl_store = app_state.store.clone();
    let ctrl_state_dir = state_dir.to_path_buf();
    let db_path = state_dir.join("portzero.db");
    let log_store = Arc::new(LogStore::open(&db_path).unwrap_or_else(|e| {
        tracing::warn!("Failed to open persistent log store: {e}, falling back to in-memory");
        LogStore::new()
    }));

    // Create tunnel manager — resolves token from env var / config / credentials
    #[cfg(feature = "tunnel")]
    let tunnel_manager = Arc::new(portzero_core::tunnel::TunnelManager::from_state_dir(
        state_dir,
        None, // TODO: load [tunnel] config from portzero.toml if present
        Some((*app_state.ws_hub).clone()),
    ));
    #[cfg(not(feature = "tunnel"))]
    let tunnel_manager = Arc::new(portzero_core::tunnel::TunnelManager::stub(Some(
        (*app_state.ws_hub).clone(),
    )));

    let ctrl_tunnel_manager = tunnel_manager.clone();
    tokio::spawn(async move {
        if let Err(e) = control::serve_control_socket(
            &ctrl_state_dir,
            ctrl_router,
            ctrl_ws_hub,
            log_store,
            ctrl_network_sim,
            ctrl_mock_engine,
            ctrl_store,
            ctrl_tunnel_manager,
            proxy_port,
        )
        .await
        {
            tracing::error!("Control socket error: {e}");
        }
    });

    // Build the Pingora server.
    // Pingora's run_forever() calls block_on() internally to create its own
    // tokio runtime. We must run it on a dedicated thread.
    let mut server = Server::new(None)?;
    server.bootstrap();

    let mut proxy_service = http_proxy_service(&server.configuration, proxy);
    proxy_service.add_tcp(&format!("0.0.0.0:{}", proxy_port));

    server.add_service(proxy_service);

    // Use `server.run()` instead of `server.run_forever()` because
    // `run_forever()` ends with `std::process::exit(0)` which kills the
    // process without running destructors — our CleanupGuard would never
    // execute and the PID file / socket would be left behind.
    let handle = std::thread::spawn(move || {
        server.run(pingora_core::server::RunArgs::default());
    });

    handle
        .join()
        .map_err(|_| anyhow::anyhow!("Pingora server thread panicked"))?;

    // _cleanup guard handles PID file and socket removal on drop

    Ok(())
}

/// Start the daemon as a detached background process.
///
/// Re-execs `portzero start` (without `--daemon`) as a detached child,
/// waits for the control socket to become responsive, then returns.
async fn start_background(state_dir: &Path) -> Result<()> {
    // Check if already running
    if let Some(mut client) = control::ControlClient::connect(state_dir).await {
        if client.ping().await {
            println!("Daemon is already running.");
            return Ok(());
        }
    }

    println!("Starting PortZero daemon in the background...");

    let exe = std::env::current_exe()?;
    let mut cmd = std::process::Command::new(&exe);
    cmd.arg("start"); // without --daemon, so it runs in foreground in the child

    // Detach: redirect stdio and create new session
    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        unsafe {
            cmd.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }
    }

    cmd.spawn()?;

    // Wait for the daemon to start (poll the control socket)
    for i in 0..50 {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        if let Some(mut client) = control::ControlClient::connect(state_dir).await {
            if client.ping().await {
                println!("Daemon started (PID file: {})", state_dir.join("portzero.pid").display());
                return Ok(());
            }
        }
        if i == 10 {
            println!("Waiting for daemon to start...");
        }
    }

    anyhow::bail!(
        "Failed to start daemon after 5 seconds.\n\
         Check logs or try: portzero start (foreground)"
    )
}

/// Stop the proxy daemon (sends SIGQUIT to the running process).
pub async fn stop(state_dir: &Path) -> Result<()> {
    let pid_file = state_dir.join("portzero.pid");
    if !pid_file.exists() {
        println!("No running daemon found.");
        return Ok(());
    }

    let pid_str = std::fs::read_to_string(&pid_file)?;
    let pid: i32 = pid_str.trim().parse()?;

    #[cfg(unix)]
    {
        if !is_process_alive(pid) {
            // Process is already dead — just clean up stale files
            println!("Daemon (PID {}) is no longer running. Cleaning up stale files...", pid);
            let _ = std::fs::remove_file(&pid_file);
            let _ = std::fs::remove_file(control::socket_path(state_dir));
            return Ok(());
        }

        // Pingora signal semantics:
        //   SIGTERM = graceful terminate
        //   SIGINT  = fast shutdown
        //   SIGQUIT = graceful upgrade (NOT shutdown — waits for new process)
        let output = std::process::Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .output()?;
        if output.status.success() {
            println!("Sent SIGTERM to daemon (PID {})", pid);
            // Give the process a moment to exit and let its CleanupGuard run
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            // Remove stale files in case the guard didn't run
            let _ = std::fs::remove_file(&pid_file);
            let _ = std::fs::remove_file(control::socket_path(state_dir));
        } else {
            println!("Failed to send signal to PID {}", pid);
        }
    }

    #[cfg(not(unix))]
    {
        println!(
            "Stopping daemon not supported on this platform. Kill PID {} manually.",
            pid
        );
    }

    Ok(())
}

/// Show daemon status.
pub async fn status(state_dir: &Path) -> Result<()> {
    // Try connecting to the control socket first
    if let Some(mut client) = control::ControlClient::connect(state_dir).await {
        if client.ping().await {
            println!("Daemon is running.");

            // List apps
            match client.request(&control::ControlRequest::List).await {
                Ok(control::ControlResponse::Apps { apps }) => {
                    if apps.is_empty() {
                        println!("No apps registered.");
                    } else {
                        println!("\nRegistered apps:");
                        for app in &apps {
                            println!(
                                "  {} → localhost:{} (PID {}) {}",
                                app.name, app.port, app.pid, app.url
                            );
                        }
                    }
                }
                _ => {}
            }
            return Ok(());
        }
    }

    // Fallback: check PID file
    let pid_file = state_dir.join("portzero.pid");
    if pid_file.exists() {
        let pid_str = std::fs::read_to_string(&pid_file)?;
        let pid: i32 = pid_str.trim().parse().unwrap_or(0);

        #[cfg(unix)]
        let alive = pid > 0 && is_process_alive(pid);
        #[cfg(not(unix))]
        let alive = false;

        if alive {
            println!(
                "Daemon process is running (PID {}), but not responding on control socket.",
                pid
            );
            println!("It may be starting up or stuck. Try: portzero stop");
        } else {
            // Process is dead — clean up stale files automatically
            println!("Daemon is not running (stale PID file for PID {}).", pid);
            println!("Cleaning up stale state files...");
            let _ = std::fs::remove_file(&pid_file);
            let _ = std::fs::remove_file(control::socket_path(state_dir));
            println!("Done. Start the daemon with: portzero start");
        }
    } else {
        println!("Daemon not running.");
    }
    Ok(())
}
