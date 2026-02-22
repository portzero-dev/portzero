//! Axum HTTP server setup and router construction.

use crate::routes;
use crate::state::AppState;
use crate::static_files::serve_dashboard;
use crate::ws::ws_handler;

use axum::{
    routing::{delete, get, patch, post, put},
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

/// Build the complete axum Router with all API routes.
pub fn build_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let api = Router::new()
        // Apps
        .route("/api/apps", get(routes::apps::list_apps))
        .route("/api/apps/{name}", get(routes::apps::get_app))
        .route("/api/apps/{name}/restart", post(routes::apps::restart_app))
        .route("/api/apps/{name}/stop", post(routes::apps::stop_app))
        .route("/api/apps/{name}/logs", get(routes::apps::get_app_logs))
        .route(
            "/api/apps/{name}/schema",
            get(routes::schema::get_app_schema),
        )
        // Traffic
        .route("/api/requests", get(routes::requests::list_requests))
        .route("/api/requests", delete(routes::requests::delete_requests))
        .route("/api/requests/{id}", get(routes::requests::get_request))
        .route(
            "/api/requests/{id}/replay",
            post(routes::requests::replay_request),
        )
        .route(
            "/api/requests/{id1}/diff/{id2}",
            get(routes::requests::diff_requests),
        )
        // Mocks
        .route("/api/mocks", get(routes::mocks::list_mocks))
        .route("/api/mocks", post(routes::mocks::create_mock))
        .route("/api/mocks/{id}", put(routes::mocks::update_mock))
        .route("/api/mocks/{id}", delete(routes::mocks::delete_mock))
        .route("/api/mocks/{id}/toggle", patch(routes::mocks::toggle_mock))
        // Network simulation
        .route("/api/network/{app}", get(routes::network::get_profile))
        .route("/api/network/{app}", put(routes::network::set_profile))
        .route("/api/network/{app}", delete(routes::network::clear_profile))
        // Tunnel (behind feature flag — will be enabled in a future release)
        ;
    #[cfg(feature = "tunnel")]
    let api = api
        .route("/api/apps/{name}/share", post(routes::tunnel::start_tunnel))
        .route(
            "/api/apps/{name}/share",
            delete(routes::tunnel::stop_tunnel),
        );
    let api = api
        // Status
        .route("/api/status", get(routes::status::get_status))
        // Certificates
        .route("/api/certs/status", get(routes::certs::get_cert_status))
        .route("/api/certs/trust", post(routes::certs::trust_ca))
        .route("/api/certs/untrust", post(routes::certs::untrust_ca))
        // WebSocket
        .route("/api/ws", get(ws_handler));

    Router::new()
        .merge(api)
        .fallback(serve_dashboard)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
