//! CLI commands: `portzero trust` / `portzero untrust`
//!
//! Installs or removes the PortZero CA certificate from the system trust store.
//! Requires sudo/admin privileges.

use anyhow::Result;
use portzero_core::certs::{self, TrustResult};
use std::path::Path;

/// Install the PortZero CA certificate into the system trust store.
pub fn trust(state_dir: &Path) -> Result<()> {
    // Ensure certs exist first
    certs::ensure_certs(state_dir)?;

    println!("Installing PortZero CA certificate into system trust store...");
    println!("This requires administrator privileges.");
    println!();

    match certs::trust_ca(state_dir, false)? {
        TrustResult::Trusted => {
            println!("CA certificate trusted successfully!");
            println!("HTTPS is now available for all *.localhost domains.");
            println!();
            println!("Note: You may need to restart your browser for the change to take effect.");
        }
        TrustResult::AlreadyTrusted => {
            println!("CA certificate is already trusted.");
        }
        TrustResult::NeedsSudo => {
            println!("Administrator privileges required.");
            println!();
            println!("Run manually:");
            println!("  {}", certs::trust_ca_command(state_dir));
        }
        TrustResult::Failed(msg) => {
            eprintln!("Failed to trust CA certificate: {}", msg);
            eprintln!();
            eprintln!("Try running manually:");
            eprintln!("  {}", certs::trust_ca_command(state_dir));
        }
        TrustResult::Unsupported => {
            let paths = certs::CertPaths::new(state_dir);
            eprintln!("Automatic trust is not supported on this platform.");
            eprintln!();
            eprintln!("Manually import the CA certificate into your system's trust store:");
            eprintln!("  {}", paths.ca_cert.display());
        }
    }

    Ok(())
}

/// Remove the PortZero CA certificate from the system trust store.
pub fn untrust(state_dir: &Path) -> Result<()> {
    println!("Removing PortZero CA certificate from system trust store...");

    match certs::untrust_ca(state_dir, false)? {
        TrustResult::Trusted => {
            println!("CA certificate removed successfully.");
        }
        TrustResult::AlreadyTrusted => {
            println!("Done.");
        }
        TrustResult::NeedsSudo => {
            println!("Administrator privileges required.");
        }
        TrustResult::Failed(msg) => {
            eprintln!("Failed to remove CA certificate: {}", msg);
        }
        TrustResult::Unsupported => {
            eprintln!("Automatic untrust is not supported on this platform.");
        }
    }

    Ok(())
}
