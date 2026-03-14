//! TLS certificate generation and management.
//!
//! Generates a local CA and per-hostname certificates for HTTPS support.
//! Uses rcgen for certificate generation and rustls for TLS contexts.

use anyhow::Result;
use rcgen::{BasicConstraints, CertificateParams, DnType, IsCa, KeyPair, KeyUsagePurpose, SanType};
use std::fs;
use std::path::{Path, PathBuf};

/// Paths to certificate files.
pub struct CertPaths {
    pub ca_key: PathBuf,
    pub ca_cert: PathBuf,
    pub server_key: PathBuf,
    pub server_cert: PathBuf,
}

impl CertPaths {
    pub fn new(state_dir: &Path) -> Self {
        let certs_dir = state_dir.join("certs");
        Self {
            ca_key: certs_dir.join("ca.key"),
            ca_cert: certs_dir.join("ca.crt"),
            server_key: certs_dir.join("server.key"),
            server_cert: certs_dir.join("server.crt"),
        }
    }

    pub fn all_exist(&self) -> bool {
        self.ca_key.exists()
            && self.ca_cert.exists()
            && self.server_key.exists()
            && self.server_cert.exists()
    }
}

/// Generated certificate data (PEM-encoded).
pub struct GeneratedCerts {
    pub ca_cert_pem: String,
    pub ca_key_pem: String,
    pub server_cert_pem: String,
    pub server_key_pem: String,
}

/// Generate a local CA and server certificate for *.localhost.
pub fn generate_localhost_certs() -> Result<GeneratedCerts> {
    // Generate CA key pair
    let ca_key_pair = KeyPair::generate()?;

    // CA certificate parameters
    let mut ca_params = CertificateParams::default();
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    ca_params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
    ca_params
        .distinguished_name
        .push(DnType::CommonName, "PortZero Local CA");
    ca_params
        .distinguished_name
        .push(DnType::OrganizationName, "PortZero");

    let ca_cert = ca_params.self_signed(&ca_key_pair)?;

    // Server key pair
    let server_key_pair = KeyPair::generate()?;

    // Server certificate parameters
    let mut server_params = CertificateParams::default();
    server_params
        .distinguished_name
        .push(DnType::CommonName, "localhost");
    server_params.subject_alt_names = vec![
        SanType::DnsName("localhost".try_into()?),
        SanType::DnsName("*.localhost".try_into()?),
    ];

    let server_cert = server_params.signed_by(&server_key_pair, &ca_cert, &ca_key_pair)?;

    Ok(GeneratedCerts {
        ca_cert_pem: ca_cert.pem(),
        ca_key_pem: ca_key_pair.serialize_pem(),
        server_cert_pem: server_cert.pem(),
        server_key_pem: server_key_pair.serialize_pem(),
    })
}

/// Ensure certificates exist in the state directory, generating them if needed.
/// Returns true if new certs were generated.
pub fn ensure_certs(state_dir: &Path) -> Result<bool> {
    let paths = CertPaths::new(state_dir);

    if paths.all_exist() {
        return Ok(false);
    }

    let certs_dir = state_dir.join("certs");
    fs::create_dir_all(&certs_dir)?;

    let certs = generate_localhost_certs()?;

    fs::write(&paths.ca_key, &certs.ca_key_pem)?;
    fs::write(&paths.ca_cert, &certs.ca_cert_pem)?;
    fs::write(&paths.server_key, &certs.server_key_pem)?;
    fs::write(&paths.server_cert, &certs.server_cert_pem)?;

    // Restrict permissions on key files
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o600);
        fs::set_permissions(&paths.ca_key, perms.clone())?;
        fs::set_permissions(&paths.server_key, perms)?;
    }

    tracing::info!("Generated TLS certificates in {}", certs_dir.display());
    Ok(true)
}

/// Load server certificate and key from state directory.
pub fn load_server_certs(state_dir: &Path) -> Result<(Vec<u8>, Vec<u8>)> {
    let paths = CertPaths::new(state_dir);
    let cert = fs::read(&paths.server_cert)?;
    let key = fs::read(&paths.server_key)?;
    Ok((cert, key))
}

/// Load the CA certificate PEM from state directory.
pub fn load_ca_cert(state_dir: &Path) -> Result<String> {
    let paths = CertPaths::new(state_dir);
    let pem = fs::read_to_string(&paths.ca_cert)?;
    Ok(pem)
}

// ---------------------------------------------------------------------------
// System trust store integration
// ---------------------------------------------------------------------------

/// Result of a trust operation.
#[derive(Debug, Clone)]
pub enum TrustResult {
    /// CA was successfully added to the system trust store.
    Trusted,
    /// CA was already trusted.
    AlreadyTrusted,
    /// Trust operation requires elevated privileges (sudo).
    NeedsSudo,
    /// Trust operation failed.
    Failed(String),
    /// Platform not supported for automatic trust.
    Unsupported,
}

/// Check if the PortZero CA is already trusted in the system trust store.
pub fn is_ca_trusted(state_dir: &Path) -> Result<bool> {
    let paths = CertPaths::new(state_dir);
    if !paths.ca_cert.exists() {
        return Ok(false);
    }

    #[cfg(target_os = "macos")]
    {
        is_ca_trusted_macos(&paths.ca_cert)
    }

    #[cfg(target_os = "linux")]
    {
        is_ca_trusted_linux(&paths.ca_cert)
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        Ok(false)
    }
}

/// Install the PortZero CA certificate into the system trust store.
///
/// **Requires root/sudo on all platforms.**
///
/// On macOS: uses `security add-trusted-cert` to add to the System Keychain.
/// On Linux: copies to `/usr/local/share/ca-certificates/` and runs `update-ca-certificates`.
///
/// This function is designed to be called from the Tauri dashboard's "Trust Certificate"
/// button, which will prompt the user for their password via `osascript` (macOS) or
/// `pkexec` (Linux).
pub fn trust_ca(state_dir: &Path, use_sudo_prompt: bool) -> Result<TrustResult> {
    let paths = CertPaths::new(state_dir);
    if !paths.ca_cert.exists() {
        anyhow::bail!("CA certificate not found. Run `portzero` first to generate certificates.");
    }

    // Check if already trusted
    if is_ca_trusted(state_dir).unwrap_or(false) {
        return Ok(TrustResult::AlreadyTrusted);
    }

    #[cfg(target_os = "macos")]
    {
        trust_ca_macos(&paths.ca_cert, use_sudo_prompt)
    }

    #[cfg(target_os = "linux")]
    {
        trust_ca_linux(&paths.ca_cert, use_sudo_prompt)
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        Ok(TrustResult::Unsupported)
    }
}

/// Remove the PortZero CA certificate from the system trust store.
pub fn untrust_ca(state_dir: &Path, use_sudo_prompt: bool) -> Result<TrustResult> {
    let paths = CertPaths::new(state_dir);

    #[cfg(target_os = "macos")]
    {
        untrust_ca_macos(&paths.ca_cert, use_sudo_prompt)
    }

    #[cfg(target_os = "linux")]
    {
        untrust_ca_linux(use_sudo_prompt)
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        Ok(TrustResult::Unsupported)
    }
}

/// Get a shell command string that the user can run manually to trust the CA.
/// Useful for displaying in the CLI when automatic trust fails.
pub fn trust_ca_command(state_dir: &Path) -> String {
    let paths = CertPaths::new(state_dir);
    let cert_path = paths.ca_cert.display();

    #[cfg(target_os = "macos")]
    {
        format!(
            "security add-trusted-cert -r trustRoot \
             -k ~/Library/Keychains/login.keychain-db \"{}\"",
            cert_path
        )
    }

    #[cfg(target_os = "linux")]
    {
        format!(
            "sudo cp \"{}\" /usr/local/share/ca-certificates/portzero-ca.crt && \
             sudo update-ca-certificates",
            cert_path
        )
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        format!(
            "Manually import \"{}\" into your system's certificate trust store",
            cert_path
        )
    }
}

// ---------------------------------------------------------------------------
// macOS implementation
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
fn is_ca_trusted_macos(_ca_cert_path: &Path) -> Result<bool> {
    use std::process::Command;

    // Check the user's login keychain first (preferred for desktop app)
    let login_keychain = std::env::var("HOME")
        .map(|h| PathBuf::from(h).join("Library/Keychains/login.keychain-db"))
        .unwrap_or_default();

    let output = Command::new("security")
        .args(["find-certificate", "-c", "PortZero Local CA", "-a", "-Z"])
        .arg(&login_keychain)
        .output()?;

    if output.status.success() && !output.stdout.is_empty() {
        return Ok(true);
    }

    // Also check the System Keychain (may have been trusted via CLI/sudo)
    let output = Command::new("security")
        .args(["find-certificate", "-c", "PortZero Local CA", "-a", "-Z"])
        .arg("/Library/Keychains/System.keychain")
        .output()?;

    Ok(output.status.success() && !output.stdout.is_empty())
}

#[cfg(target_os = "macos")]
fn trust_ca_macos(ca_cert_path: &Path, use_sudo_prompt: bool) -> Result<TrustResult> {
    use std::process::Command;

    let cert_path_str = ca_cert_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("invalid cert path"))?;

    if use_sudo_prompt {
        // GUI path (Tauri desktop app): add to the user's login keychain.
        // This does NOT require admin privileges — avoids the
        // SecTrustSettingsSetTrustSettings authorization error that occurs
        // when a sandboxed/bundled app tries to modify the System keychain.
        let home = std::env::var("HOME")
            .map_err(|_| anyhow::anyhow!("cannot determine home directory"))?;
        let login_keychain = PathBuf::from(&home).join("Library/Keychains/login.keychain-db");

        let login_keychain_str = login_keychain
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("invalid keychain path"))?;

        // Add the certificate as trusted to the login keychain (no admin needed)
        let import_output = Command::new("security")
            .args([
                "add-trusted-cert",
                "-r",
                "trustRoot",
                "-k",
                login_keychain_str,
                cert_path_str,
            ])
            .output()?;

        if import_output.status.success() {
            tracing::info!("CA certificate trusted via macOS login keychain");
            Ok(TrustResult::Trusted)
        } else {
            let stderr = String::from_utf8_lossy(&import_output.stderr);
            let stderr_str = stderr.to_string();

            // If the login keychain approach also fails (e.g. keychain locked),
            // fall back to osascript with admin privileges for the System keychain.
            tracing::warn!(
                "Login keychain trust failed ({}), falling back to osascript",
                stderr_str.trim()
            );

            let script = format!(
                r#"do shell script "security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain '{}'" with administrator privileges"#,
                cert_path_str
            );

            let output = Command::new("osascript").args(["-e", &script]).output()?;

            if output.status.success() {
                tracing::info!("CA certificate trusted via macOS System Keychain (osascript)");
                Ok(TrustResult::Trusted)
            } else {
                let os_stderr = String::from_utf8_lossy(&output.stderr);
                if os_stderr.contains("User canceled") || os_stderr.contains("-128") {
                    Ok(TrustResult::NeedsSudo)
                } else {
                    Ok(TrustResult::Failed(os_stderr.to_string()))
                }
            }
        }
    } else {
        // Direct sudo (for CLI usage) — targets the System keychain
        let output = Command::new("sudo")
            .args([
                "security",
                "add-trusted-cert",
                "-d",
                "-r",
                "trustRoot",
                "-k",
                "/Library/Keychains/System.keychain",
                cert_path_str,
            ])
            .output()?;

        if output.status.success() {
            tracing::info!("CA certificate trusted via macOS System Keychain");
            Ok(TrustResult::Trusted)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Ok(TrustResult::Failed(stderr.to_string()))
        }
    }
}

#[cfg(target_os = "macos")]
fn untrust_ca_macos(_ca_cert_path: &Path, use_sudo_prompt: bool) -> Result<TrustResult> {
    use std::process::Command;

    if use_sudo_prompt {
        // GUI path: remove from login keychain first (no admin needed)
        let home = std::env::var("HOME")
            .map_err(|_| anyhow::anyhow!("cannot determine home directory"))?;
        let login_keychain = PathBuf::from(&home).join("Library/Keychains/login.keychain-db");

        let login_keychain_str = login_keychain
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("invalid keychain path"))?;

        let output = Command::new("security")
            .args([
                "delete-certificate",
                "-c",
                "PortZero Local CA",
                login_keychain_str,
            ])
            .output()?;

        if output.status.success() {
            tracing::info!("CA certificate removed from macOS login keychain");
            return Ok(TrustResult::Trusted);
        }

        // If not in login keychain, try System keychain via osascript
        let script = r#"do shell script "security delete-certificate -c 'PortZero Local CA' /Library/Keychains/System.keychain" with administrator privileges"#;

        let output = Command::new("osascript").args(["-e", script]).output()?;

        if output.status.success() {
            tracing::info!("CA certificate removed from macOS System Keychain");
            Ok(TrustResult::Trusted)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("User canceled") || stderr.contains("-128") {
                Ok(TrustResult::NeedsSudo)
            } else {
                Ok(TrustResult::Failed(stderr.to_string()))
            }
        }
    } else {
        // CLI path: try both keychains with sudo
        let _ = Command::new("sudo")
            .args([
                "security",
                "delete-certificate",
                "-c",
                "PortZero Local CA",
                "/Library/Keychains/System.keychain",
            ])
            .output();

        let login_keychain = std::env::var("HOME")
            .map(|h| PathBuf::from(h).join("Library/Keychains/login.keychain-db"))
            .unwrap_or_default();

        let _ = Command::new("security")
            .args(["delete-certificate", "-c", "PortZero Local CA"])
            .arg(&login_keychain)
            .output();

        tracing::info!("CA certificate removal attempted from both keychains");
        Ok(TrustResult::Trusted)
    }
}

// ---------------------------------------------------------------------------
// Linux implementation
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
fn is_ca_trusted_linux(ca_cert_path: &Path) -> Result<bool> {
    let target = Path::new("/usr/local/share/ca-certificates/portzero-ca.crt");
    Ok(target.exists())
}

#[cfg(target_os = "linux")]
fn trust_ca_linux(ca_cert_path: &Path, use_sudo_prompt: bool) -> Result<TrustResult> {
    use std::process::Command;

    let cert_path_str = ca_cert_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("invalid cert path"))?;

    let (cp_prefix, update_prefix) = if use_sudo_prompt {
        // Use pkexec for GUI prompt
        ("pkexec", "pkexec")
    } else {
        ("sudo", "sudo")
    };

    // Copy cert
    let cp_output = Command::new(cp_prefix)
        .args([
            "cp",
            cert_path_str,
            "/usr/local/share/ca-certificates/portzero-ca.crt",
        ])
        .output()?;

    if !cp_output.status.success() {
        let stderr = String::from_utf8_lossy(&cp_output.stderr);
        return Ok(TrustResult::Failed(format!(
            "failed to copy cert: {}",
            stderr
        )));
    }

    // Update CA certificates
    let update_output = Command::new(update_prefix)
        .arg("update-ca-certificates")
        .output()?;

    if update_output.status.success() {
        tracing::info!("CA certificate trusted via update-ca-certificates");
        Ok(TrustResult::Trusted)
    } else {
        let stderr = String::from_utf8_lossy(&update_output.stderr);
        Ok(TrustResult::Failed(format!(
            "update-ca-certificates failed: {}",
            stderr
        )))
    }
}

#[cfg(target_os = "linux")]
fn untrust_ca_linux(use_sudo_prompt: bool) -> Result<TrustResult> {
    use std::process::Command;

    let prefix = if use_sudo_prompt { "pkexec" } else { "sudo" };

    let rm_output = Command::new(prefix)
        .args([
            "rm",
            "-f",
            "/usr/local/share/ca-certificates/portzero-ca.crt",
        ])
        .output()?;

    if !rm_output.status.success() {
        let stderr = String::from_utf8_lossy(&rm_output.stderr);
        return Ok(TrustResult::Failed(format!(
            "failed to remove cert: {}",
            stderr
        )));
    }

    let update_output = Command::new(prefix)
        .arg("update-ca-certificates")
        .output()?;

    if update_output.status.success() {
        Ok(TrustResult::Trusted)
    } else {
        let stderr = String::from_utf8_lossy(&update_output.stderr);
        Ok(TrustResult::Failed(format!(
            "update-ca-certificates failed: {}",
            stderr
        )))
    }
}

// ---------------------------------------------------------------------------
// Build rustls ServerConfig from stored certs
// ---------------------------------------------------------------------------

/// Build a `rustls::ServerConfig` from the stored server certificate and key.
pub fn build_tls_config(state_dir: &Path) -> Result<rustls::ServerConfig> {
    let paths = CertPaths::new(state_dir);
    let cert_pem = fs::read(&paths.server_cert)?;
    let key_pem = fs::read(&paths.server_key)?;

    let certs = rustls_pemfile::certs(&mut cert_pem.as_slice())
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let key = rustls_pemfile::private_key(&mut key_pem.as_slice())?
        .ok_or_else(|| anyhow::anyhow!("no private key found in server.key"))?;

    let config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)?;

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_certs() {
        let certs = generate_localhost_certs().unwrap();
        assert!(certs.ca_cert_pem.contains("BEGIN CERTIFICATE"));
        assert!(certs.ca_key_pem.contains("BEGIN PRIVATE KEY"));
        assert!(certs.server_cert_pem.contains("BEGIN CERTIFICATE"));
        assert!(certs.server_key_pem.contains("BEGIN PRIVATE KEY"));
    }

    #[test]
    fn test_ensure_certs() {
        let tmp = tempfile::tempdir().unwrap();
        let generated = ensure_certs(tmp.path()).unwrap();
        assert!(generated);

        let paths = CertPaths::new(tmp.path());
        assert!(paths.all_exist());

        // Second call should not regenerate
        let generated = ensure_certs(tmp.path()).unwrap();
        assert!(!generated);
    }
}
