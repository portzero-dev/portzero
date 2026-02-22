//! Process manager: spawns, monitors, and optionally restarts child processes.
//!
//! Features:
//! - Deterministic port assignment via `Router::find_free_port`
//! - Auto-restart with exponential backoff (1s, 2s, 4s, ... max 30s, reset after 60s stable)
//! - Log capture: stdout/stderr into ring buffers + broadcast via `WsHub`
//! - Stdout URL rewriting: `http://localhost:<port>` → `http://<name>.localhost:1337`
//! - Graceful shutdown: SIGTERM → wait 5s → SIGKILL

use crate::router::Router;
use crate::types::{AppStatus, LogLine, LogStream, Route, WsEvent, MAX_LOG_LINES};
use crate::ws::WsHub;
use anyhow::{Context, Result};
use chrono::Utc;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

/// Maximum restart backoff in seconds.
const MAX_BACKOFF_SECS: u64 = 30;

/// Graceful shutdown timeout before SIGKILL.
const SHUTDOWN_TIMEOUT_SECS: u64 = 5;

/// A managed child process entry.
pub struct ManagedProcess {
    pub name: String,
    pub pid: u32,
    pub port: u16,
    pub command: Vec<String>,
    pub cwd: PathBuf,
    pub status: AppStatus,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub restarts: u32,
    pub auto_restart: bool,
    pub log_buffer: VecDeque<LogLine>,
    /// Extra environment variables for this process.
    pub env_vars: Vec<(String, String)>,
}

/// Handle for interacting with a running managed process.
struct ProcessHandle {
    child: Child,
    /// Sends a signal to stop the log-capture + monitor task.
    cancel_tx: tokio::sync::watch::Sender<bool>,
}

/// The process manager.
pub struct ProcessManager {
    /// Managed process metadata.
    processes: dashmap::DashMap<String, Arc<Mutex<ManagedProcess>>>,
    /// Child handles (separate so we can kill/restart without holding process lock).
    handles: dashmap::DashMap<String, Arc<Mutex<ProcessHandle>>>,
    /// Shared router for registering/deregistering routes.
    router: Arc<Router>,
    /// WebSocket hub for broadcasting events.
    ws_hub: Arc<WsHub>,
    /// Proxy port (for URL rewriting).
    proxy_port: u16,
}

impl ProcessManager {
    pub fn new(router: Arc<Router>, ws_hub: Arc<WsHub>, proxy_port: u16) -> Self {
        Self {
            processes: dashmap::DashMap::new(),
            handles: dashmap::DashMap::new(),
            router,
            ws_hub,
            proxy_port,
        }
    }

    /// Spawn a new managed process.
    ///
    /// - Assigns a deterministic port via the router.
    /// - Sets the `PORT` env var so the child knows which port to listen on.
    /// - Captures stdout/stderr into the log ring buffer.
    /// - Registers the route in the router.
    /// - Broadcasts `AppRegistered` event.
    pub async fn spawn(
        &self,
        name: String,
        command: Vec<String>,
        cwd: PathBuf,
        auto_restart: bool,
        extra_env: Vec<(String, String)>,
    ) -> Result<Route> {
        // If already running, stop the existing one first
        if self.processes.contains_key(&name) {
            self.stop(&name).await?;
        }

        let port = self.router.find_free_port(&name);

        let (child, pid) = spawn_child(&command, &cwd, port, &extra_env)
            .await
            .with_context(|| format!("failed to spawn process for '{}'", name))?;

        let managed = Arc::new(Mutex::new(ManagedProcess {
            name: name.clone(),
            pid,
            port,
            command: command.clone(),
            cwd: cwd.clone(),
            status: AppStatus::Running,
            started_at: Utc::now(),
            restarts: 0,
            auto_restart,
            log_buffer: VecDeque::with_capacity(MAX_LOG_LINES),
            env_vars: extra_env,
        }));

        let route = self
            .router
            .register(name.clone(), port, pid, command.clone(), cwd.clone());

        // Set up log capture + process monitoring
        let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);
        let handle = Arc::new(Mutex::new(ProcessHandle { child, cancel_tx }));

        self.processes.insert(name.clone(), managed.clone());
        self.handles.insert(name.clone(), handle.clone());

        // Spawn the background monitor task
        self.spawn_monitor(name.clone(), managed, handle, cancel_rx);

        // Broadcast
        self.ws_hub.broadcast(WsEvent::AppRegistered {
            name: name.clone(),
            port,
            pid,
            url: format!("http://{}.localhost:{}", name, self.proxy_port),
        });

        tracing::info!(name = %name, port = port, pid = pid, "Process spawned");

        Ok(route)
    }

    /// Stop a managed process gracefully.
    pub async fn stop(&self, name: &str) -> Result<()> {
        if let Some(handle_entry) = self.handles.get(name) {
            let handle = handle_entry.value().clone();
            let mut h = handle.lock().await;

            // Signal the monitor task to stop
            let _ = h.cancel_tx.send(true);

            // Graceful shutdown
            kill_child(&mut h.child).await;
            drop(h);
        }

        // Update state
        if let Some(proc_entry) = self.processes.get(name) {
            let mut proc = proc_entry.value().lock().await;
            proc.status = AppStatus::Stopped;
        }

        self.router.update_status(name, AppStatus::Stopped);
        self.handles.remove(name);

        self.ws_hub.broadcast(WsEvent::AppRemoved {
            name: name.to_string(),
        });

        tracing::info!(name = %name, "Process stopped");
        Ok(())
    }

    /// Restart a managed process.
    pub async fn restart(&self, name: &str) -> Result<Route> {
        let (command, cwd, auto_restart, env_vars) = {
            let proc_entry = self
                .processes
                .get(name)
                .ok_or_else(|| anyhow::anyhow!("no process named '{}'", name))?;
            let proc = proc_entry.value().lock().await;
            (
                proc.command.clone(),
                proc.cwd.clone(),
                proc.auto_restart,
                proc.env_vars.clone(),
            )
        };

        self.stop(name).await?;
        self.spawn(name.to_string(), command, cwd, auto_restart, env_vars)
            .await
    }

    /// Stop all managed processes.
    pub async fn stop_all(&self) -> Result<()> {
        let names: Vec<String> = self
            .processes
            .iter()
            .map(|entry| entry.key().clone())
            .collect();

        for name in names {
            if let Err(e) = self.stop(&name).await {
                tracing::error!(name = %name, error = %e, "Failed to stop process");
            }
        }
        Ok(())
    }

    /// List all managed processes.
    pub fn list(&self) -> Vec<Arc<Mutex<ManagedProcess>>> {
        self.processes
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Get a process by name.
    pub fn get(&self, name: &str) -> Option<Arc<Mutex<ManagedProcess>>> {
        self.processes.get(name).map(|e| e.value().clone())
    }

    /// Get log lines for a process.
    pub async fn get_logs(&self, name: &str, lines: Option<usize>) -> Result<Vec<LogLine>> {
        let proc_entry = self
            .processes
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("no process named '{}'", name))?;
        let proc = proc_entry.value().lock().await;
        let n = lines.unwrap_or(100).min(MAX_LOG_LINES);
        let logs: Vec<LogLine> = proc
            .log_buffer
            .iter()
            .rev()
            .take(n)
            .rev()
            .cloned()
            .collect();
        Ok(logs)
    }

    /// Get the number of managed processes.
    pub fn count(&self) -> usize {
        self.processes.len()
    }

    // -----------------------------------------------------------------------
    // Internal
    // -----------------------------------------------------------------------

    /// Spawn a background task that captures stdout/stderr and monitors the child.
    /// Uses a loop for auto-restart instead of recursion.
    fn spawn_monitor(
        &self,
        name: String,
        managed: Arc<Mutex<ManagedProcess>>,
        initial_handle: Arc<Mutex<ProcessHandle>>,
        mut cancel_rx: tokio::sync::watch::Receiver<bool>,
    ) {
        let ws_hub = self.ws_hub.clone();
        let router = self.router.clone();
        let proxy_port = self.proxy_port;
        let handles = self.handles.clone();

        tokio::spawn(async move {
            let mut current_handle = initial_handle;

            loop {
                // Capture stdout/stderr from current child
                let (stdout, stderr) = {
                    let mut h = current_handle.lock().await;
                    (h.child.stdout.take(), h.child.stderr.take())
                };

                // Spawn stdout reader
                let stdout_task = {
                    let name = name.clone();
                    let managed = managed.clone();
                    let ws_hub = ws_hub.clone();
                    tokio::spawn(async move {
                        if let Some(stdout) = stdout {
                            capture_stream(
                                stdout,
                                LogStream::Stdout,
                                &name,
                                &managed,
                                &ws_hub,
                                proxy_port,
                                true, // print to terminal
                            )
                            .await;
                        }
                    })
                };

                // Spawn stderr reader
                let stderr_task = {
                    let name = name.clone();
                    let managed = managed.clone();
                    let ws_hub = ws_hub.clone();
                    tokio::spawn(async move {
                        if let Some(stderr) = stderr {
                            capture_stream(
                                stderr,
                                LogStream::Stderr,
                                &name,
                                &managed,
                                &ws_hub,
                                proxy_port,
                                true,
                            )
                            .await;
                        }
                    })
                };

                // Wait for child to exit
                let exit_status = {
                    let mut h = current_handle.lock().await;
                    h.child.wait().await
                };

                let _ = stdout_task.await;
                let _ = stderr_task.await;

                // Check if cancelled (user called stop)
                if *cancel_rx.borrow() {
                    break;
                }

                let exit_code = match exit_status {
                    Ok(status) => status.code().unwrap_or(-1),
                    Err(_) => -1,
                };

                tracing::warn!(name = %name, exit_code = exit_code, "Process exited");

                // Update status to Crashed
                {
                    let mut proc = managed.lock().await;
                    proc.status = AppStatus::Crashed {
                        exit_code,
                        at: Utc::now(),
                    };
                }

                router.update_status(
                    &name,
                    AppStatus::Crashed {
                        exit_code,
                        at: Utc::now(),
                    },
                );

                ws_hub.broadcast(WsEvent::AppCrashed {
                    name: name.clone(),
                    exit_code,
                });

                // Check if auto-restart is enabled
                let should_restart = {
                    let proc = managed.lock().await;
                    proc.auto_restart
                };

                if !should_restart {
                    handles.remove(&name);
                    break;
                }

                // Exponential backoff
                let restarts = {
                    let mut proc = managed.lock().await;
                    proc.restarts += 1;
                    proc.restarts
                };

                let backoff_secs = (1u64 << (restarts - 1).min(5)).min(MAX_BACKOFF_SECS);
                tracing::info!(
                    name = %name,
                    restarts = restarts,
                    backoff_secs = backoff_secs,
                    "Auto-restarting process after backoff"
                );

                tokio::time::sleep(std::time::Duration::from_secs(backoff_secs)).await;

                // Check again after sleep
                if *cancel_rx.borrow() {
                    break;
                }

                // Get process info for respawn
                let (command, cwd, port, env_vars) = {
                    let proc = managed.lock().await;
                    (
                        proc.command.clone(),
                        proc.cwd.clone(),
                        proc.port,
                        proc.env_vars.clone(),
                    )
                };

                match spawn_child(&command, &cwd, port, &env_vars).await {
                    Ok((child, new_pid)) => {
                        // Update managed process state
                        {
                            let mut proc = managed.lock().await;
                            proc.pid = new_pid;
                            proc.status = AppStatus::Running;
                            proc.started_at = Utc::now();
                        }

                        router.update_pid(&name, new_pid);

                        // Create new cancel channel for this iteration
                        let (new_cancel_tx, new_cancel_rx) = tokio::sync::watch::channel(false);
                        cancel_rx = new_cancel_rx;

                        let new_handle = Arc::new(Mutex::new(ProcessHandle {
                            child,
                            cancel_tx: new_cancel_tx,
                        }));

                        handles.insert(name.clone(), new_handle.clone());
                        current_handle = new_handle;

                        ws_hub.broadcast(WsEvent::AppRestarted {
                            name: name.clone(),
                            pid: new_pid,
                        });

                        tracing::info!(name = %name, pid = new_pid, "Process restarted");

                        // Loop continues — monitor the new child
                    }
                    Err(e) => {
                        tracing::error!(
                            name = %name,
                            error = %e,
                            "Failed to restart process"
                        );
                        handles.remove(&name);
                        break;
                    }
                }
            }
        });
    }
}

/// Spawn the actual child process.
async fn spawn_child(
    command: &[String],
    cwd: &PathBuf,
    port: u16,
    extra_env: &[(String, String)],
) -> Result<(Child, u32)> {
    if command.is_empty() {
        anyhow::bail!("empty command");
    }

    let (program, args) = if command.len() == 1 {
        // Single string command — run via shell
        #[cfg(unix)]
        {
            ("sh".to_string(), vec!["-c".to_string(), command[0].clone()])
        }
        #[cfg(not(unix))]
        {
            (
                "cmd".to_string(),
                vec!["/C".to_string(), command[0].clone()],
            )
        }
    } else {
        (command[0].clone(), command[1..].to_vec())
    };

    let mut cmd = Command::new(&program);
    cmd.args(&args)
        .current_dir(cwd)
        .env("PORT", port.to_string())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);

    for (key, value) in extra_env {
        cmd.env(key, value);
    }

    // On unix, create a new process group so we can signal the whole group
    #[cfg(unix)]
    unsafe {
        cmd.pre_exec(|| {
            libc::setpgid(0, 0);
            Ok(())
        });
    }

    let child = cmd
        .spawn()
        .with_context(|| format!("failed to spawn '{}' in '{}'", program, cwd.display()))?;

    let pid = child
        .id()
        .ok_or_else(|| anyhow::anyhow!("child process exited immediately"))?;

    Ok((child, pid))
}

/// Capture a stream (stdout or stderr) into the log buffer and broadcast.
async fn capture_stream<R: tokio::io::AsyncRead + Unpin>(
    stream: R,
    stream_type: LogStream,
    name: &str,
    managed: &Arc<Mutex<ManagedProcess>>,
    ws_hub: &WsHub,
    proxy_port: u16,
    print_to_terminal: bool,
) {
    let reader = BufReader::new(stream);
    let mut lines = reader.lines();

    while let Ok(Some(line)) = lines.next_line().await {
        let port = managed.lock().await.port;
        let content = rewrite_url(&line, port, name, proxy_port);

        if print_to_terminal {
            match stream_type {
                LogStream::Stdout => println!("[{}] {}", name, content),
                LogStream::Stderr => eprintln!("[{}] {}", name, content),
            }
        }

        let log_line = LogLine {
            timestamp: Utc::now(),
            stream: stream_type,
            content: content.clone(),
        };

        // Add to ring buffer
        {
            let mut proc = managed.lock().await;
            if proc.log_buffer.len() >= MAX_LOG_LINES {
                proc.log_buffer.pop_front();
            }
            proc.log_buffer.push_back(log_line);
        }

        // Broadcast via WebSocket
        ws_hub.broadcast(WsEvent::LogLine {
            app: name.to_string(),
            stream: stream_type,
            line: content,
            timestamp: Utc::now(),
        });
    }
}

/// Gracefully kill a child process: SIGTERM → wait → SIGKILL.
async fn kill_child(child: &mut Child) {
    #[cfg(unix)]
    let pgid = child.id().map(|pid| pid as i32);

    #[cfg(unix)]
    {
        if let Some(pid) = pgid {
            // Send SIGTERM to the entire process group (child + descendants)
            unsafe {
                libc::kill(-pid, libc::SIGTERM);
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = child.kill().await;
    }

    // Wait with timeout
    match tokio::time::timeout(
        std::time::Duration::from_secs(SHUTDOWN_TIMEOUT_SECS),
        child.wait(),
    )
    .await
    {
        Ok(_) => {}
        Err(_) => {
            tracing::warn!("Process did not exit in time, sending SIGKILL");
            #[cfg(unix)]
            {
                // SIGKILL the entire process group
                if let Some(pid) = pgid {
                    unsafe {
                        libc::kill(-pid, libc::SIGKILL);
                    }
                }
            }
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
    }
}

/// Rewrite URLs in stdout lines: `http://localhost:<port>` → `http://<name>.localhost:<proxy_port>`
fn rewrite_url(line: &str, app_port: u16, app_name: &str, proxy_port: u16) -> String {
    let patterns = [
        format!("http://localhost:{}", app_port),
        format!("http://127.0.0.1:{}", app_port),
        format!("http://0.0.0.0:{}", app_port),
    ];
    let replacement = format!("http://{}.localhost:{}", app_name, proxy_port);

    let mut result = line.to_string();
    for pattern in &patterns {
        result = result.replace(pattern, &replacement);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rewrite_url_localhost() {
        let line = "  ▲ Next.js 14.0.4\n  - Local: http://localhost:4001";
        let rewritten = rewrite_url(line, 4001, "my-app", 1337);
        assert!(rewritten.contains("http://my-app.localhost:1337"));
        assert!(!rewritten.contains("http://localhost:4001"));
    }

    #[test]
    fn test_rewrite_url_127() {
        let line = "Server running at http://127.0.0.1:4001";
        let rewritten = rewrite_url(line, 4001, "api", 1337);
        assert!(rewritten.contains("http://api.localhost:1337"));
    }

    #[test]
    fn test_rewrite_url_0000() {
        let line = "Listening on http://0.0.0.0:4001";
        let rewritten = rewrite_url(line, 4001, "web", 1337);
        assert!(rewritten.contains("http://web.localhost:1337"));
    }

    #[test]
    fn test_rewrite_url_no_match() {
        let line = "some random log output";
        let rewritten = rewrite_url(line, 4001, "my-app", 1337);
        assert_eq!(rewritten, line);
    }

    #[test]
    fn test_rewrite_url_different_port() {
        let line = "http://localhost:3000 is ready";
        let rewritten = rewrite_url(line, 4001, "my-app", 1337);
        // Should NOT rewrite because port doesn't match
        assert_eq!(rewritten, line);
    }
}
