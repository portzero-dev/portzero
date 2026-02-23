//! CLI command: `portzero throttle`
//!
//! Manage network simulation profiles for apps via the daemon's control socket.
//!
//! # Usage
//!
//! ```sh
//! portzero throttle set my-app --latency 2000             # Add 2s latency
//! portzero throttle set my-app --latency 200 --jitter 50  # 200ms +/- 50ms
//! portzero throttle set my-app --drop 0.1                 # 10% packet loss
//! portzero throttle set my-app --bandwidth 50000          # 50 KB/s
//! portzero throttle set my-app --path "/api/*" --drop 0.5 # 50% loss on API only
//! portzero throttle list                                  # List active profiles
//! portzero throttle clear my-app                          # Remove simulation
//! ```

use anyhow::Result;
use clap::Subcommand;
use portzero_core::control::{ControlClient, ControlRequest, ControlResponse};
use portzero_core::types::SetNetworkProfile;
use std::path::Path;

/// Throttle subcommands.
#[derive(Debug, Subcommand)]
pub enum ThrottleCommand {
    /// Set network simulation for an app
    Set {
        /// App name
        app: String,
        /// Fixed latency in milliseconds
        #[arg(long)]
        latency: Option<u64>,
        /// Random jitter +/- milliseconds (applied on top of latency)
        #[arg(long)]
        jitter: Option<u64>,
        /// Packet loss probability (0.0 - 1.0)
        #[arg(long)]
        drop: Option<f64>,
        /// Bandwidth limit in bytes per second
        #[arg(long)]
        bandwidth: Option<u64>,
        /// Only apply to matching paths (glob pattern)
        #[arg(long)]
        path: Option<String>,
    },
    /// List active network simulation profiles
    List,
    /// Clear network simulation for an app
    Clear {
        /// App name
        app: String,
    },
}

/// Execute a throttle command via the daemon's control socket.
pub async fn execute_via_daemon(cmd: &ThrottleCommand, state_dir: &Path) -> Result<()> {
    let Some(mut client) = ControlClient::connect(state_dir).await else {
        anyhow::bail!(
            "Cannot connect to daemon. Is it running?\n\
             Start it with: portzero start"
        );
    };

    match cmd {
        ThrottleCommand::Set {
            app,
            latency,
            jitter,
            drop,
            bandwidth,
            path,
        } => {
            // Validate inputs
            if let Some(rate) = drop {
                if !(0.0..=1.0).contains(rate) {
                    anyhow::bail!("Packet loss rate must be between 0.0 and 1.0, got {}", rate);
                }
            }

            if jitter.is_some() && latency.is_none() {
                anyhow::bail!("--jitter requires --latency to be set");
            }

            let profile = SetNetworkProfile {
                latency_ms: *latency,
                jitter_ms: *jitter,
                packet_loss_rate: *drop,
                bandwidth_limit: *bandwidth,
                path_filter: path.clone(),
            };

            match client.set_network_profile(app, profile).await {
                Ok(np) => {
                    println!("Network simulation set for '{}':", app);
                    if let Some(lat) = np.latency_ms {
                        print!("  Latency: {}ms", lat);
                        if let Some(j) = np.jitter_ms {
                            print!(" +/- {}ms", j);
                        }
                        println!();
                    }
                    if np.packet_loss_rate > 0.0 {
                        println!("  Packet loss: {:.0}%", np.packet_loss_rate * 100.0);
                    }
                    if let Some(bw) = np.bandwidth_limit {
                        println!("  Bandwidth limit: {} B/s ({})", bw, format_bytes(bw));
                    }
                    if let Some(p) = &np.path_filter {
                        println!("  Path filter: {}", p);
                    }
                }
                Err(e) => anyhow::bail!("Failed to set network profile: {e}"),
            }
        }

        ThrottleCommand::List => {
            // Get list of registered apps, then query each for their profile
            match client.request(&ControlRequest::List).await {
                Ok(ControlResponse::Apps { apps }) => {
                    let app_names: Vec<String> = apps
                        .iter()
                        .filter(|a| a.name != "_portzero")
                        .map(|a| a.name.clone())
                        .collect();

                    if app_names.is_empty() {
                        println!("No apps registered.");
                        return Ok(());
                    }

                    let mut found_any = false;

                    // Need a fresh client for each subsequent request
                    // (our client was consumed by the List request above,
                    //  but ControlClient supports multiple requests on one connection)
                    for name in &app_names {
                        match client.get_network_profile(name).await {
                            Ok(p) => {
                                // Only show if something is actually configured
                                let has_config = p.latency_ms.is_some()
                                    || p.packet_loss_rate > 0.0
                                    || p.bandwidth_limit.is_some();

                                if has_config {
                                    if !found_any {
                                        println!(
                                            "{:<15}  {:<12}  {:<12}  {:<10}  {:<15}  {}",
                                            "App",
                                            "Latency",
                                            "Jitter",
                                            "Loss",
                                            "Bandwidth",
                                            "Path Filter"
                                        );
                                        println!("{}", "-".repeat(80));
                                        found_any = true;
                                    }

                                    let latency = p
                                        .latency_ms
                                        .map(|l| format!("{}ms", l))
                                        .unwrap_or_else(|| "-".to_string());
                                    let jitter = p
                                        .jitter_ms
                                        .map(|j| format!("+/-{}ms", j))
                                        .unwrap_or_else(|| "-".to_string());
                                    let loss = if p.packet_loss_rate > 0.0 {
                                        format!("{:.0}%", p.packet_loss_rate * 100.0)
                                    } else {
                                        "-".to_string()
                                    };
                                    let bw = p
                                        .bandwidth_limit
                                        .map(format_bytes)
                                        .unwrap_or_else(|| "-".to_string());
                                    let path_filter = p.path_filter.as_deref().unwrap_or("-");

                                    println!(
                                        "{:<15}  {:<12}  {:<12}  {:<10}  {:<15}  {}",
                                        p.app_name, latency, jitter, loss, bw, path_filter
                                    );
                                }
                            }
                            Err(_) => {} // skip apps we can't query
                        }
                    }

                    if !found_any {
                        println!("No active network simulation profiles.");
                    }
                }
                _ => {
                    println!("No active network simulation profiles.");
                }
            }
        }

        ThrottleCommand::Clear { app } => match client.clear_network_profile(app).await {
            Ok(()) => println!("Network simulation cleared for '{}'.", app),
            Err(e) => anyhow::bail!("Failed to clear network profile: {e}"),
        },
    }
    Ok(())
}

/// Format bytes as human-readable string.
fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_048_576 {
        format!("{:.1} MB/s", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KB/s", bytes as f64 / 1024.0)
    } else {
        format!("{} B/s", bytes)
    }
}
