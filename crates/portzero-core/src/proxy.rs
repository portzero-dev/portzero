//! Pingora `ProxyHttp` trait implementation — the heart of PortZero.
//!
//! Routes `<name>.localhost:1337` → `127.0.0.1:<port>` based on subdomain,
//! with callbacks that wire in recording, interception, mocking, and network
//! simulation at each stage of the request lifecycle.
//!
//! # Lifecycle
//!
//! ```text
//! upstream_peer()       → resolve subdomain, pick upstream
//! request_filter()      → mock check, intercept check
//! upstream_request_filter() → rewrite Host header, network sim delay
//! upstream_response_filter()  → capture response headers
//! upstream_response_body_filter() → capture response body chunks
//! logging()             → persist to SQLite, broadcast WS event
//! ```

use crate::mock_engine::MockEngine;
use crate::network_sim::NetworkSim;
use crate::recorder::{Recorder, RecordingSession};
use crate::router::{self, Router};
use crate::types::RESERVED_SUBDOMAIN;
use crate::ws::WsHub;
use async_trait::async_trait;
use bytes::Bytes;
use pingora_core::upstreams::peer::HttpPeer;
use pingora_error::{Error, ErrorType};
use pingora_http::{RequestHeader, ResponseHeader};
use pingora_proxy::{ProxyHttp, Session};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Per-request context threaded through all Pingora callbacks.
pub struct RequestContext {
    /// Which app is handling this request.
    pub app_name: String,
    /// Target upstream port.
    pub target_port: u16,
    /// The active recording session (built incrementally across callbacks).
    pub recording: Option<RecordingSession>,
    /// When the request started.
    pub request_start: Instant,
    /// Whether this response was served by the mock engine.
    pub mocked: bool,
    /// Captured request method (for logging callback, which can't access session headers).
    pub method: String,
    /// Captured request path.
    pub path: String,
    /// Captured request URL.
    pub url: String,
}

impl Default for RequestContext {
    fn default() -> Self {
        Self {
            app_name: String::new(),
            target_port: 0,
            recording: None,
            request_start: Instant::now(),
            mocked: false,
            method: String::new(),
            path: String::new(),
            url: String::new(),
        }
    }
}

/// The PortZero reverse proxy.
///
/// Implements Pingora's `ProxyHttp` trait, routing by subdomain and recording
/// all traffic for the dashboard.
pub struct PortZeroProxy {
    /// Subdomain → port routing table.
    pub router: Arc<Router>,
    /// Request/response recorder (async persistence to SQLite).
    pub recorder: Arc<Recorder>,
    /// WebSocket hub for real-time events to the dashboard.
    pub ws_hub: Arc<WsHub>,
    /// Network simulation engine (latency, packet loss, bandwidth throttling).
    pub network_sim: Arc<NetworkSim>,
    /// Mock engine: serves synthetic responses for matching requests.
    pub mock_engine: Arc<MockEngine>,
}

impl PortZeroProxy {
    pub fn new(
        router: Arc<Router>,
        recorder: Arc<Recorder>,
        ws_hub: Arc<WsHub>,
        network_sim: Arc<NetworkSim>,
        mock_engine: Arc<MockEngine>,
    ) -> Self {
        Self {
            router,
            recorder,
            ws_hub,
            network_sim,
            mock_engine,
        }
    }
}

#[async_trait]
impl ProxyHttp for PortZeroProxy {
    type CTX = RequestContext;

    fn new_ctx(&self) -> Self::CTX {
        RequestContext::default()
    }

    /// Check mocks before connecting to upstream.
    ///
    /// If a mock matches, the response is written directly and `Ok(true)` is
    /// returned (response already sent — skip upstream). The request is still
    /// recorded with `mocked: true`.
    async fn request_filter(
        &self,
        session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> pingora_core::Result<bool> {
        let host = session
            .req_header()
            .headers
            .get("host")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");

        let subdomain = router::extract_subdomain(host);
        if subdomain.is_empty() || subdomain == RESERVED_SUBDOMAIN {
            return Ok(false); // Dashboard: pass through. Empty: let upstream_peer handle the error.
        }

        // Resolve route to confirm the app exists (no point mocking for unknown apps)
        if self.router.resolve(subdomain).is_none() {
            return Ok(false);
        }

        let method = session.req_header().method.as_str().to_string();
        let path = session.req_header().uri.path().to_string();

        // --- Mock check ---
        // Use the subdomain as a temporary request ID for the mock hit event;
        // it'll be replaced by the real recording ID if we record.
        let request_id = uuid::Uuid::new_v4().to_string();
        if let Some(mock_resp) =
            self.mock_engine
                .match_request(subdomain, &method, &path, &request_id)
        {
            tracing::info!(
                app = %subdomain,
                method = %method,
                path = %path,
                status = mock_resp.status_code,
                "Serving mock response"
            );

            // Build the full URL for recording
            let full_url = {
                let pq = session
                    .req_header()
                    .uri
                    .path_and_query()
                    .map(|pq| pq.as_str())
                    .unwrap_or(&path);
                format!("http://{}{}", host, pq)
            };
            let query = session.req_header().uri.query().unwrap_or("").to_string();

            // Start a recording session so the mocked request appears in traffic
            let mut recording = self
                .recorder
                .start_recording(subdomain, &method, &full_url, &path);
            recording.set_query_string(query);

            // Capture request headers
            let req_headers: HashMap<String, String> = session
                .req_header()
                .headers
                .iter()
                .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
                .collect();
            recording.set_request_headers(req_headers);

            // Set mock response on the recording
            recording.set_response_status(mock_resp.status_code);
            recording.set_response_headers(mock_resp.headers.clone());
            recording.append_response_body(&Bytes::from(mock_resp.body.clone()));
            recording.set_mocked();

            // Complete the recording
            recording.complete_sync();

            // Write mock response directly to the client.
            // We consume mock_resp fields to satisfy Pingora's 'static requirement
            // on header names/values.
            let body_len = mock_resp.body.len();
            let mut resp =
                ResponseHeader::build(mock_resp.status_code, Some(mock_resp.headers.len()))?;
            for (k, v) in mock_resp.headers {
                let _ = resp.insert_header(k, &v);
            }
            let _ = resp.insert_header("content-length", &body_len.to_string());

            session.write_response_header(Box::new(resp), false).await?;
            session
                .write_response_body(Some(Bytes::from(mock_resp.body)), true)
                .await?;

            return Ok(true); // Response already sent
        }

        Ok(false) // No mock matched, proceed to upstream
    }

    /// Route based on subdomain: `my-app.localhost:1337` → `127.0.0.1:<port>`
    async fn upstream_peer(
        &self,
        session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> pingora_core::Result<Box<HttpPeer>> {
        let host = session
            .req_header()
            .headers
            .get("host")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");

        let subdomain = router::extract_subdomain(host);

        if subdomain.is_empty() {
            return Err(Error::explain(
                ErrorType::HTTPStatus(404),
                "no subdomain in request — use <app>.localhost",
            ));
        }

        let route = self.router.resolve(subdomain).ok_or_else(|| {
            Error::explain(
                ErrorType::HTTPStatus(502),
                format!("no app registered for subdomain '{}'", subdomain),
            )
        })?;

        ctx.app_name = subdomain.to_string();
        ctx.target_port = route.port;
        ctx.request_start = Instant::now();

        // Capture request metadata for recording.
        // Skip recording for the reserved dashboard subdomain — we don't want
        // the dashboard's own API calls cluttering the traffic inspector.
        let is_dashboard = subdomain == RESERVED_SUBDOMAIN;

        let method = session.req_header().method.as_str().to_string();
        let path = session.req_header().uri.path().to_string();
        let query = session.req_header().uri.query().unwrap_or("").to_string();

        // Build the full URL including the host so replays can target the right address.
        // HTTP/1.1 requests typically have a relative URI (just the path), so we
        // reconstruct the full URL from the Host header.
        let full_url = {
            let path_and_query = session
                .req_header()
                .uri
                .path_and_query()
                .map(|pq| pq.as_str())
                .unwrap_or(&path);
            format!("http://{}{}", host, path_and_query)
        };

        ctx.method = method.clone();
        ctx.url = full_url.clone();
        ctx.path = path.clone();

        if !is_dashboard {
            // Start a recording session
            let mut recording =
                self.recorder
                    .start_recording(&ctx.app_name, &method, &full_url, &path);
            recording.set_query_string(query);

            // Capture request headers
            let req_headers: HashMap<String, String> = session
                .req_header()
                .headers
                .iter()
                .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
                .collect();
            recording.set_request_headers(req_headers);

            ctx.recording = Some(recording);
        }

        // Build the upstream peer (plain HTTP to local process)
        let peer = HttpPeer::new(
            format!("127.0.0.1:{}", route.port),
            false, // no TLS to upstream
            String::new(),
        );

        Ok(Box::new(peer))
    }

    /// Modify the request before sending to upstream.
    ///
    /// - Checks network simulation: packet loss (drop request) and latency (inject delay).
    /// - Rewrites the `Host` header to `localhost:<port>` so the upstream app
    ///   sees the correct host.
    /// - Adds `X-Forwarded-For`, `X-Forwarded-Host`, `X-Forwarded-Proto` headers.
    async fn upstream_request_filter(
        &self,
        session: &mut Session,
        upstream_request: &mut RequestHeader,
        ctx: &mut Self::CTX,
    ) -> pingora_core::Result<()> {
        // --- Network simulation: packet loss ---
        if self.network_sim.should_drop(&ctx.app_name, &ctx.path) {
            tracing::debug!(
                app = %ctx.app_name,
                path = %ctx.path,
                "Network sim: dropping request (simulated packet loss)"
            );
            return Err(Error::explain(
                ErrorType::HTTPStatus(503),
                "simulated packet loss",
            ));
        }

        // --- Network simulation: latency injection ---
        if let Some(delay) = self.network_sim.get_delay(&ctx.app_name, &ctx.path) {
            tracing::debug!(
                app = %ctx.app_name,
                path = %ctx.path,
                delay_ms = delay.as_millis(),
                "Network sim: injecting latency"
            );
            tokio::time::sleep(delay).await;
        }

        // Rewrite Host to what the upstream expects
        upstream_request
            .insert_header("Host", &format!("localhost:{}", ctx.target_port))
            .map_err(|e| {
                Error::explain(
                    ErrorType::InternalError,
                    format!("failed to set Host header: {}", e),
                )
            })?;

        // Add forwarding headers
        let original_host = session
            .req_header()
            .headers
            .get("host")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("")
            .to_string();

        let _ = upstream_request.insert_header("X-Forwarded-Host", &original_host);
        let _ = upstream_request.insert_header("X-Forwarded-Proto", "http");

        // Add client IP if available
        if let Some(addr) = session.client_addr() {
            let _ = upstream_request.insert_header("X-Forwarded-For", &addr.to_string());
        }

        Ok(())
    }

    /// Capture response headers from upstream for recording.
    async fn upstream_response_filter(
        &self,
        _session: &mut Session,
        upstream_response: &mut ResponseHeader,
        ctx: &mut Self::CTX,
    ) -> pingora_core::Result<()> {
        if let Some(ref mut recording) = ctx.recording {
            recording.set_response_status(upstream_response.status.as_u16());

            let headers: HashMap<String, String> = upstream_response
                .headers
                .iter()
                .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
                .collect();
            recording.set_response_headers(headers);
        }

        Ok(())
    }

    /// Capture response body chunks for recording, with optional bandwidth throttling.
    fn upstream_response_body_filter(
        &self,
        _session: &mut Session,
        body: &mut Option<Bytes>,
        _end_of_stream: bool,
        ctx: &mut Self::CTX,
    ) -> pingora_core::Result<Option<Duration>> {
        if let Some(ref mut recording) = ctx.recording {
            if let Some(data) = body {
                recording.append_response_body(data);
            }
        }

        // --- Network simulation: bandwidth throttling ---
        // Return a delay proportional to the chunk size to simulate limited bandwidth.
        // Pingora will wait this duration before sending the next chunk.
        if let Some(data) = body {
            if let Some(delay) =
                self.network_sim
                    .throttle_delay(&ctx.app_name, &ctx.path, data.len())
            {
                tracing::debug!(
                    app = %ctx.app_name,
                    path = %ctx.path,
                    chunk_bytes = data.len(),
                    delay_ms = delay.as_millis(),
                    "Network sim: throttling response body"
                );
                return Ok(Some(delay));
            }
        }

        Ok(None)
    }

    /// Final logging: persist the captured request/response to SQLite and
    /// broadcast a completion event via WebSocket.
    async fn logging(&self, _session: &mut Session, _error: Option<&Error>, ctx: &mut Self::CTX) {
        if let Some(recording) = ctx.recording.take() {
            recording.complete_sync();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::router::extract_subdomain;

    #[test]
    fn test_subdomain_routing_logic() {
        // Verify extract_subdomain works as expected for proxy routing
        assert_eq!(extract_subdomain("my-app.localhost:1337"), "my-app");
        assert_eq!(extract_subdomain("api.my-app.localhost:1337"), "api.my-app");
        assert_eq!(extract_subdomain("localhost:1337"), "");
        assert_eq!(extract_subdomain("_portzero.localhost:1337"), "_portzero");
    }

    #[test]
    fn test_request_context_default() {
        let ctx = RequestContext::default();
        assert!(ctx.app_name.is_empty());
        assert_eq!(ctx.target_port, 0);
        assert!(!ctx.mocked);
    }
}
