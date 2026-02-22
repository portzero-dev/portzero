//! Network simulation endpoints.
//!
//! GET    /api/network/:app    — Get current network profile
//! PUT    /api/network/:app    — Set network profile
//! DELETE /api/network/:app    — Clear network simulation

use crate::state::AppState;
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use portzero_core::types::*;

/// GET /api/network/:app — Get the current network profile for an app.
pub async fn get_profile(
    Path(app): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match state.network_profiles.get(&app) {
        Some(profile) => Json(profile.value().clone()).into_response(),
        None => Json(serde_json::json!(null)).into_response(),
    }
}

/// PUT /api/network/:app — Set the network simulation profile.
pub async fn set_profile(
    Path(app): Path<String>,
    State(state): State<AppState>,
    Json(body): Json<SetNetworkProfile>,
) -> impl IntoResponse {
    let profile = NetworkProfile {
        app_name: app.clone(),
        latency_ms: body.latency_ms,
        jitter_ms: body.jitter_ms,
        packet_loss_rate: body.packet_loss_rate.unwrap_or(0.0),
        bandwidth_limit: body.bandwidth_limit,
        path_filter: body.path_filter,
    };

    state.network_profiles.insert(app, profile.clone());
    Json(profile).into_response()
}

/// DELETE /api/network/:app — Clear network simulation for an app.
pub async fn clear_profile(
    Path(app): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    state.network_profiles.remove(&app);
    StatusCode::NO_CONTENT
}
