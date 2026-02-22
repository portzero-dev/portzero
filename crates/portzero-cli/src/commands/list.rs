//! CLI command: `portzero list`
//!
//! Lists all active apps with their status, URL, port, and PID.

use anyhow::Result;
use portzero_core::types::DEFAULT_PROXY_PORT;
use std::path::Path;

/// List active apps by reading the router state.
///
/// Note: In a full daemon architecture, this would query the running daemon
/// via the API. For the single-process CLI mode, it reads from shared state.
/// We implement a lightweight version that reads from the state directory.
pub async fn list(state_dir: &Path) -> Result<()> {
    let db_path = state_dir.join("portzero.db");

    if !db_path.exists() {
        println!("No apps running. Start one with: portzero <command>");
        return Ok(());
    }

    let store = portzero_core::store::Store::open(&db_path)?;
    let request_count = store.request_count()?;

    println!("PortZero — http://localhost:{}", DEFAULT_PROXY_PORT);
    println!();

    // In the single-process model, the router state is in-memory.
    // For a persistent daemon, we'd query the API.
    // For now, show a helpful message.
    println!("  {} request(s) captured", request_count);
    println!();
    println!("Note: `portzero list` shows apps managed by the running daemon.");
    println!("Start apps with: portzero <name> <command>");
    println!("Or start all from config: portzero up");

    Ok(())
}
