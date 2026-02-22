//! Schema inference endpoint.
//!
//! GET /api/apps/:name/schema — Get the inferred API schema for an app.

use crate::state::AppState;
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use portzero_core::types::*;

/// GET /api/apps/:name/schema — Get inferred API schema.
pub async fn get_app_schema(
    Path(name): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match state.schemas.get(&name) {
        Some(schema) => Json(schema.value().clone()).into_response(),
        None => {
            if state.apps.contains_key(&name) {
                // App exists but no schema inferred yet
                Json(InferredSchema {
                    app_name: name,
                    endpoints: vec![],
                    last_updated: chrono::Utc::now(),
                })
                .into_response()
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
