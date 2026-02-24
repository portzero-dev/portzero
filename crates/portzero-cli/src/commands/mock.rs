//! CLI command: `portzero mock`
//!
//! Manage response mock rules for apps via the daemon's control socket.
//!
//! # Usage
//!
//! ```sh
//! portzero mock add my-app POST /api/payments --status 500 --body '{"error":"declined"}'
//! portzero mock add my-app GET "/api/users/*" --status 200 --body-file ./fixtures/users.json
//! portzero mock list
//! portzero mock enable <id>
//! portzero mock disable <id>
//! portzero mock delete <id>
//! ```

use anyhow::Result;
use clap::Subcommand;
use portzero_core::control::ControlClient;
use portzero_core::types::CreateMockRule;
use std::collections::HashMap;
use std::path::Path;

/// Mock subcommands.
#[derive(Debug, Subcommand)]
pub enum MockCommand {
    /// Create a new mock rule: portzero mock add <app> <method> <path> [options]
    Add {
        /// App name
        app: String,
        /// HTTP method to match (GET, POST, PUT, DELETE, etc.)
        method: String,
        /// Path pattern to match (supports * and ** wildcards)
        path: String,
        /// Response status code
        #[arg(long, default_value = "200")]
        status: u16,
        /// Response body (inline JSON string)
        #[arg(long, default_value = "")]
        body: String,
        /// Response body from file
        #[arg(long)]
        body_file: Option<String>,
        /// Response header (can be specified multiple times, format: "Key: Value")
        #[arg(long = "header", short = 'H')]
        headers: Vec<String>,
    },
    /// List all mock rules
    List {
        /// Filter by app name
        #[arg(long)]
        app: Option<String>,
    },
    /// Enable a mock rule
    Enable {
        /// Mock rule ID
        id: String,
    },
    /// Disable a mock rule
    Disable {
        /// Mock rule ID
        id: String,
    },
    /// Delete a mock rule
    Delete {
        /// Mock rule ID
        id: String,
    },
}

/// Execute a mock command via the daemon's control socket.
pub async fn execute_via_daemon(cmd: &MockCommand, state_dir: &Path) -> Result<()> {
    let Some(mut client) = ControlClient::connect(state_dir).await else {
        anyhow::bail!(
            "Cannot connect to daemon. Is it running?\n\
             Start it with: portzero start"
        );
    };

    match cmd {
        MockCommand::Add {
            app,
            method,
            path,
            status,
            body,
            body_file,
            headers,
        } => {
            // Resolve body from file if specified
            let response_body = if let Some(file_path) = body_file {
                std::fs::read_to_string(file_path).map_err(|e| {
                    anyhow::anyhow!("Failed to read body file '{}': {}", file_path, e)
                })?
            } else {
                body.clone()
            };

            // Parse headers
            let mut response_headers = HashMap::new();
            for h in headers {
                let parts: Vec<&str> = h.splitn(2, ':').collect();
                if parts.len() == 2 {
                    response_headers
                        .insert(parts[0].trim().to_string(), parts[1].trim().to_string());
                } else {
                    anyhow::bail!("Invalid header format: '{}'. Use 'Key: Value'", h);
                }
            }

            // Auto-add Content-Type for JSON bodies
            if !response_body.is_empty()
                && (response_body.starts_with('{') || response_body.starts_with('['))
                && !response_headers.contains_key("Content-Type")
            {
                response_headers.insert("Content-Type".to_string(), "application/json".to_string());
            }

            let rule = CreateMockRule {
                app_name: app.clone(),
                method: Some(method.to_uppercase()),
                path_pattern: path.clone(),
                status_code: *status,
                response_headers,
                response_body,
                enabled: true,
            };

            match client.create_mock(rule).await {
                Ok(mock) => {
                    println!("Mock created:");
                    println!("  ID:      {}", mock.id);
                    println!("  App:     {}", mock.app_name);
                    println!(
                        "  Match:   {} {}",
                        mock.method.as_deref().unwrap_or("*"),
                        mock.path_pattern
                    );
                    println!("  Status:  {}", mock.status_code);
                    println!("  Enabled: true");
                }
                Err(e) => anyhow::bail!("Failed to create mock: {e}"),
            }
        }

        MockCommand::List { app } => {
            let mocks = client.list_mocks().await?;

            let mocks: Vec<_> = match app {
                Some(app_name) => mocks
                    .into_iter()
                    .filter(|m| &m.app_name == app_name)
                    .collect(),
                None => mocks,
            };

            if mocks.is_empty() {
                println!("No mock rules configured.");
                return Ok(());
            }

            println!(
                "{:<36}  {:<10}  {:<6}  {:<20}  {:<6}  {:<5}  {}",
                "ID", "App", "Method", "Path", "Status", "Hits", "Enabled"
            );
            println!("{}", "-".repeat(100));

            for mock in &mocks {
                println!(
                    "{:<36}  {:<10}  {:<6}  {:<20}  {:<6}  {:<5}  {}",
                    mock.id,
                    mock.app_name,
                    mock.method.as_deref().unwrap_or("*"),
                    truncate(&mock.path_pattern, 20),
                    mock.status_code,
                    mock.hit_count,
                    if mock.enabled { "yes" } else { "no" },
                );
            }
        }

        MockCommand::Enable { id } => {
            let updates = portzero_core::types::UpdateMockRule {
                method: None,
                path_pattern: None,
                status_code: None,
                response_headers: None,
                response_body: None,
                enabled: Some(true),
            };
            match client.update_mock(id, updates).await {
                Ok(_) => println!("Mock '{}' enabled.", id),
                Err(e) => anyhow::bail!("Failed to enable mock '{}': {e}", id),
            }
        }

        MockCommand::Disable { id } => {
            let updates = portzero_core::types::UpdateMockRule {
                method: None,
                path_pattern: None,
                status_code: None,
                response_headers: None,
                response_body: None,
                enabled: Some(false),
            };
            match client.update_mock(id, updates).await {
                Ok(_) => println!("Mock '{}' disabled.", id),
                Err(e) => anyhow::bail!("Failed to disable mock '{}': {e}", id),
            }
        }

        MockCommand::Delete { id } => match client.delete_mock(id).await {
            Ok(()) => println!("Mock '{}' deleted.", id),
            Err(e) => anyhow::bail!("Failed to delete mock '{}': {e}", id),
        },
    }
    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max - 3])
    }
}
