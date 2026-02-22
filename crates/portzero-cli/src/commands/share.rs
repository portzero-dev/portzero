//! CLI command: `portzero share`
//!
//! Expose a local app to the internet via a public tunnel.
//!
//! # Usage
//!
//! ```sh
//! portzero share start my-app                              # Share with default settings
//! portzero share start my-app --subdomain custom-name      # Custom subdomain
//! portzero share start my-app --relay relay.example.com    # Use specific relay
//! portzero share stop my-app                               # Stop sharing
//! portzero share list                                      # List active tunnels
//! ```

use clap::Subcommand;
use portzero_core::control::ControlClient;
use std::path::Path;

/// Share subcommands.
#[derive(Debug, Subcommand)]
pub enum ShareCommand {
    /// Start sharing an app (expose via public tunnel)
    Start {
        /// App name
        app: String,
        /// Custom subdomain for the public URL
        #[arg(long)]
        subdomain: Option<String>,
        /// Relay server to use (overrides config)
        #[arg(long)]
        relay: Option<String>,
    },
    /// Stop sharing an app
    Stop {
        /// App name
        app: String,
    },
    /// List all active tunnels
    List,
}

/// Execute a share command via the daemon's control socket.
pub async fn execute_via_daemon(cmd: &ShareCommand, state_dir: &Path) -> anyhow::Result<()> {
    let mut client = match ControlClient::connect(state_dir).await {
        Some(c) => c,
        None => {
            anyhow::bail!(
                "Cannot connect to daemon. Is it running?\n\
                 Start it with: portzero start"
            );
        }
    };

    match cmd {
        ShareCommand::Start {
            app,
            subdomain,
            relay,
        } => {
            // Check if a tunnel token is available from any source
            let resolved = portzero_core::credentials::resolve_tunnel_config(state_dir, None);
            if resolved.auth_token.is_none() {
                anyhow::bail!(
                    "No tunnel authentication token found.\n\n\
                     To enable tunnels, use one of:\n  \
                     1. portzero login                        (hosted service)\n  \
                     2. PORTZERO_TUNNEL_TOKEN=<jwt>           (env var)\n  \
                     3. [tunnel] token = \"<jwt>\" in portzero.toml  (config file)\n\n\
                     Self-hosted relay users: generate a token with `localup generate-token`"
                );
            }

            println!("Starting tunnel for '{}'...", app);

            let info = client
                .share(
                    app,
                    subdomain.as_deref(),
                    relay.as_deref(),
                )
                .await?;

            println!();
            println!("Tunnel started for '{}':", app);
            println!("  Public URL: {}", info.public_url);
            println!("  Relay:      {}", info.relay);
            println!();
            println!("All traffic through the tunnel is captured in the dashboard.");
            println!("Run `portzero share stop {}` to stop sharing.", app);
        }

        ShareCommand::Stop { app } => {
            client.unshare(app).await?;
            println!("Tunnel stopped for '{}'.", app);
        }

        ShareCommand::List => {
            let tunnels = client.list_tunnels().await?;
            if tunnels.is_empty() {
                println!("No active tunnels.");
                return Ok(());
            }

            println!(
                "{:<15}  {:<40}  {:<25}  {}",
                "App", "Public URL", "Relay", "Started At"
            );
            println!("{}", "-".repeat(95));

            for t in &tunnels {
                println!(
                    "{:<15}  {:<40}  {:<25}  {}",
                    t.app_name,
                    t.public_url,
                    t.relay,
                    t.started_at.format("%Y-%m-%d %H:%M:%S"),
                );
            }
        }
    }

    Ok(())
}
