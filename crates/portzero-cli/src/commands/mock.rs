//! CLI command: `portzero mock`
//!
//! Manage response mock rules for apps.
//!
//! # Usage
//!
//! ```sh
//! portzero mock my-app POST /api/payments --status 500 --body '{"error":"declined"}'
//! portzero mock my-app GET "/api/users/*" --status 200 --body-file ./fixtures/users.json
//! portzero mock list
//! portzero mock disable <id>
//! portzero mock delete <id>
//! ```

use clap::Subcommand;
use portzero_core::mock_engine::MockEngine;
use portzero_core::types::MockRule;
use std::collections::HashMap;
use std::sync::Arc;

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

/// Execute a mock command.
pub fn execute(cmd: &MockCommand, mock_engine: &Arc<MockEngine>) -> anyhow::Result<()> {
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

            // If body looks like JSON and no Content-Type header is set, add it
            if !response_body.is_empty()
                && (response_body.starts_with('{') || response_body.starts_with('['))
                && !response_headers.contains_key("Content-Type")
            {
                response_headers.insert("Content-Type".to_string(), "application/json".to_string());
            }

            let rule = mock_engine.add_mock(
                app.clone(),
                Some(method.to_uppercase()),
                path.clone(),
                *status,
                response_headers,
                response_body,
            );

            println!("Mock created:");
            println!("  ID:      {}", rule.id);
            println!("  App:     {}", rule.app_name);
            println!(
                "  Match:   {} {}",
                rule.method.as_deref().unwrap_or("*"),
                rule.path_pattern
            );
            println!("  Status:  {}", rule.status_code);
            println!("  Enabled: true");
        }

        MockCommand::List { app } => {
            let mocks = match app {
                Some(app_name) => mock_engine.list_mocks_for_app(app_name),
                None => mock_engine.list_mocks(),
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
            match mock_engine.update_mock(id, None, None, None, None, None, Some(true)) {
                Some(_) => println!("Mock '{}' enabled.", id),
                None => anyhow::bail!("Mock '{}' not found.", id),
            }
        }

        MockCommand::Disable { id } => {
            match mock_engine.update_mock(id, None, None, None, None, None, Some(false)) {
                Some(_) => println!("Mock '{}' disabled.", id),
                None => anyhow::bail!("Mock '{}' not found.", id),
            }
        }

        MockCommand::Delete { id } => {
            if mock_engine.remove_mock(id) {
                println!("Mock '{}' deleted.", id);
            } else {
                anyhow::bail!("Mock '{}' not found.", id);
            }
        }
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
