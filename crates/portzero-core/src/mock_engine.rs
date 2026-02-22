//! Mock engine: returns synthetic responses for matching requests without
//! hitting the upstream application.
//!
//! Mocks are evaluated in the proxy's `request_filter` callback before the
//! request is forwarded. If a mock matches, the response is written directly
//! and `Ok(true)` is returned (response already sent).
//!
//! # Priority
//!
//! Mocks are evaluated in the order they were added. The first enabled mock
//! that matches wins. This lets users create broad catch-all mocks and then
//! add more specific ones before them.

use crate::types::{MockResponse, MockRule, WsEvent};
use crate::ws::WsHub;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;
use tracing;
use uuid::Uuid;

/// The mock engine.
pub struct MockEngine {
    /// Active mock rules (evaluated in order).
    mocks: RwLock<Vec<MockRuleEntry>>,
    /// WebSocket hub for broadcasting mock hit events.
    ws_hub: Option<WsHub>,
}

/// Internal entry wrapping a MockRule with an atomic hit counter.
struct MockRuleEntry {
    rule: MockRule,
    hit_count: AtomicU64,
}

impl MockEngine {
    /// Create a new mock engine.
    pub fn new(ws_hub: Option<WsHub>) -> Self {
        Self {
            mocks: RwLock::new(Vec::new()),
            ws_hub,
        }
    }

    // -----------------------------------------------------------------------
    // CRUD
    // -----------------------------------------------------------------------

    /// Add a new mock rule. Returns the rule with a generated ID.
    pub fn add_mock(
        &self,
        app_name: String,
        method: Option<String>,
        path_pattern: String,
        status_code: u16,
        response_headers: HashMap<String, String>,
        response_body: String,
    ) -> MockRule {
        let rule = MockRule {
            id: Uuid::new_v4().to_string(),
            app_name,
            method,
            path_pattern,
            status_code,
            response_headers,
            response_body,
            enabled: true,
            hit_count: 0,
        };
        let mut mocks = self.mocks.write().unwrap();
        mocks.push(MockRuleEntry {
            rule: rule.clone(),
            hit_count: AtomicU64::new(0),
        });
        rule
    }

    /// Add a pre-built mock rule (e.g. loaded from the database).
    pub fn add_mock_raw(&self, rule: MockRule) {
        let hit_count = rule.hit_count;
        let mut mocks = self.mocks.write().unwrap();
        mocks.push(MockRuleEntry {
            rule,
            hit_count: AtomicU64::new(hit_count),
        });
    }

    /// Remove a mock rule by ID. Returns whether it was found.
    pub fn remove_mock(&self, id: &str) -> bool {
        let mut mocks = self.mocks.write().unwrap();
        let before = mocks.len();
        mocks.retain(|entry| entry.rule.id != id);
        mocks.len() < before
    }

    /// Update an existing mock rule. Returns whether it was found.
    pub fn update_mock(
        &self,
        id: &str,
        method: Option<Option<String>>,
        path_pattern: Option<String>,
        status_code: Option<u16>,
        response_headers: Option<HashMap<String, String>>,
        response_body: Option<String>,
        enabled: Option<bool>,
    ) -> Option<MockRule> {
        let mut mocks = self.mocks.write().unwrap();
        for entry in mocks.iter_mut() {
            if entry.rule.id == id {
                if let Some(m) = method {
                    entry.rule.method = m;
                }
                if let Some(p) = path_pattern {
                    entry.rule.path_pattern = p;
                }
                if let Some(s) = status_code {
                    entry.rule.status_code = s;
                }
                if let Some(h) = response_headers {
                    entry.rule.response_headers = h;
                }
                if let Some(b) = response_body {
                    entry.rule.response_body = b;
                }
                if let Some(e) = enabled {
                    entry.rule.enabled = e;
                }
                return Some(entry.rule.clone());
            }
        }
        None
    }

    /// Toggle a mock's enabled state. Returns the new state, or None if not found.
    pub fn toggle_mock(&self, id: &str) -> Option<bool> {
        let mut mocks = self.mocks.write().unwrap();
        for entry in mocks.iter_mut() {
            if entry.rule.id == id {
                entry.rule.enabled = !entry.rule.enabled;
                return Some(entry.rule.enabled);
            }
        }
        None
    }

    /// List all mock rules with current hit counts.
    pub fn list_mocks(&self) -> Vec<MockRule> {
        let mocks = self.mocks.read().unwrap();
        mocks
            .iter()
            .map(|entry| {
                let mut rule = entry.rule.clone();
                rule.hit_count = entry.hit_count.load(Ordering::Relaxed);
                rule
            })
            .collect()
    }

    /// List mock rules for a specific app.
    pub fn list_mocks_for_app(&self, app_name: &str) -> Vec<MockRule> {
        let mocks = self.mocks.read().unwrap();
        mocks
            .iter()
            .filter(|entry| entry.rule.app_name == app_name)
            .map(|entry| {
                let mut rule = entry.rule.clone();
                rule.hit_count = entry.hit_count.load(Ordering::Relaxed);
                rule
            })
            .collect()
    }

    /// Get a specific mock by ID.
    pub fn get_mock(&self, id: &str) -> Option<MockRule> {
        let mocks = self.mocks.read().unwrap();
        mocks.iter().find(|e| e.rule.id == id).map(|entry| {
            let mut rule = entry.rule.clone();
            rule.hit_count = entry.hit_count.load(Ordering::Relaxed);
            rule
        })
    }

    // -----------------------------------------------------------------------
    // Request matching (called from the proxy)
    // -----------------------------------------------------------------------

    /// Try to match a request against the mock rules.
    ///
    /// Returns a `MockResponse` if a mock matches, `None` otherwise.
    /// Also increments the hit counter and broadcasts a `MockHit` event.
    pub fn match_request(
        &self,
        app_name: &str,
        method: &str,
        path: &str,
        request_id: &str,
    ) -> Option<MockResponse> {
        let mocks = self.mocks.read().unwrap();
        for entry in mocks.iter() {
            if !entry.rule.enabled {
                continue;
            }
            if entry.rule.app_name != app_name {
                continue;
            }
            if let Some(ref m) = entry.rule.method {
                if m != method {
                    continue;
                }
            }
            if !path_matches_mock_pattern(path, &entry.rule.path_pattern) {
                continue;
            }

            // Match found!
            entry.hit_count.fetch_add(1, Ordering::Relaxed);

            tracing::debug!(
                mock_id = %entry.rule.id,
                app = %app_name,
                method = %method,
                path = %path,
                "Mock matched"
            );

            // Broadcast hit event
            if let Some(ref hub) = self.ws_hub {
                hub.broadcast(WsEvent::MockHit {
                    mock_id: entry.rule.id.clone(),
                    request_id: request_id.to_string(),
                });
            }

            return Some(MockResponse {
                status_code: entry.rule.status_code,
                headers: entry.rule.response_headers.clone(),
                body: entry.rule.response_body.clone(),
            });
        }
        None
    }
}

// ---------------------------------------------------------------------------
// Path matching for mocks
// ---------------------------------------------------------------------------

/// Match a request path against a mock's path pattern.
///
/// Supports:
/// - Exact match: `/api/users` matches `/api/users`
/// - Wildcard: `/api/users/*` matches `/api/users/123`
/// - Double wildcard: `/api/**` matches `/api/users/123/posts`
/// - Prefix: `/api/users` matches `/api/users?foo=bar` (query is stripped before matching)
fn path_matches_mock_pattern(path: &str, pattern: &str) -> bool {
    // Strip query string from path if present
    let path = path.split('?').next().unwrap_or(path);

    if pattern == path {
        return true;
    }

    if !pattern.contains('*') {
        return path == pattern;
    }

    let pattern_parts: Vec<&str> = pattern.split('/').filter(|s| !s.is_empty()).collect();
    let path_parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    mock_match_parts(&path_parts, &pattern_parts)
}

fn mock_match_parts(path: &[&str], pattern: &[&str]) -> bool {
    if pattern.is_empty() {
        return path.is_empty();
    }

    let pat = pattern[0];

    if pat == "**" {
        for i in 0..=path.len() {
            if mock_match_parts(&path[i..], &pattern[1..]) {
                return true;
            }
        }
        return false;
    }

    if path.is_empty() {
        return false;
    }

    if pat == "*" || path[0] == pat {
        return mock_match_parts(&path[1..], &pattern[1..]);
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_path_match() {
        assert!(path_matches_mock_pattern("/api/users", "/api/users"));
        assert!(!path_matches_mock_pattern("/api/users", "/api/posts"));
    }

    #[test]
    fn test_wildcard_path_match() {
        assert!(path_matches_mock_pattern("/api/users/123", "/api/users/*"));
        assert!(path_matches_mock_pattern("/api/users/abc", "/api/users/*"));
        assert!(!path_matches_mock_pattern(
            "/api/users/123/posts",
            "/api/users/*"
        ));
    }

    #[test]
    fn test_double_wildcard_match() {
        assert!(path_matches_mock_pattern("/api/users/123", "/api/**"));
        assert!(path_matches_mock_pattern("/api/users/123/posts", "/api/**"));
        assert!(path_matches_mock_pattern("/api", "/api/**"));
    }

    #[test]
    fn test_query_string_stripped() {
        assert!(path_matches_mock_pattern("/api/users?page=1", "/api/users"));
    }

    #[test]
    fn test_add_and_match_mock() {
        let engine = MockEngine::new(None);
        engine.add_mock(
            "api".to_string(),
            Some("POST".to_string()),
            "/api/payments".to_string(),
            500,
            HashMap::new(),
            r#"{"error":"declined"}"#.to_string(),
        );

        // Should match
        let resp = engine.match_request("api", "POST", "/api/payments", "req-1");
        assert!(resp.is_some());
        let resp = resp.unwrap();
        assert_eq!(resp.status_code, 500);
        assert_eq!(resp.body, r#"{"error":"declined"}"#);

        // Wrong method
        let resp = engine.match_request("api", "GET", "/api/payments", "req-2");
        assert!(resp.is_none());

        // Wrong app
        let resp = engine.match_request("web", "POST", "/api/payments", "req-3");
        assert!(resp.is_none());

        // Wrong path
        let resp = engine.match_request("api", "POST", "/api/users", "req-4");
        assert!(resp.is_none());
    }

    #[test]
    fn test_mock_hit_counting() {
        let engine = MockEngine::new(None);
        engine.add_mock(
            "api".to_string(),
            None,
            "/api/health".to_string(),
            200,
            HashMap::new(),
            r#"{"ok":true}"#.to_string(),
        );

        engine.match_request("api", "GET", "/api/health", "r1");
        engine.match_request("api", "GET", "/api/health", "r2");
        engine.match_request("api", "GET", "/api/health", "r3");

        let mocks = engine.list_mocks();
        assert_eq!(mocks.len(), 1);
        assert_eq!(mocks[0].hit_count, 3);
    }

    #[test]
    fn test_disabled_mock_not_matched() {
        let engine = MockEngine::new(None);
        let mock = engine.add_mock(
            "api".to_string(),
            None,
            "/api/health".to_string(),
            200,
            HashMap::new(),
            "ok".to_string(),
        );

        engine.toggle_mock(&mock.id);

        let resp = engine.match_request("api", "GET", "/api/health", "r1");
        assert!(resp.is_none());
    }

    #[test]
    fn test_mock_crud() {
        let engine = MockEngine::new(None);

        // Add
        let m1 = engine.add_mock(
            "api".to_string(),
            None,
            "/api/a".to_string(),
            200,
            HashMap::new(),
            "a".to_string(),
        );
        let m2 = engine.add_mock(
            "api".to_string(),
            None,
            "/api/b".to_string(),
            200,
            HashMap::new(),
            "b".to_string(),
        );

        assert_eq!(engine.list_mocks().len(), 2);

        // Update
        let updated = engine.update_mock(
            &m1.id,
            None,
            None,
            Some(404),
            None,
            Some("not found".to_string()),
            None,
        );
        assert!(updated.is_some());
        let updated = updated.unwrap();
        assert_eq!(updated.status_code, 404);
        assert_eq!(updated.response_body, "not found");

        // Get
        let fetched = engine.get_mock(&m1.id).unwrap();
        assert_eq!(fetched.status_code, 404);

        // Delete
        assert!(engine.remove_mock(&m2.id));
        assert_eq!(engine.list_mocks().len(), 1);

        // Delete non-existent
        assert!(!engine.remove_mock("non-existent"));
    }

    #[test]
    fn test_first_match_wins() {
        let engine = MockEngine::new(None);

        // Add broad catch-all first
        engine.add_mock(
            "api".to_string(),
            None,
            "/api/**".to_string(),
            200,
            HashMap::new(),
            "catch-all".to_string(),
        );

        // Add specific mock second (won't match because catch-all is first)
        engine.add_mock(
            "api".to_string(),
            Some("POST".to_string()),
            "/api/users".to_string(),
            201,
            HashMap::new(),
            "specific".to_string(),
        );

        let resp = engine
            .match_request("api", "POST", "/api/users", "r1")
            .unwrap();
        assert_eq!(resp.body, "catch-all");
    }

    #[test]
    fn test_wildcard_mock() {
        let engine = MockEngine::new(None);
        engine.add_mock(
            "api".to_string(),
            Some("GET".to_string()),
            "/api/users/*".to_string(),
            200,
            {
                let mut h = HashMap::new();
                h.insert("Content-Type".to_string(), "application/json".to_string());
                h
            },
            r#"{"id":1,"name":"Test"}"#.to_string(),
        );

        let resp = engine
            .match_request("api", "GET", "/api/users/123", "r1")
            .unwrap();
        assert_eq!(resp.status_code, 200);
        assert_eq!(
            resp.headers.get("Content-Type").unwrap(),
            "application/json"
        );

        // Nested path should not match single wildcard
        let resp = engine.match_request("api", "GET", "/api/users/123/posts", "r2");
        assert!(resp.is_none());
    }
}
