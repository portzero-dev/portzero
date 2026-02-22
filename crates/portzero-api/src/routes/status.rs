//! System status endpoint.
//!
//! GET /api/status — Daemon health + stats

use crate::state::AppState;
use axum::{Json, extract::State, response::IntoResponse};
use portzero_core::types::*;

/// GET /api/status — Get daemon health and stats.
pub async fn get_status(State(state): State<AppState>) -> impl IntoResponse {
    let uptime = state.started_at.elapsed().as_secs();
    let total_requests = state.store.request_count().unwrap_or(0);
    let status = DaemonStatus {
        uptime_secs: uptime,
        proxy_port: DEFAULT_PROXY_PORT,
        total_apps: state.apps.len(),
        total_requests,
        version: env!("CARGO_PKG_VERSION").to_string(),
    };

    Json(status)
}
