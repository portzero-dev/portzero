//! Tunnel management endpoints.
//!
//! POST   /api/apps/:name/share   — Start a public tunnel
//! DELETE /api/apps/:name/share   — Stop a public tunnel

use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use portzero_core::types::*;

/// POST /api/apps/:name/share — Start a public tunnel for an app.
///
/// In the full system, this delegates to the TunnelManager (Task 4) which
/// connects to a LocalUp relay. For now, we store a placeholder tunnel and
/// broadcast the event.
pub async fn start_tunnel(
    Path(name): Path<String>,
    State(state): State<AppState>,
    body: Option<Json<ShareRequest>>,
) -> impl IntoResponse {
    if !state.apps.contains_key(&name) {
        return (
            StatusCode::NOT_FOUND,
            Json(ApiError::not_found(format!("App '{}' not found", name))),
        )
            .into_response();
    }

    if state.tunnels.contains_key(&name) {
        return (
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": "conflict",
                "message": format!("Tunnel already active for '{}'", name),
                "tunnel": state.tunnels.get(&name).map(|t| t.value().clone()),
            })),
        )
            .into_response();
    }

    let subdomain = body
        .as_ref()
        .and_then(|b| b.subdomain.clone())
        .unwrap_or_else(|| name.clone());

    let tunnel = TunnelInfo {
        app_name: name.clone(),
        public_url: format!("https://{}.relay.portzero.dev", subdomain),
        relay: body
            .as_ref()
            .and_then(|b| b.relay.clone())
            .unwrap_or_else(|| "relay.portzero.dev:4443".to_string()),
        transport: "quic".to_string(),
        started_at: chrono::Utc::now(),
    };

    state.tunnels.insert(name.clone(), tunnel.clone());

    state.ws_hub.broadcast(WsEvent::TunnelStarted {
        app: name,
        public_url: tunnel.public_url.clone(),
    });

    (StatusCode::CREATED, Json(tunnel)).into_response()
}

/// DELETE /api/apps/:name/share — Stop a public tunnel.
pub async fn stop_tunnel(
    Path(name): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match state.tunnels.remove(&name) {
        Some(_) => {
            state.ws_hub.broadcast(WsEvent::TunnelStopped { app: name });
            StatusCode::NO_CONTENT.into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(ApiError::not_found(format!(
                "No active tunnel for '{}'",
                name
            ))),
        )
            .into_response(),
    }
}
