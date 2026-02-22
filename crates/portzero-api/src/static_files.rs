//! Serve the embedded dashboard SPA via rust-embed.
//!
//! In production, the dashboard is pre-built by Vite and embedded in the binary.
//! For now, a placeholder index.html is served.

use axum::{
    extract::Request,
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
};
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "../../dashboard-dist"]
struct DashboardAssets;

/// Fallback handler that serves the embedded dashboard SPA.
pub async fn serve_dashboard(req: Request) -> Response {
    let path = req.uri().path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    match DashboardAssets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, mime.as_ref())],
                content.data.to_vec(),
            )
                .into_response()
        }
        None => {
            // SPA fallback: serve index.html for any unmatched path
            // (client-side routing handles the rest)
            match DashboardAssets::get("index.html") {
                Some(index) => Html(
                    String::from_utf8_lossy(&index.data).to_string(),
                )
                .into_response(),
                None => (StatusCode::NOT_FOUND, "Dashboard not found").into_response(),
            }
        }
    }
}
