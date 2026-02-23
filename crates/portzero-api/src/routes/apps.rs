//! App management endpoints.
//!
//! GET    /api/apps           — List all connected apps
//! GET    /api/apps/:name     — Single app details
//! POST   /api/apps/:name/restart — Restart an app
//! POST   /api/apps/:name/stop    — Stop an app
//! GET    /api/apps/:name/logs    — Get buffered logs

use crate::state::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use portzero_core::types::*;
use serde::Deserialize;

/// GET /api/apps — List all registered apps.
///
/// Filters out the internal `_portzero` dashboard route.
pub async fn list_apps(State(state): State<AppState>) -> impl IntoResponse {
    let apps: Vec<AppInfo> = state
        .apps
        .iter()
        .filter(|entry| entry.key() != RESERVED_SUBDOMAIN)
        .map(|entry| entry.value().clone())
        .collect();
    Json(apps)
}

/// GET /api/apps/:name — Get a single app's details.
pub async fn get_app(Path(name): Path<String>, State(state): State<AppState>) -> impl IntoResponse {
    match state.apps.get(&name) {
        Some(app) => Json(serde_json::to_value(app.value()).unwrap()).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(ApiError::not_found(format!("App '{}' not found", name))),
        )
            .into_response(),
    }
}

/// POST /api/apps/:name/restart — Restart an app.
///
/// In the real system, this triggers the process manager to restart the child.
/// The API server broadcasts the event; the actual restart is handled by the daemon.
pub async fn restart_app(
    Path(name): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match state.apps.get(&name) {
        Some(app) => {
            // Broadcast restart event — the daemon's process manager listens for this.
            state.ws_hub.broadcast(WsEvent::AppRestarted {
                name: name.clone(),
                pid: app.pid.unwrap_or(0),
            });
            (
                StatusCode::OK,
                Json(serde_json::json!({"status": "restart_requested", "app": name})),
            )
                .into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(ApiError::not_found(format!("App '{}' not found", name))),
        )
            .into_response(),
    }
}

/// POST /api/apps/:name/stop — Stop an app.
pub async fn stop_app(
    Path(name): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match state.apps.get(&name) {
        Some(_) => {
            state
                .ws_hub
                .broadcast(WsEvent::AppRemoved { name: name.clone() });
            (
                StatusCode::OK,
                Json(serde_json::json!({"status": "stop_requested", "app": name})),
            )
                .into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(ApiError::not_found(format!("App '{}' not found", name))),
        )
            .into_response(),
    }
}

/// Query params for GET /api/apps/:name/logs
#[derive(Deserialize)]
pub struct LogQuery {
    pub lines: Option<usize>,
}

/// GET /api/apps/:name/logs — Get buffered log lines.
pub async fn get_app_logs(
    Path(name): Path<String>,
    Query(query): Query<LogQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let max_lines = query.lines.unwrap_or(100);

    match state.logs.get(&name) {
        Some(log_buffer) => {
            let logs: Vec<&LogLine> = log_buffer.iter().rev().take(max_lines).collect();
            let logs: Vec<LogLine> = logs.into_iter().rev().cloned().collect();
            Json(serde_json::to_value(&logs).unwrap()).into_response()
        }
        None => {
            // App exists but no logs yet, or app doesn't exist
            if state.apps.contains_key(&name) {
                Json(serde_json::json!([])).into_response()
            } else {
                (
                    StatusCode::NOT_FOUND,
                    Json(ApiError::not_found(format!("App '{}' not found", name))),
                )
                    .into_response()
            }
        }
    }
}
