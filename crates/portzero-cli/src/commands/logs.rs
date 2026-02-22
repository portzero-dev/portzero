//! CLI command: `portzero logs <name>`
//!
//! Tail process logs for a managed app via the daemon's LogStore.
//! Shows buffered log lines and optionally follows new output in
//! real-time via the daemon's event subscription.

use anyhow::Result;
use portzero_core::control::ControlClient;
use portzero_core::types::LogStream;
use std::path::Path;

/// Show logs for a managed app.
///
/// Queries the daemon's in-memory LogStore for buffered lines, then
/// optionally subscribes to the event stream for real-time following.
pub async fn logs(name: &str, lines: usize, follow: bool, state_dir: &Path) -> Result<()> {
    // Connect to daemon
    let mut client = match ControlClient::connect(state_dir).await {
        Some(c) => c,
        None => {
            eprintln!("Cannot connect to daemon. Is it running?");
            eprintln!("Start it with: portzero start");
            return Ok(());
        }
    };

    // Fetch buffered log lines
    let log_lines = client.get_logs(name, Some(lines)).await?;

    if log_lines.is_empty() && !follow {
        println!("No log output for '{}' yet.", name);
        println!();
        println!("Logs are captured when apps are started with:");
        println!("  portzero <name> <command>");
        println!("  portzero up");
        return Ok(());
    }

    // Print buffered lines
    for log_line in &log_lines {
        print_log_line(name, &log_line.stream, &log_line.content);
    }

    if !follow {
        return Ok(());
    }

    // Follow mode: subscribe to daemon events and print new log lines
    if !log_lines.is_empty() {
        println!();
    }
    eprintln!("Following logs for '{}'... (Ctrl+C to stop)", name);

    // We need a fresh connection for subscribe (it takes ownership)
    let sub_client = match ControlClient::connect(state_dir).await {
        Some(c) => c,
        None => {
            eprintln!("Lost connection to daemon.");
            return Ok(());
        }
    };

    let mut subscription = sub_client.subscribe().await?;

    loop {
        match subscription.next_event().await {
            Some(portzero_core::types::WsEvent::LogLine {
                app, stream, line, ..
            }) if app == name => {
                print_log_line(name, &stream, &line);
            }
            Some(_) => {
                // Ignore events for other apps or other event types
            }
            None => {
                // Connection closed
                eprintln!("Connection to daemon lost.");
                break;
            }
        }
    }

    Ok(())
}

/// Print a log line with stream-appropriate formatting.
fn print_log_line(name: &str, stream: &LogStream, content: &str) {
    match stream {
        LogStream::Stdout => println!("[{}] {}", name, content),
        LogStream::Stderr => eprintln!("[{}] {}", name, content),
    }
}
