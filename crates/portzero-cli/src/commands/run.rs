//! CLI command: `portzero [name] <command>`
//!
//! Spawns the child process locally, registers it with the running daemon
//! (which owns the proxy on port 1337), and streams child output.
//!
//! If no daemon is running, auto-starts one in the background first.

use anyhow::Result;
use portzero_core::certs;
use portzero_core::control::ControlClient;
use portzero_core::types::DEFAULT_PROXY_PORT;
use std::path::Path;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

/// Run a single app: spawn the process, register with daemon, stream output.
pub async fn run(
    name: String,
    command: Vec<String>,
    _auto_restart: bool,
    state_dir: &Path,
) -> Result<()> {
    if command.is_empty() {
        anyhow::bail!(
            "No command specified.\n\
             Usage: portzero <command>          (name inferred from cwd)\n\
             Usage: portzero <name> <command>   (explicit name)"
        );
    }

    // Ensure certs
    let generated = certs::ensure_certs(state_dir)?;
    if generated {
        eprintln!("Generated TLS certificates. Trust them with: portzero trust");
    }

    let proxy_port = DEFAULT_PROXY_PORT;
    let cwd = std::env::current_dir()?;

    // Connect to daemon (auto-start if not running)
    let mut client = ensure_daemon(state_dir).await?;

    // Allocate a port from the daemon's router
    let port = client.allocate_port(&name).await?;

    // Build the child command
    let (program, args) = if command.len() == 1 {
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
        .current_dir(&cwd)
        .env("PORT", port.to_string())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);

    // Create a new process group so we can kill the whole tree
    #[cfg(unix)]
    unsafe {
        cmd.pre_exec(|| {
            libc::setpgid(0, 0);
            Ok(())
        });
    }

    let mut child = cmd.spawn()?;
    let pid = child
        .id()
        .ok_or_else(|| anyhow::anyhow!("child exited immediately"))?;

    // Register with the daemon
    client.register(&name, port, pid, &command, &cwd).await?;

    println!();
    println!("  App:   {}", name);
    println!("  URL:   http://{}.localhost:{}", name, proxy_port);
    println!("  Port:  {} (assigned)", port);
    println!("  PID:   {}", pid);
    println!("  Cmd:   {}", command.join(" "));
    println!();

    // Set up Ctrl+C handler to kill child + deregister
    let child_pid = pid;
    let deregister_name = name.clone();
    let deregister_state_dir = state_dir.to_path_buf();
    ctrlc::set_handler(move || {
        eprintln!("\nShutting down...");

        #[cfg(unix)]
        {
            unsafe {
                libc::kill(-(child_pid as i32), libc::SIGTERM);
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
            unsafe {
                libc::kill(-(child_pid as i32), libc::SIGKILL);
            }
        }

        // Deregister from daemon (best-effort, synchronous)
        // We create a small runtime just for this one call
        if let Ok(rt) = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            rt.block_on(async {
                if let Some(mut c) = ControlClient::connect(&deregister_state_dir).await {
                    let _ = c.deregister(&deregister_name).await;
                }
            });
        }

        std::process::exit(0);
    })?;

    // Stream stdout and stderr, forwarding to daemon for log storage
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let name_out = name.clone();
    let state_dir_out = state_dir.to_path_buf();
    let stdout_task = tokio::spawn(async move {
        if let Some(stdout) = stdout {
            let mut log_client = ControlClient::connect(&state_dir_out).await;
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let line = rewrite_url(&line, port, &name_out, proxy_port);
                println!("[{}] {}", name_out, line);
                // Forward to daemon with reconnect on failure
                if let Some(ref mut c) = log_client {
                    if c.log_append(&name_out, "stdout", &line).await.is_err() {
                        log_client = ControlClient::connect(&state_dir_out).await;
                        if let Some(ref mut c) = log_client {
                            let _ = c.log_append(&name_out, "stdout", &line).await;
                        }
                    }
                } else {
                    log_client = ControlClient::connect(&state_dir_out).await;
                    if let Some(ref mut c) = log_client {
                        let _ = c.log_append(&name_out, "stdout", &line).await;
                    }
                }
            }
        }
    });

    let name_err = name.clone();
    let state_dir_err = state_dir.to_path_buf();
    let stderr_task = tokio::spawn(async move {
        if let Some(stderr) = stderr {
            let mut log_client = ControlClient::connect(&state_dir_err).await;
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let line = rewrite_url(&line, port, &name_err, proxy_port);
                eprintln!("[{}] {}", name_err, line);
                // Forward to daemon with reconnect on failure
                if let Some(ref mut c) = log_client {
                    if c.log_append(&name_err, "stderr", &line).await.is_err() {
                        log_client = ControlClient::connect(&state_dir_err).await;
                        if let Some(ref mut c) = log_client {
                            let _ = c.log_append(&name_err, "stderr", &line).await;
                        }
                    }
                } else {
                    log_client = ControlClient::connect(&state_dir_err).await;
                    if let Some(ref mut c) = log_client {
                        let _ = c.log_append(&name_err, "stderr", &line).await;
                    }
                }
            }
        }
    });

    // Wait for child to exit
    let exit_status = child.wait().await?;

    let _ = stdout_task.await;
    let _ = stderr_task.await;

    // Deregister from daemon
    if let Some(mut c) = ControlClient::connect(state_dir).await {
        let _ = c.deregister(&name).await;
    }

    let code = exit_status.code().unwrap_or(1);
    if code != 0 {
        eprintln!("[{}] exited with code {}", name, code);
    }

    std::process::exit(code);
}

/// Ensure the daemon is running. If not, start it in the background.
/// Public alias for use from other commands (e.g. `up`).
pub async fn ensure_daemon_public(state_dir: &Path) -> Result<ControlClient> {
    ensure_daemon(state_dir).await
}

async fn ensure_daemon(state_dir: &Path) -> Result<ControlClient> {
    // Try connecting first
    if let Some(mut client) = ControlClient::connect(state_dir).await {
        if client.ping().await {
            return Ok(client);
        }
    }

    // Daemon not running — start it in the background
    eprintln!("Starting PortZero daemon...");

    let exe = std::env::current_exe()?;
    let mut daemon_cmd = std::process::Command::new(&exe);
    daemon_cmd.arg("start");

    // Detach the daemon process
    daemon_cmd
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        unsafe {
            daemon_cmd.pre_exec(|| {
                // Create new session so the daemon isn't killed with the CLI
                libc::setsid();
                Ok(())
            });
        }
    }

    daemon_cmd.spawn()?;

    // Wait for the daemon to start (poll the control socket)
    for i in 0..50 {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        if let Some(mut client) = ControlClient::connect(state_dir).await {
            if client.ping().await {
                eprintln!("Daemon started.");
                return Ok(client);
            }
        }
        if i == 10 {
            eprintln!("Waiting for daemon to start...");
        }
    }

    anyhow::bail!(
        "Failed to start daemon after 5 seconds.\n\
         Try starting it manually: portzero start"
    )
}

/// Rewrite localhost URLs in child output.
pub fn rewrite_url(line: &str, app_port: u16, app_name: &str, proxy_port: u16) -> String {
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
    use super::rewrite_url;

    #[test]
    fn test_rewrite_localhost() {
        let result = rewrite_url(
            "Server running at http://localhost:3000",
            3000,
            "my-app",
            1337,
        );
        assert_eq!(result, "Server running at http://my-app.localhost:1337");
    }

    #[test]
    fn test_rewrite_127() {
        let result = rewrite_url("Listening on http://127.0.0.1:4000", 4000, "api", 1337);
        assert_eq!(result, "Listening on http://api.localhost:1337");
    }

    #[test]
    fn test_rewrite_0000() {
        let result = rewrite_url("Ready at http://0.0.0.0:5000/", 5000, "web", 1337);
        assert_eq!(result, "Ready at http://web.localhost:1337/");
    }

    #[test]
    fn test_rewrite_no_match() {
        let result = rewrite_url("Some random log line", 3000, "app", 1337);
        assert_eq!(result, "Some random log line");
    }

    #[test]
    fn test_rewrite_different_port_no_match() {
        // Port in output doesn't match app_port — no rewrite
        let result = rewrite_url("http://localhost:9999", 3000, "app", 1337);
        assert_eq!(result, "http://localhost:9999");
    }

    #[test]
    fn test_rewrite_multiple_occurrences() {
        let result = rewrite_url(
            "http://localhost:3000 and also http://localhost:3000/api",
            3000,
            "app",
            1337,
        );
        assert_eq!(
            result,
            "http://app.localhost:1337 and also http://app.localhost:1337/api"
        );
    }
}
