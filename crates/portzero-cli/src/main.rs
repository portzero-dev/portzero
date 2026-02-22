//! PortZero CLI — the main entry point.
//!
//! # Argument disambiguation
//!
//! 1. If the first arg is a known subcommand → treat as subcommand.
//! 2. If the first arg resolves as an executable → treat as command, name = `basename(cwd)`.
//! 3. Otherwise → first arg is app name, rest is command.
//!
//! # Examples
//!
//! ```sh
//! portzero next dev              # name = cwd basename, command = "next dev"
//! portzero my-app next dev       # name = "my-app", command = "next dev"
//! portzero list                  # list active apps
//! portzero up                    # start all from portzero.toml
//! portzero logs my-app           # tail logs
//! portzero trust                 # install CA into system trust store
//! ```

mod commands;
mod daemon;

use clap::{Parser, Subcommand};

/// PortZero — local development reverse proxy, process manager & traffic inspector.
#[derive(Debug, Parser)]
#[command(
    name = "portzero",
    version,
    about = "Local dev proxy with traffic inspection, request replay, mocking & AI integration",
    after_help = "EXAMPLES:\n  \
        portzero next dev              Run 'next dev' as <cwd-name>.localhost:1337\n  \
        portzero my-app next dev       Run 'next dev' as my-app.localhost:1337\n  \
        portzero list                  List active apps\n  \
        portzero up                    Start all apps from portzero.toml\n  \
        portzero logs my-app           Tail logs for an app"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Raw arguments for the run command (when no subcommand matches).
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run a command as a managed app (explicit form).
    Run {
        /// App name
        name: String,
        /// Command to run
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, required = true)]
        command: Vec<String>,
        /// Don't auto-restart on crash
        #[arg(long)]
        no_restart: bool,
    },

    /// List all active apps and their URLs.
    List,

    /// Start all apps defined in portzero.toml.
    Up,

    /// Stop all running apps.
    Down,

    /// Tail logs for a managed app.
    Logs {
        /// App name
        name: String,
        /// Number of lines to show
        #[arg(short = 'n', long, default_value = "100")]
        lines: usize,
        /// Follow (tail -f style)
        #[arg(short, long)]
        follow: bool,
    },

    /// Start the proxy daemon (foreground).
    Start {
        /// Run in background (daemonize).
        #[arg(short, long)]
        daemon: bool,
    },

    /// Stop the proxy daemon.
    Stop,

    /// Show daemon status.
    Status,

    /// Install the PortZero CA certificate into the system trust store.
    /// Requires sudo/admin password.
    Trust,

    /// Remove the PortZero CA certificate from the system trust store.
    Untrust,

    /// Mock response rules.
    Mock {
        #[command(subcommand)]
        cmd: commands::mock::MockCommand,
    },

    /// Network throttle simulation.
    Throttle {
        #[command(subcommand)]
        cmd: commands::throttle::ThrottleCommand,
    },

    /// Public tunnel sharing.
    #[cfg(feature = "tunnel")]
    Share {
        #[command(subcommand)]
        cmd: commands::share::ShareCommand,
    },

    /// Log in to the tunnel relay service for public sharing.
    ///
    /// Default relay: https://tunnel.kfs.es (free hosted service).
    /// Self-hosted: pass --relay to point to your own LocalUp relay.
    /// Alternative: set PORTZERO_TUNNEL_TOKEN env var or [tunnel] token in portzero.toml.
    #[cfg(feature = "tunnel")]
    Login {
        /// Relay API URL (default: https://tunnel.kfs.es)
        #[arg(long)]
        relay: Option<String>,
    },

    /// Log out from the tunnel relay service.
    #[cfg(feature = "tunnel")]
    Logout,

    /// Show the currently logged-in user and relay info.
    #[cfg(feature = "tunnel")]
    Whoami,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    // Determine the state directory
    let state_dir = state_dir()?;
    std::fs::create_dir_all(&state_dir)?;

    match cli.command {
        Some(Command::Run {
            name,
            command,
            no_restart,
        }) => {
            commands::run::run(name, command, !no_restart, &state_dir).await?;
        }
        Some(Command::List) => {
            commands::list::list(&state_dir).await?;
        }
        Some(Command::Up) => {
            commands::up::up(&state_dir).await?;
        }
        Some(Command::Down) => {
            commands::up::down(&state_dir).await?;
        }
        Some(Command::Logs {
            name,
            lines,
            follow,
        }) => {
            commands::logs::logs(&name, lines, follow, &state_dir).await?;
        }
        Some(Command::Start { daemon: daemonize }) => {
            daemon::start(daemonize, &state_dir).await?;
        }
        Some(Command::Stop) => {
            daemon::stop(&state_dir).await?;
        }
        Some(Command::Status) => {
            daemon::status(&state_dir).await?;
        }
        Some(Command::Trust) => {
            commands::trust::trust(&state_dir)?;
        }
        Some(Command::Untrust) => {
            commands::trust::untrust(&state_dir)?;
        }
        Some(Command::Mock { cmd: _ }) => {
            // Task 4 commands need a running engine instance.
            // In the daemon model, these would connect via API.
            // For now, stub with a message.
            eprintln!(
                "Mock commands require a running daemon.\n\
                 Start the daemon first with: portzero start\n\
                 Then use: portzero mock <subcommand>"
            );
            // When integrated with the daemon, this would be:
            // commands::mock::execute(&cmd, &mock_engine)?;
        }
        Some(Command::Throttle { cmd: _ }) => {
            eprintln!(
                "Throttle commands require a running daemon.\n\
                 Start the daemon first with: portzero start"
            );
        }
        #[cfg(feature = "tunnel")]
        Some(Command::Share { cmd }) => {
            commands::share::execute_via_daemon(&cmd, &state_dir).await?;
        }
        #[cfg(feature = "tunnel")]
        Some(Command::Login { relay }) => {
            commands::auth::login(&state_dir, relay.as_deref()).await?;
        }
        #[cfg(feature = "tunnel")]
        Some(Command::Logout) => {
            commands::auth::logout(&state_dir).await?;
        }
        #[cfg(feature = "tunnel")]
        Some(Command::Whoami) => {
            commands::auth::whoami(&state_dir).await?;
        }
        None => {
            // No subcommand — try argument disambiguation for the "run" shorthand
            if cli.args.is_empty() {
                use clap::CommandFactory;
                Cli::command().print_help()?;
                return Ok(());
            }

            let (name, command) = disambiguate_args(&cli.args);
            commands::run::run(name, command, true, &state_dir).await?;
        }
    }

    Ok(())
}

/// Disambiguate raw args into (app_name, command).
///
/// Rules:
/// 1. If first arg is an executable in PATH or ./node_modules/.bin → name = basename(cwd), command = all args
/// 2. Otherwise → first arg is app name, rest is command
fn disambiguate_args(args: &[String]) -> (String, Vec<String>) {
    let first = &args[0];

    if is_executable(first) {
        let name = cwd_basename();
        (name, args.to_vec())
    } else if args.len() > 1 {
        (first.clone(), args[1..].to_vec())
    } else {
        (first.clone(), vec![])
    }
}

/// Check if a command name resolves to an executable.
fn is_executable(name: &str) -> bool {
    if which::which(name).is_ok() {
        return true;
    }

    let local_bin = std::path::Path::new("node_modules/.bin").join(name);
    local_bin.exists()
}

/// Get the basename of the current working directory.
fn cwd_basename() -> String {
    std::env::current_dir()
        .ok()
        .and_then(|p| p.file_name().map(|f| f.to_string_lossy().to_string()))
        .unwrap_or_else(|| "app".to_string())
}

/// Determine the state directory for PortZero data.
fn state_dir() -> anyhow::Result<std::path::PathBuf> {
    if let Ok(dir) = std::env::var("PORTZERO_STATE_DIR") {
        return Ok(std::path::PathBuf::from(dir));
    }

    let home = dirs_next::home_dir()
        .ok_or_else(|| anyhow::anyhow!("could not determine home directory"))?;
    Ok(home.join(".portzero"))
}
