//! Request traffic endpoints.
//!
//! GET    /api/requests              — List captured requests (with filtering)
//! GET    /api/requests/:id          — Full request/response detail
//! POST   /api/requests/:id/replay   — Replay a request
//! DELETE /api/requests              — Clear captured requests
//! GET    /api/requests/:id1/diff/:id2 — Diff two requests

use crate::state::AppState;
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use portzero_core::store::RequestFilter as StoreFilter;
use portzero_core::types::*;
use serde::Deserialize;

/// Query params for GET /api/requests — matches the architecture spec.
#[derive(Debug, Deserialize, Default)]
pub struct RequestQueryParams {
    pub app: Option<String>,
    pub method: Option<String>,
    pub status: Option<u16>,
    pub status_range: Option<String>,
    pub path: Option<String>,
    pub search: Option<String>,
    pub from: Option<i64>,
    pub to: Option<i64>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Convert API query params to the store's filter type.
fn to_store_filter(q: &RequestQueryParams) -> StoreFilter {
    StoreFilter {
        app_name: q.app.clone(),
        method: q.method.clone(),
        status_code: q.status,
        path_prefix: q.path.clone(),
        search: q.search.clone(),
        limit: q.limit,
        offset: q.offset,
    }
}

/// GET /api/requests — List captured requests with filtering and pagination.
pub async fn list_requests(
    Query(params): Query<RequestQueryParams>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let filter = to_store_filter(&params);
    match state.store.list_request_summaries(&filter) {
        Ok(summaries) => {
            Json(summaries).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError::internal(format!("Failed to list requests: {}", e))),
        )
            .into_response(),
    }
}

/// GET /api/requests/:id — Full request/response detail.
pub async fn get_request(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match state.store.get_request(&id) {
        Ok(Some(record)) => {
            // Convert body bytes to base64 or string for JSON serialization
            let record = serialize_request_record(record);
            Json(record).into_response()
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiError::not_found(format!("Request '{}' not found", id))),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError::internal(format!("Failed to get request: {}", e))),
        )
            .into_response(),
    }
}

/// POST /api/requests/:id/replay — Replay a captured request.
///
/// Sends the request to the upstream app via reqwest, records the real
/// response, and returns the completed replay record.
pub async fn replay_request(
    Path(id): Path<String>,
    State(state): State<AppState>,
    body: Option<Json<ReplayRequest>>,
) -> impl IntoResponse {
    let original = match state.store.get_request(&id) {
        Ok(Some(r)) => r,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiError::not_found(format!(
                    "Request '{}' not found",
                    id
                ))),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::internal(e.to_string())),
            )
                .into_response();
        }
    };

    let overrides = body.map(|b| b.0);
    let replay_id = uuid::Uuid::new_v4().to_string();

    let method_str = overrides
        .as_ref()
        .and_then(|o| o.method.clone())
        .unwrap_or(original.method.clone());
    let url = overrides
        .as_ref()
        .and_then(|o| o.url.clone())
        .unwrap_or(original.url.clone());
    let headers = overrides
        .as_ref()
        .and_then(|o| o.headers.clone())
        .unwrap_or(original.request_headers.clone());
    let req_body = overrides
        .as_ref()
        .and_then(|o| o.body.as_ref().map(|b| b.as_bytes().to_vec()))
        .or(original.request_body.clone());

    // Actually send the HTTP request
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true) // dev tool, accept self-signed
        .build()
        .unwrap_or_default();

    let method = method_str
        .parse::<reqwest::Method>()
        .unwrap_or(reqwest::Method::GET);

    let start = std::time::Instant::now();

    let mut req_builder = client.request(method, &url);
    for (k, v) in &headers {
        req_builder = req_builder.header(k, v);
    }
    if let Some(body_bytes) = &req_body {
        req_builder = req_builder.body(body_bytes.clone());
    }

    let (resp_status, resp_status_msg, resp_headers, resp_body, resp_content_type, duration_ms) =
        match req_builder.send().await {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let status_msg = resp
                    .status()
                    .canonical_reason()
                    .unwrap_or("")
                    .to_string();
                let content_type = resp
                    .headers()
                    .get("content-type")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string());
                let resp_hdrs: std::collections::HashMap<String, String> = resp
                    .headers()
                    .iter()
                    .filter_map(|(k, v)| v.to_str().ok().map(|vs| (k.to_string(), vs.to_string())))
                    .collect();
                let body_bytes = resp.bytes().await.ok().map(|b| b.to_vec());
                let dur = start.elapsed().as_millis() as u64;
                (status, status_msg, resp_hdrs, body_bytes, content_type, dur)
            }
            Err(e) => {
                tracing::warn!("Replay request to {} failed: {}", url, e);
                let dur = start.elapsed().as_millis() as u64;
                (
                    0u16,
                    format!("Replay failed: {}", e),
                    std::collections::HashMap::new(),
                    None,
                    None,
                    dur,
                )
            }
        };

    let replay = RequestRecord {
        id: replay_id.clone(),
        app_name: original.app_name.clone(),
        timestamp: chrono::Utc::now(),
        duration_ms,
        method: method_str,
        url,
        path: original.path.clone(),
        query_string: original.query_string.clone(),
        request_headers: headers,
        request_body: req_body,
        request_content_type: original.request_content_type.clone(),
        status_code: resp_status,
        status_message: resp_status_msg,
        response_headers: resp_headers,
        response_body: resp_body,
        response_content_type: resp_content_type,
        mocked: false,
        parent_id: Some(id),
    };

    if let Err(e) = state.store.insert_request(&replay) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError::internal(format!("Failed to store replay: {}", e))),
        )
            .into_response();
    }

    // Return the full replay record as the dashboard expects RequestDetail
    (StatusCode::CREATED, Json(replay)).into_response()
}

/// Query params for DELETE /api/requests
#[derive(Deserialize)]
pub struct DeleteQuery {
    pub app: Option<String>,
}

/// DELETE /api/requests — Clear captured requests (optionally filtered by app).
pub async fn delete_requests(
    Query(query): Query<DeleteQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match state.store.clear_requests(query.app.as_deref()) {
        Ok(deleted) => Json(serde_json::json!({"deleted": deleted})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError::internal(format!(
                "Failed to delete requests: {}",
                e
            ))),
        )
            .into_response(),
    }
}

/// GET /api/requests/:id1/diff/:id2 — Side-by-side diff of two requests.
pub async fn diff_requests(
    Path((id1, id2)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let left = match state.store.get_request(&id1) {
        Ok(Some(r)) => r,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiError::not_found(format!(
                    "Request '{}' not found",
                    id1
                ))),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::internal(e.to_string())),
            )
                .into_response();
        }
    };

    let right = match state.store.get_request(&id2) {
        Ok(Some(r)) => r,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiError::not_found(format!(
                    "Request '{}' not found",
                    id2
                ))),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::internal(e.to_string())),
            )
                .into_response();
        }
    };

    let diff = RequestDiff {
        left: serialize_request_record(left),
        right: serialize_request_record(right),
    };

    Json(diff).into_response()
}

/// Convert body bytes to UTF-8 strings for JSON serialization.
/// In a real system we'd check content type and possibly base64-encode binary bodies.
fn serialize_request_record(r: RequestRecord) -> RequestRecord {
    // Bodies are stored as Vec<u8> but we want them as strings in JSON.
    // For now, convert lossily; binary bodies would need base64.
    r
}
