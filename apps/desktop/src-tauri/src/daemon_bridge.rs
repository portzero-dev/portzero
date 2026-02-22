use anyhow::{Context, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};
use tokio::fs;

/// Get the PortZero state directory (~/.portzero/).
fn default_state_dir() -> PathBuf {
    dirs_next::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".portzero")
}

/// PID file path — must match the CLI daemon's `portzero.pid`.
fn pid_file_path(state_dir: &Path) -> PathBuf {
    state_dir.join("portzero.pid")
}

// ---------------------------------------------------------------------------
// Status
// ---------------------------------------------------------------------------

/// Detailed daemon status returned to the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct DaemonRunInfo {
    /// Whether the daemon process is alive.
    pub running: bool,
    /// The daemon's PID (if known).
    pub pid: Option<u32>,
    /// Whether the control socket is responsive (daemon is healthy).
    pub responsive: bool,
}

/// Check if the daemon process is currently running.
///
/// Uses two checks:
/// 1. PID file + process alive check (fast)
/// 2. Control socket ping (authoritative — daemon is actually responsive)
pub async fn get_daemon_info(state_dir: &Path) -> DaemonRunInfo {
    let pid = read_daemon_pid(state_dir).await.ok();
    let process_alive = pid.map(is_process_alive).unwrap_or(false);

    // The control socket ping is the authoritative check — a stale PID file
    // can point to a recycled PID that belongs to a completely different process.
    let responsive = is_daemon_responsive(state_dir).await;

    // The daemon is truly "running" only if the control socket responds.
    // A live PID without a responsive socket means either:
    // - The PID was recycled (stale PID file) — not our daemon
    // - The daemon is starting up / hung — show as not-responding
    DaemonRunInfo {
        running: responsive,
        pid: if responsive || process_alive {
            pid
        } else {
            None
        },
        responsive,
    }
}

/// Ping the daemon's control socket to check if it's actually responsive.
async fn is_daemon_responsive(state_dir: &Path) -> bool {
    if let Some(mut client) = portzero_core::control::ControlClient::connect(state_dir).await {
        client.ping().await
    } else {
        false
    }
}

/// Read the daemon PID from the PID file.
async fn read_daemon_pid(state_dir: &Path) -> Result<u32> {
    let content = fs::read_to_string(pid_file_path(state_dir))
        .await
        .context("Failed to read portzero.pid")?;
    let pid: u32 = content
        .trim()
        .parse()
        .context("Invalid PID in portzero.pid")?;
    Ok(pid)
}

// ---------------------------------------------------------------------------
// Start / Stop / Restart
// ---------------------------------------------------------------------------

/// Start the daemon. Returns an error if it's already running.
pub async fn start_daemon(state_dir: &Path) -> Result<()> {
    let info = get_daemon_info(state_dir).await;
    if info.running {
        anyhow::bail!("Daemon is already running (PID {})", info.pid.unwrap_or(0));
    }

    // Clean up stale PID/socket files from a previous crashed daemon
    let pid_file = pid_file_path(state_dir);
    if pid_file.exists() {
        let _ = fs::remove_file(&pid_file).await;
    }
    let sock_file = state_dir.join("portzero.sock");
    if sock_file.exists() {
        let _ = fs::remove_file(&sock_file).await;
    }

    tracing::info!("Starting daemon...");

    let portzero_bin = find_portzero_binary()?;

    // Spawn the daemon process. It runs `portzero start` which blocks,
    // so we spawn it detached. The child writes its own PID file.
    let _child = tokio::process::Command::new(&portzero_bin)
        .arg("start")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .context("Failed to spawn portzero daemon")?;

    // Wait for it to become responsive (up to 3 seconds)
    for _ in 0..15 {
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        if is_daemon_responsive(state_dir).await {
            tracing::info!("Daemon started successfully");
            return Ok(());
        }
    }

    anyhow::bail!("Daemon process spawned but did not become responsive within 3 seconds")
}

/// Stop the daemon. Escalates through SIGQUIT → SIGTERM → SIGKILL.
pub async fn stop_daemon(state_dir: &Path) -> Result<()> {
    // First verify the daemon is actually responsive via control socket.
    // Don't kill a random process from a stale PID file.
    let responsive = is_daemon_responsive(state_dir).await;

    let pid = read_daemon_pid(state_dir).await.ok();

    if !responsive {
        // Daemon isn't running — just clean up stale files
        if let Some(pid) = pid {
            tracing::debug!(
                pid = pid,
                "PID file exists but daemon not responsive, cleaning up"
            );
        }
        let _ = fs::remove_file(pid_file_path(state_dir)).await;
        let _ = fs::remove_file(state_dir.join("portzero.sock")).await;
        return Ok(());
    }

    let pid = pid.context("Daemon is responsive but no PID file found")?;

    if !is_process_alive(pid) {
        // Socket responds but PID is dead — very unlikely, but clean up
        let _ = fs::remove_file(pid_file_path(state_dir)).await;
        let _ = fs::remove_file(state_dir.join("portzero.sock")).await;
        return Ok(());
    }

    #[cfg(unix)]
    {
        // Pingora signal semantics:
        //   SIGTERM = graceful terminate (what we want)
        //   SIGINT  = fast shutdown
        //   SIGQUIT = graceful upgrade (waits for new process — NOT a shutdown)

        // 1) SIGTERM — graceful Pingora shutdown
        tracing::info!(pid = pid, "Sending SIGTERM to daemon");
        let _ = send_signal(pid, "TERM");

        // Wait up to 5 seconds for graceful shutdown
        for _ in 0..25 {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            if !is_process_alive(pid) {
                cleanup_files(state_dir).await;
                tracing::info!("Daemon stopped (SIGTERM)");
                return Ok(());
            }
        }

        // 2) Escalate to SIGINT (fast shutdown)
        tracing::warn!(pid = pid, "SIGTERM didn't work, sending SIGINT");
        let _ = send_signal(pid, "INT");

        for _ in 0..10 {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            if !is_process_alive(pid) {
                cleanup_files(state_dir).await;
                tracing::info!("Daemon stopped (SIGINT)");
                return Ok(());
            }
        }

        // 3) Last resort: SIGKILL
        tracing::warn!(pid = pid, "SIGINT didn't work, sending SIGKILL");
        let _ = send_signal(pid, "KILL");

        for _ in 0..5 {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            if !is_process_alive(pid) {
                cleanup_files(state_dir).await;
                tracing::info!("Daemon stopped (SIGKILL)");
                return Ok(());
            }
        }
    }

    #[cfg(windows)]
    {
        let _ = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F"])
            .status();

        for _ in 0..10 {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            if !is_process_alive(pid) {
                cleanup_files(state_dir).await;
                return Ok(());
            }
        }
    }

    // If we get here, even SIGKILL didn't work (shouldn't happen)
    cleanup_files(state_dir).await;
    anyhow::bail!("Daemon (PID {pid}) could not be stopped")
}

#[cfg(unix)]
fn send_signal(pid: u32, signal: &str) -> Result<()> {
    std::process::Command::new("kill")
        .args([&format!("-{signal}"), &pid.to_string()])
        .status()
        .context(format!("Failed to send SIG{signal} to PID {pid}"))?;
    Ok(())
}

async fn cleanup_files(state_dir: &Path) {
    let _ = fs::remove_file(pid_file_path(state_dir)).await;
    let _ = fs::remove_file(state_dir.join("portzero.sock")).await;
}

/// Restart the daemon: stop then start.
pub async fn restart_daemon(state_dir: &Path) -> Result<()> {
    let info = get_daemon_info(state_dir).await;
    if info.running {
        stop_daemon(state_dir).await?;
        // Brief pause to ensure socket file is cleaned up
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    }
    start_daemon(state_dir).await
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Find the portzero CLI binary.
pub fn find_portzero_binary() -> Result<PathBuf> {
    let candidates = [
        // In PATH
        which::which("portzero").ok(),
        // Relative to this binary (cargo build output — works during dev)
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("portzero"))),
    ];

    for candidate in candidates.into_iter().flatten() {
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    anyhow::bail!("Could not find portzero binary. Install the CLI first (Settings → CLI Tool).")
}

/// Check if a process with the given PID is alive.
fn is_process_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        // kill(pid, 0) checks if process exists without sending a signal
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }

    #[cfg(windows)]
    {
        use std::process::Command;
        Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains(&pid.to_string()))
            .unwrap_or(false)
    }

    #[cfg(not(any(unix, windows)))]
    {
        let _ = pid;
        false
    }
}
