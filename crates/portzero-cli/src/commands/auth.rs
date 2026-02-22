//! CLI commands: `portzero login` / `portzero logout` / `portzero whoami`
//!
//! Manages authentication with the LocalUp tunnel relay service.
//! Credentials are stored in `~/.portzero/credentials.json`.

use anyhow::Result;
use portzero_core::credentials::{self, Credentials, LocalUpApi, DEFAULT_RELAY_API};
use std::io::{self, Write};
use std::path::Path;

/// Log in to the LocalUp tunnel service.
///
/// Prompts for email and password, authenticates against the relay API,
/// and stores the auth token for tunnel use.
pub async fn login(state_dir: &Path, relay_api: Option<&str>) -> Result<()> {
    let api_url = relay_api.unwrap_or(DEFAULT_RELAY_API);

    // Check if already logged in
    if let Some(creds) = credentials::load_credentials(state_dir)? {
        if let Some(ref email) = creds.email {
            println!("Already logged in as {}.", email);
            print!("Log in with a different account? [y/N] ");
            io::stdout().flush()?;
            let mut answer = String::new();
            io::stdin().read_line(&mut answer)?;
            if !answer.trim().eq_ignore_ascii_case("y") {
                return Ok(());
            }
        }
    }

    // Prompt for credentials
    print!("Email: ");
    io::stdout().flush()?;
    let mut email = String::new();
    io::stdin().read_line(&mut email)?;
    let email = email.trim().to_string();

    if email.is_empty() {
        anyhow::bail!("Email is required");
    }

    // Read password without echo
    let password = rpassword::prompt_password("Password: ")?;

    if password.is_empty() {
        anyhow::bail!("Password is required");
    }

    println!("Logging in...");

    let api = LocalUpApi::new(api_url);

    // Try login
    let login_result = api.login(&email, &password).await;

    match login_result {
        Ok(response) => {
            let session_token = response.token;

            // Try to get an existing auth token or create one
            let auth_token = get_or_create_auth_token(&api, &session_token).await?;

            // Save credentials
            let creds = Credentials {
                relay_api: api_url.to_string(),
                session_token: Some(session_token),
                auth_token,
                email: Some(response.user.email.clone()),
                user_id: Some(response.user.id.clone()),
            };

            credentials::save_credentials(state_dir, &creds)?;

            println!();
            println!("Logged in as {}.", response.user.email);
            if let Some(ref name) = response.user.full_name {
                println!("Name: {}", name);
            }
            println!();
            println!("You can now share apps with: portzero share start <app>");
        }
        Err(e) => {
            let err_msg = format!("{}", e);
            if err_msg.contains("not found") || err_msg.contains("No account") {
                // Account doesn't exist, offer to register
                println!();
                println!("No account found for {}.", email);
                print!("Create a new account? [Y/n] ");
                io::stdout().flush()?;
                let mut answer = String::new();
                io::stdin().read_line(&mut answer)?;
                let answer = answer.trim();

                if answer.is_empty() || answer.eq_ignore_ascii_case("y") {
                    return register(state_dir, &email, &password, api_url).await;
                } else {
                    return Ok(());
                }
            }

            anyhow::bail!("Login failed: {}", e);
        }
    }

    Ok(())
}

/// Register a new account.
async fn register(state_dir: &Path, email: &str, password: &str, api_url: &str) -> Result<()> {
    print!("Full name (optional): ");
    io::stdout().flush()?;
    let mut name = String::new();
    io::stdin().read_line(&mut name)?;
    let name = name.trim();
    let full_name = if name.is_empty() { None } else { Some(name) };

    println!("Creating account...");

    let api = LocalUpApi::new(api_url);
    let response = api.register(email, password, full_name).await?;

    // The registration response includes an auto-generated auth_token
    let auth_token = if response.auth_token.is_empty() {
        // Fallback: create one via the session token
        get_or_create_auth_token(&api, &response.token).await?
    } else {
        response.auth_token
    };

    let creds = Credentials {
        relay_api: api_url.to_string(),
        session_token: Some(response.token),
        auth_token,
        email: Some(response.user.email.clone()),
        user_id: Some(response.user.id.clone()),
    };

    credentials::save_credentials(state_dir, &creds)?;

    println!();
    println!("Account created and logged in as {}.", response.user.email);
    println!();
    println!("You can now share apps with: portzero share start <app>");

    Ok(())
}

/// Get an existing auth token or create a new one.
async fn get_or_create_auth_token(api: &LocalUpApi, session_token: &str) -> Result<String> {
    // First try listing existing tokens — if there's a "portzero" one, we can't
    // retrieve the JWT value (it's only shown on creation). So just create a new one.
    let token_info = api.create_auth_token(session_token, "portzero-cli").await?;

    Ok(token_info.token)
}

/// Log out — delete stored credentials.
pub async fn logout(state_dir: &Path) -> Result<()> {
    if let Some(creds) = credentials::load_credentials(state_dir)? {
        credentials::delete_credentials(state_dir)?;
        if let Some(ref email) = creds.email {
            println!("Logged out from {}.", email);
        } else {
            println!("Logged out.");
        }
    } else {
        println!("Not logged in.");
    }
    Ok(())
}

/// Show the currently logged-in user.
pub async fn whoami(state_dir: &Path) -> Result<()> {
    match credentials::load_credentials(state_dir)? {
        Some(creds) => {
            if let Some(ref email) = creds.email {
                println!("{}", email);
            } else {
                println!("Logged in (no email on file).");
            }
            println!("Relay: {}", creds.relay_api);
        }
        None => {
            println!("Not logged in.");
            println!("Run: portzero login");
        }
    }
    Ok(())
}
