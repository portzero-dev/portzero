//! CLI command: `portzero up` / `portzero down`
//!
//! Reads `portzero.toml` and starts all defined apps via the daemon.
//! Stays alive streaming logs from all apps (like `docker-compose up`).
//! Ctrl+C stops all child processes and deregisters them from the daemon.

use anyhow::Result;
use portzero_core::certs;
use portzero_core::config::Config;
use portzero_core::control::ControlClient;
use std::path::Path;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::Mutex;

use super::run::rewrite_url;

/// A running child process tracked by `up`.
struct ChildApp {
    name: String,
    pid: u32,
}

/// Start all apps defined in `portzero.toml`.
///
/// Stays alive streaming logs. Ctrl+C kills all children and deregisters.
pub async fn up(state_dir: &Path) -> Result<()> {
    // Load config
    let config_path = std::env::current_dir()?.join("portzero.toml");
    if !config_path.exists() {
        anyhow::bail!(
            "No portzero.toml found in current directory.\n\
             Create one with app definitions, or use: portzero <name> <command>"
        );
    }

    let config = Config::load(&config_path)?;

    // Ensure certs
    let generated = certs::ensure_certs(state_dir)?;
    if generated {
        eprintln!("Generated TLS certificates. Trust them with: portzero trust");
    }

    if config.apps.is_empty() {
        anyhow::bail!("No apps defined in portzero.toml. Add [apps.<name>] sections.");
    }

    // Ensure daemon is running
    let mut client = crate::commands::run::ensure_daemon_public(state_dir).await?;

    println!(
        "Starting {} app(s) from portzero.toml...",
        config.apps.len()
    );
    println!();

    let proxy_port = config.proxy.port;

    // Track children for cleanup
    let children: Arc<Mutex<Vec<ChildApp>>> = Arc::new(Mutex::new(Vec::new()));
    let mut log_tasks = Vec::new();

    for (name, app_config) in &config.apps {
        let command: Vec<String> = app_config
            .command
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        let cwd = if let Some(ref dir) = app_config.cwd {
            std::env::current_dir()?.join(dir)
        } else {
            std::env::current_dir()?
        };

        let extra_env: Vec<(String, String)> = app_config
            .env
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        // Allocate port from daemon
        let port = client.allocate_port(name).await?;

        // Spawn child process with piped stdout/stderr for log capture
        let mut cmd = tokio::process::Command::new(&command[0]);
        if command.len() > 1 {
            cmd.args(&command[1..]);
        }
        cmd.current_dir(&cwd)
            .env("PORT", port.to_string())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);

        for (k, v) in &extra_env {
            cmd.env(k, v);
        }

        #[cfg(unix)]
        unsafe {
            cmd.pre_exec(|| {
                libc::setpgid(0, 0);
                Ok(())
            });
        }

        match cmd.spawn() {
            Ok(mut child) => {
                let pid = child.id().unwrap_or(0);

                // Register with daemon
                if let Err(e) = client.register(name, port, pid, &command, &cwd).await {
                    eprintln!("  {} → FAILED to register: {}", name, e);
                    continue;
                }

                println!(
                    "  {} → http://{}.localhost:{} (port {}, PID {})",
                    name, name, proxy_port, port, pid
                );

                // Track child for cleanup
                children.lock().await.push(ChildApp {
                    name: name.clone(),
                    pid,
                });

                // Spawn log forwarding tasks for stdout
                let stdout = child.stdout.take();
                let name_out = name.clone();
                let state_dir_out = state_dir.to_path_buf();
                let stdout_task = tokio::spawn(async move {
                    if let Some(stdout) = stdout {
                        let mut log_client = ControlClient::connect(&state_dir_out).await;
                        let mut lines = BufReader::new(stdout).lines();
                        while let Ok(Some(line)) = lines.next_line().await {
                            let line = rewrite_url(&line, port, &name_out, proxy_port);
                            println!("[{}] {}", name_out, line);
                            if let Some(ref mut c) = log_client {
                                if c.log_append(&name_out, "stdout", &line).await.is_err() {
                                    // Reconnect on failure
                                    log_client = ControlClient::connect(&state_dir_out).await;
                                    if let Some(ref mut c) = log_client {
                                        let _ = c.log_append(&name_out, "stdout", &line).await;
                                    }
                                }
                            } else {
                                // Retry initial connection
                                log_client = ControlClient::connect(&state_dir_out).await;
                                if let Some(ref mut c) = log_client {
                                    let _ = c.log_append(&name_out, "stdout", &line).await;
                                }
                            }
                        }
                    }
                });

                // Spawn log forwarding tasks for stderr
                let stderr = child.stderr.take();
                let name_err = name.clone();
                let state_dir_err = state_dir.to_path_buf();
                let stderr_task = tokio::spawn(async move {
                    if let Some(stderr) = stderr {
                        let mut log_client = ControlClient::connect(&state_dir_err).await;
                        let mut lines = BufReader::new(stderr).lines();
                        while let Ok(Some(line)) = lines.next_line().await {
                            let line = rewrite_url(&line, port, &name_err, proxy_port);
                            eprintln!("[{}] {}", name_err, line);
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

                // Spawn a task to wait on the child (so we detect exits)
                let name_wait = name.clone();
                let wait_task = tokio::spawn(async move {
                    match child.wait().await {
                        Ok(status) => {
                            let code = status.code().unwrap_or(-1);
                            if code != 0 {
                                eprintln!("[{}] exited with code {}", name_wait, code);
                            } else {
                                eprintln!("[{}] exited", name_wait);
                            }
                        }
                        Err(e) => {
                            eprintln!("[{}] error waiting on process: {}", name_wait, e);
                        }
                    }
                });

                log_tasks.push(stdout_task);
                log_tasks.push(stderr_task);
                log_tasks.push(wait_task);
            }
            Err(e) => {
                eprintln!("  {} → FAILED: {}", name, e);
            }
        }
    }

    println!();
    println!("All apps started. Proxy at http://localhost:{}", proxy_port);
    println!("Press Ctrl+C to stop all apps.");
    println!();

    // Set up Ctrl+C handler to kill all children + deregister
    let children_ref = children.clone();
    let state_dir_cleanup = state_dir.to_path_buf();
    ctrlc::set_handler(move || {
        eprintln!("\nShutting down all apps...");

        // Kill all child process groups
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build();

        if let Ok(rt) = rt {
            rt.block_on(async {
                let apps = children_ref.lock().await;
                if let Some(mut client) = ControlClient::connect(&state_dir_cleanup).await {
                    for app in apps.iter() {
                        #[cfg(unix)]
                        {
                            unsafe {
                                libc::kill(-(app.pid as i32), libc::SIGTERM);
                            }
                        }
                        let _ = client.deregister(&app.name).await;
                    }
                } else {
                    // Can't reach daemon, just kill processes
                    for app in apps.iter() {
                        #[cfg(unix)]
                        {
                            unsafe {
                                libc::kill(-(app.pid as i32), libc::SIGTERM);
                            }
                        }
                    }
                }

                // Give processes a moment to exit gracefully
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;

                // Force kill any remaining
                for app in apps.iter() {
                    #[cfg(unix)]
                    {
                        unsafe {
                            libc::kill(-(app.pid as i32), libc::SIGKILL);
                        }
                    }
                }
            });
        }

        eprintln!("All apps stopped.");
        std::process::exit(0);
    })?;

    // Wait for all log tasks to finish (they finish when children exit)
    for task in log_tasks {
        let _ = task.await;
    }

    // All children exited — deregister remaining
    let apps = children.lock().await;
    if let Some(mut client) = ControlClient::connect(state_dir).await {
        for app in apps.iter() {
            let _ = client.deregister(&app.name).await;
        }
    }

    println!("All apps have exited.");
    Ok(())
}

/// Stop all running apps (stops the daemon).
pub async fn down(state_dir: &Path) -> Result<()> {
    println!("Stopping daemon and all apps...");
    crate::daemon::stop(state_dir).await?;
    println!("Done.");
    Ok(())
}
