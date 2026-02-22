pub mod commands;
pub mod daemon_bridge;
pub mod state;
pub mod tray;

use state::DesktopState;
use std::path::PathBuf;
use tauri::{Emitter, Manager};

/// Get the PortZero state directory (~/.portzero/).
fn default_state_dir() -> PathBuf {
    dirs_next::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".portzero")
}

/// Spawn a background task that subscribes to `WsHub` and forwards every
/// event to the Tauri frontend as an `"ws-event"` event.
fn start_event_bridge(handle: tauri::AppHandle, hub: &portzero_core::WsHub) {
    let mut rx = hub.subscribe();
    tauri::async_runtime::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    let _ = handle.emit("ws-event", &event);
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("WsHub event bridge lagged, dropped {n} events");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    tracing::info!("WsHub channel closed, stopping event bridge");
                    break;
                }
            }
        }
    });
}

/// Spawn a background task that subscribes to the daemon's event stream
/// via the control socket and emits events directly to the Tauri frontend
/// via IPC — no intermediate WsHub relay needed.
///
/// Reconnects automatically if the daemon restarts or the connection drops.
fn start_daemon_event_bridge(handle: tauri::AppHandle, state_dir: PathBuf) {
    tauri::async_runtime::spawn(async move {
        loop {
            // Try to connect and subscribe
            if let Some(client) = portzero_core::control::ControlClient::connect(&state_dir).await {
                tracing::info!("Connected to daemon control socket, subscribing to events...");
                match client.subscribe().await {
                    Ok(mut subscription) => {
                        tracing::info!("Subscribed to daemon event stream");
                        // Read events and emit directly to Tauri IPC
                        while let Some(event) = subscription.next_event().await {
                            let _ = handle.emit("ws-event", &event);
                        }
                        tracing::info!("Daemon event stream disconnected, will reconnect...");
                    }
                    Err(e) => {
                        tracing::warn!("Failed to subscribe to daemon events: {e}");
                    }
                }
            }

            // Wait before reconnecting
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    });
}

/// Initialize and run the Tauri application.
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "portzero_desktop_lib=info,portzero_core=info".into()),
        )
        .init();

    let mut builder = tauri::Builder::default();

    // Single instance plugin must be registered first (before deep-link).
    // It ensures only one app instance runs and forwards deep link args
    // from new instances to the existing one.
    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(
            |_app, argv, _cwd| {
                tracing::info!(
                    "New app instance opened with args: {argv:?} — deep link event already triggered"
                );
            },
        ));
    }

    builder = builder
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init());

    builder
        .setup(|app| {
            // Initialize shared state. Recorder::new() calls tokio::spawn()
            // internally, so we need an active tokio runtime context.
            // Tauri's async runtime exists but .setup() runs synchronously
            // on the main thread. We enter the runtime to make tokio::spawn work.
            let state_dir = default_state_dir();

            // Generate TLS certificates if they don't exist yet
            if let Err(e) = portzero_core::certs::ensure_certs(&state_dir) {
                tracing::warn!("Failed to generate TLS certificates: {e}");
            }

            let desktop_state = tauri::async_runtime::handle().block_on(async {
                DesktopState::new(&state_dir)
            })
            .expect("Failed to initialize PortZero state");

            // Start the local WsHub → Tauri IPC bridge (for locally-managed apps)
            start_event_bridge(app.handle().clone(), &desktop_state.ws_hub);

            // Start the daemon → Tauri IPC bridge (for CLI-managed apps via daemon proxy)
            start_daemon_event_bridge(app.handle().clone(), state_dir.clone());

            // Register state so commands can access it via State<DesktopState>
            app.manage(desktop_state);

            // Deep link: register scheme at runtime for dev mode (Linux/Windows)
            #[cfg(any(target_os = "linux", all(debug_assertions, windows)))]
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                if let Err(e) = app.deep_link().register_all() {
                    tracing::warn!("Failed to register deep link schemes: {e}");
                }
            }

            // Check if the app was opened via a deep link
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                if let Ok(Some(urls)) = app.deep_link().get_current() {
                    tracing::info!("App opened via deep link: {:?}", urls);
                }

                // Forward deep link events to the frontend
                let handle = app.handle().clone();
                app.deep_link().on_open_url(move |event| {
                    let urls: Vec<String> =
                        event.urls().iter().map(|u| u.to_string()).collect();
                    tracing::info!("Deep link received: {:?}", urls);
                    let _ = handle.emit("deep-link", &urls);
                });
            }

            // Initialize the system tray
            tray::create_tray(app.handle())?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Apps
            commands::list_apps,
            commands::get_app,
            commands::restart_app,
            commands::stop_app,
            commands::get_app_logs,
            commands::get_app_schema,
            // Requests
            commands::list_requests,
            commands::get_request,
            commands::replay_request,
            commands::clear_requests,
            commands::diff_requests,
            // Mocks
            commands::list_mocks,
            commands::create_mock,
            commands::update_mock,
            commands::delete_mock,
            commands::toggle_mock,
            // Network
            commands::get_network_profile,
            commands::update_network_profile,
            commands::clear_network_profile,
            // Tunnels
            commands::start_tunnel,
            commands::stop_tunnel,
            // Status
            commands::get_status,
            // Daemon Management
            commands::get_daemon_info,
            commands::start_daemon,
            commands::stop_daemon,
            commands::restart_daemon,
            // Certs
            commands::get_cert_status,
            commands::trust_ca,
            commands::untrust_ca,
            // CLI Installation
            commands::get_cli_status,
            commands::install_cli,
            commands::uninstall_cli,
            // Utility
            commands::open_in_browser,
        ])
        .run(tauri::generate_context!())
        .expect("error running PortZero dashboard");
}

