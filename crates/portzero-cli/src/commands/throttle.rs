//! CLI command: `portzero throttle`
//!
//! Manage network simulation profiles for apps.
//!
//! # Usage
//!
//! ```sh
//! portzero throttle my-app --latency 2000             # Add 2s latency
//! portzero throttle my-app --latency 200 --jitter 50  # 200ms +/- 50ms
//! portzero throttle my-app --drop 0.1                 # 10% packet loss
//! portzero throttle my-app --bandwidth 50000          # 50 KB/s
//! portzero throttle my-app --path "/api/*" --drop 0.5 # 50% loss on API only
//! portzero throttle list                              # List active profiles
//! portzero throttle clear my-app                      # Remove simulation
//! ```

use clap::Subcommand;
use portzero_core::network_sim::NetworkSim;
use portzero_core::types::NetworkProfile;
use std::sync::Arc;

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

/// Execute a throttle command.
pub fn execute(cmd: &ThrottleCommand, network_sim: &Arc<NetworkSim>) -> anyhow::Result<()> {
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

            let profile = NetworkProfile {
                app_name: app.clone(),
                latency_ms: *latency,
                jitter_ms: *jitter,
                packet_loss_rate: drop.unwrap_or(0.0),
                bandwidth_limit: *bandwidth,
                path_filter: path.clone(),
            };

            network_sim.set_profile(profile);

            println!("Network simulation set for '{}':", app);
            if let Some(lat) = latency {
                print!("  Latency: {}ms", lat);
                if let Some(j) = jitter {
                    print!(" +/- {}ms", j);
                }
                println!();
            }
            if let Some(rate) = drop {
                if *rate > 0.0 {
                    println!("  Packet loss: {:.0}%", rate * 100.0);
                }
            }
            if let Some(bw) = bandwidth {
                println!("  Bandwidth limit: {} B/s ({})", bw, format_bytes(*bw));
            }
            if let Some(p) = path {
                println!("  Path filter: {}", p);
            }
        }

        ThrottleCommand::List => {
            let profiles = network_sim.list_profiles();
            if profiles.is_empty() {
                println!("No active network simulation profiles.");
                return Ok(());
            }

            println!(
                "{:<15}  {:<12}  {:<12}  {:<10}  {:<15}  {}",
                "App", "Latency", "Jitter", "Loss", "Bandwidth", "Path Filter"
            );
            println!("{}", "-".repeat(80));

            for p in &profiles {
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
                    .map(|b| format_bytes(b))
                    .unwrap_or_else(|| "-".to_string());
                let path_filter = p.path_filter.as_deref().unwrap_or("-");

                println!(
                    "{:<15}  {:<12}  {:<12}  {:<10}  {:<15}  {}",
                    p.app_name, latency, jitter, loss, bw, path_filter
                );
            }
        }

        ThrottleCommand::Clear { app } => {
            if network_sim.clear_profile(app) {
                println!("Network simulation cleared for '{}'.", app);
            } else {
                println!("No active simulation for '{}'.", app);
            }
        }
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
