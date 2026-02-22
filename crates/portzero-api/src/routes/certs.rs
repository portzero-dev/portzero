//! Certificate management endpoints.
//!
//! GET  /api/certs/status — Check if the CA is trusted
//! POST /api/certs/trust  — Trust the CA in the system store (requires sudo prompt)
//! POST /api/certs/untrust — Remove the CA from the system store

use crate::state::AppState;
use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
};
use portzero_core::certs;
use serde::Serialize;

/// Response for GET /api/certs/status
#[derive(Serialize)]
pub struct CertStatus {
    /// Whether certificates have been generated
    pub certs_exist: bool,
    /// Whether the CA is trusted in the system trust store
    pub ca_trusted: bool,
    /// Path to the CA certificate file
    pub ca_cert_path: String,
    /// Manual trust command for the user
    pub trust_command: String,
}

/// GET /api/certs/status — Check certificate and trust status.
pub async fn get_cert_status(State(state): State<AppState>) -> impl IntoResponse {
    let paths = certs::CertPaths::new(&state.state_dir);
    let certs_exist = paths.all_exist();
    let ca_trusted = certs::is_ca_trusted(&state.state_dir).unwrap_or(false);
    let trust_command = certs::trust_ca_command(&state.state_dir);

    Json(CertStatus {
        certs_exist,
        ca_trusted,
        ca_cert_path: paths.ca_cert.display().to_string(),
        trust_command,
    })
}

/// Response for trust/untrust operations.
#[derive(Serialize)]
pub struct TrustResponse {
    pub status: String,
    pub message: String,
}

/// POST /api/certs/trust — Trust the CA certificate in the system store.
///
/// Uses `osascript` on macOS or `pkexec` on Linux to prompt for admin password.
pub async fn trust_ca(State(state): State<AppState>) -> impl IntoResponse {
    // Ensure certs exist first
    if let Err(e) = certs::ensure_certs(&state.state_dir) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(TrustResponse {
                status: "error".to_string(),
                message: format!("Failed to generate certificates: {}", e),
            }),
        )
            .into_response();
    }

    match certs::trust_ca(&state.state_dir, true) {
        Ok(certs::TrustResult::Trusted) => Json(TrustResponse {
            status: "trusted".to_string(),
            message: "CA certificate trusted in system store".to_string(),
        })
        .into_response(),
        Ok(certs::TrustResult::AlreadyTrusted) => Json(TrustResponse {
            status: "already_trusted".to_string(),
            message: "CA certificate is already trusted".to_string(),
        })
        .into_response(),
        Ok(certs::TrustResult::NeedsSudo) => (
            StatusCode::FORBIDDEN,
            Json(TrustResponse {
                status: "cancelled".to_string(),
                message: "User cancelled the password prompt".to_string(),
            }),
        )
            .into_response(),
        Ok(certs::TrustResult::Unsupported) => (
            StatusCode::NOT_IMPLEMENTED,
            Json(TrustResponse {
                status: "unsupported".to_string(),
                message: "Automatic trust is not supported on this platform".to_string(),
            }),
        )
            .into_response(),
        Ok(certs::TrustResult::Failed(msg)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(TrustResponse {
                status: "error".to_string(),
                message: msg,
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(TrustResponse {
                status: "error".to_string(),
                message: e.to_string(),
            }),
        )
            .into_response(),
    }
}

/// POST /api/certs/untrust — Remove the CA certificate from the system store.
pub async fn untrust_ca(State(state): State<AppState>) -> impl IntoResponse {
    match certs::untrust_ca(&state.state_dir, true) {
        Ok(certs::TrustResult::Trusted) => Json(TrustResponse {
            status: "untrusted".to_string(),
            message: "CA certificate removed from system store".to_string(),
        })
        .into_response(),
        Ok(certs::TrustResult::AlreadyTrusted) => Json(TrustResponse {
            status: "not_trusted".to_string(),
            message: "CA certificate was not in system store".to_string(),
        })
        .into_response(),
        Ok(certs::TrustResult::NeedsSudo) => (
            StatusCode::FORBIDDEN,
            Json(TrustResponse {
                status: "cancelled".to_string(),
                message: "User cancelled the password prompt".to_string(),
            }),
        )
            .into_response(),
        Ok(certs::TrustResult::Unsupported) => (
            StatusCode::NOT_IMPLEMENTED,
            Json(TrustResponse {
                status: "unsupported".to_string(),
                message: "Automatic untrust is not supported on this platform".to_string(),
            }),
        )
            .into_response(),
        Ok(certs::TrustResult::Failed(msg)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(TrustResponse {
                status: "error".to_string(),
                message: msg,
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(TrustResponse {
                status: "error".to_string(),
                message: e.to_string(),
            }),
        )
            .into_response(),
    }
}
