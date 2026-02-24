//! CLI command: `portzero list`
//!
//! Lists all active apps with their status, URL, port, and PID.
//! Queries the running daemon via the control socket, matching the
//! same data flow used by the desktop app.

use anyhow::Result;
use chrono::Utc;
use portzero_core::control::{ControlClient, ControlRequest, ControlResponse};
use portzero_core::types::DEFAULT_PROXY_PORT;
use std::path::Path;

/// List active apps by querying the daemon via the control socket.
pub async fn list(state_dir: &Path) -> Result<()> {
    // Try connecting to the daemon's control socket (same approach as the desktop app)
    let Some(mut client) = ControlClient::connect(state_dir).await else {
        println!("Daemon not running.");
        println!("Start it with: portzero start");
        return Ok(());
    };

    match client.request(&ControlRequest::List).await {
        Ok(ControlResponse::Apps { apps }) => {
            // Filter out the internal _portzero dashboard route
            let apps: Vec<_> = apps.iter().filter(|a| a.name != "_portzero").collect();

            if apps.is_empty() {
                println!("PortZero — http://localhost:{}", DEFAULT_PROXY_PORT);
                println!();
                println!("No apps running.");
                println!();
                println!("Start an app with: portzero <name> <command>");
                println!("Or start all from config: portzero up");
                return Ok(());
            }

            println!("PortZero — http://localhost:{}", DEFAULT_PROXY_PORT);
            println!();

            for app in &apps {
                let uptime = {
                    let secs = (Utc::now() - app.started_at).num_seconds().max(0);
                    format_uptime(secs as u64)
                };
                let cmd = app.command.join(" ");

                println!(
                    "  {} → {} (port {} | PID {} | up {})",
                    app.name, app.url, app.port, app.pid, uptime
                );
                println!("    $ {}", cmd);
                println!();
            }

            println!("{} app(s) running", apps.len());
        }
        Ok(ControlResponse::Error { message }) => {
            anyhow::bail!("Daemon error: {message}");
        }
        Ok(_) => {
            anyhow::bail!("Unexpected response from daemon");
        }
        Err(e) => {
            anyhow::bail!("Failed to communicate with daemon: {e}");
        }
    }

    Ok(())
}

fn format_uptime(total_secs: u64) -> String {
    let days = total_secs / 86400;
    let hours = (total_secs % 86400) / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;

    if days > 0 {
        format!("{}d {}h {}m", days, hours, mins)
    } else if hours > 0 {
        format!("{}h {}m", hours, mins)
    } else if mins > 0 {
        format!("{}m {}s", mins, secs)
    } else {
        format!("{}s", secs)
    }
}
