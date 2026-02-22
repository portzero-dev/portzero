//! Mock rule endpoints.
//!
//! GET    /api/mocks           — List all mock rules
//! POST   /api/mocks           — Create a mock rule
//! PUT    /api/mocks/:id       — Update a mock rule
//! DELETE /api/mocks/:id       — Delete a mock rule
//! PATCH  /api/mocks/:id/toggle — Enable/disable a mock

use crate::state::AppState;
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use portzero_core::types::*;

/// GET /api/mocks — List all mock rules.
pub async fn list_mocks(State(state): State<AppState>) -> impl IntoResponse {
    match state.store.list_mocks(None) {
        Ok(mocks) => Json(mocks).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError::internal(format!("Failed to list mocks: {}", e))),
        )
            .into_response(),
    }
}

/// POST /api/mocks — Create a mock rule.
pub async fn create_mock(
    State(state): State<AppState>,
    Json(body): Json<CreateMockRule>,
) -> impl IntoResponse {
    let rule = MockRule {
        id: uuid::Uuid::new_v4().to_string(),
        app_name: body.app_name,
        method: body.method,
        path_pattern: body.path_pattern,
        status_code: body.status_code,
        response_headers: body.response_headers,
        response_body: body.response_body,
        enabled: body.enabled,
        hit_count: 0,
    };

    match state.store.insert_mock(&rule) {
        Ok(()) => (StatusCode::CREATED, Json(rule)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError::internal(format!("Failed to create mock: {}", e))),
        )
            .into_response(),
    }
}

/// PUT /api/mocks/:id — Update a mock rule.
pub async fn update_mock(
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(body): Json<UpdateMockRule>,
) -> impl IntoResponse {
    // Get existing rule
    let existing = match state.store.list_mocks(None) {
        Ok(mocks) => mocks.into_iter().find(|m| m.id == id),
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::internal(e.to_string())),
            )
                .into_response();
        }
    };

    let Some(existing) = existing else {
        return (
            StatusCode::NOT_FOUND,
            Json(ApiError::not_found(format!("Mock '{}' not found", id))),
        )
            .into_response();
    };

    let updated = MockRule {
        id: existing.id.clone(),
        app_name: existing.app_name.clone(),
        method: body.method.unwrap_or(existing.method),
        path_pattern: body.path_pattern.unwrap_or(existing.path_pattern),
        status_code: body.status_code.unwrap_or(existing.status_code),
        response_headers: body.response_headers.unwrap_or(existing.response_headers),
        response_body: body.response_body.unwrap_or(existing.response_body),
        enabled: body.enabled.unwrap_or(existing.enabled),
        hit_count: existing.hit_count,
    };

    // Use insert_mock with INSERT OR REPLACE
    match state.store.insert_mock(&updated) {
        Ok(()) => Json(updated).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError::internal(format!("Failed to update mock: {}", e))),
        )
            .into_response(),
    }
}

/// DELETE /api/mocks/:id — Delete a mock rule.
pub async fn delete_mock(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match state.store.delete_mock(&id) {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(ApiError::not_found(format!("Mock '{}' not found", id))),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError::internal(format!("Failed to delete mock: {}", e))),
        )
            .into_response(),
    }
}

/// PATCH /api/mocks/:id/toggle — Toggle a mock's enabled state.
pub async fn toggle_mock(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // Get existing, toggle, re-insert
    let existing = match state.store.list_mocks(None) {
        Ok(mocks) => mocks.into_iter().find(|m| m.id == id),
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::internal(e.to_string())),
            )
                .into_response();
        }
    };

    let Some(existing) = existing else {
        return (
            StatusCode::NOT_FOUND,
            Json(ApiError::not_found(format!("Mock '{}' not found", id))),
        )
            .into_response();
    };

    let toggled = MockRule {
        enabled: !existing.enabled,
        ..existing
    };

    match state.store.insert_mock(&toggled) {
        Ok(()) => Json(toggled).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError::internal(e.to_string())),
        )
            .into_response(),
    }
}
