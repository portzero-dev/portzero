use anyhow::Result;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager,
};

/// Create the system tray with menu items.
pub fn create_tray(app: &AppHandle) -> Result<()> {
    let show = MenuItem::with_id(app, "show", "Show Dashboard", true, None::<&str>)?;
    let separator1 = PredefinedMenuItem::separator(app)?;
    let start_daemon = MenuItem::with_id(app, "start_daemon", "Start Daemon", true, None::<&str>)?;
    let stop_daemon = MenuItem::with_id(app, "stop_daemon", "Stop Daemon", true, None::<&str>)?;
    let restart_daemon =
        MenuItem::with_id(app, "restart_daemon", "Restart Daemon", true, None::<&str>)?;
    let separator2 = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit PortZero", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[
            &show,
            &separator1,
            &start_daemon,
            &stop_daemon,
            &restart_daemon,
            &separator2,
            &quit,
        ],
    )?;

    TrayIconBuilder::new()
        .icon(
            app.default_window_icon()
                .cloned()
                .unwrap_or_else(|| tauri::image::Image::new(&[], 0, 0)),
        )
        .menu(&menu)
        .tooltip("PortZero")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "start_daemon" => {
                let handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    let state_dir = crate::default_state_dir();
                    match crate::daemon_bridge::start_daemon(&state_dir).await {
                        Ok(()) => tracing::info!("Daemon started via tray"),
                        Err(e) => tracing::error!("Failed to start daemon via tray: {e}"),
                    }
                    // Trigger a frontend refresh by emitting a custom event
                    let _ = tauri::Emitter::emit(&handle, "daemon-state-changed", ());
                });
            }
            "stop_daemon" => {
                let handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    let state_dir = crate::default_state_dir();
                    match crate::daemon_bridge::stop_daemon(&state_dir).await {
                        Ok(()) => tracing::info!("Daemon stopped via tray"),
                        Err(e) => tracing::error!("Failed to stop daemon via tray: {e}"),
                    }
                    let _ = tauri::Emitter::emit(&handle, "daemon-state-changed", ());
                });
            }
            "restart_daemon" => {
                let handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    let state_dir = crate::default_state_dir();
                    match crate::daemon_bridge::restart_daemon(&state_dir).await {
                        Ok(()) => tracing::info!("Daemon restarted via tray"),
                        Err(e) => tracing::error!("Failed to restart daemon via tray: {e}"),
                    }
                    let _ = tauri::Emitter::emit(&handle, "daemon-state-changed", ());
                });
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .build(app)?;

    Ok(())
}
