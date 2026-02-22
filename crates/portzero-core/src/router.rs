//! Subdomain → port routing table.
//!
//! Thread-safe routing table behind `RwLock`. Read-heavy workload: every
//! proxied request reads the table, writes only happen on app register/deregister.

use crate::types::{AppStatus, Route};
use chrono::Utc;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::RwLock;

use crate::types::{PORT_RANGE_BASE, PORT_RANGE_SIZE};

/// Thread-safe routing table.
pub struct Router {
    routes: RwLock<Vec<Route>>,
}

impl Router {
    pub fn new() -> Self {
        Self {
            routes: RwLock::new(Vec::new()),
        }
    }

    /// Resolve a subdomain to a route.
    ///
    /// Supports nested subdomains by longest-suffix match:
    /// "api.my-app" is checked first, then "my-app".
    pub fn resolve(&self, subdomain: &str) -> Option<Route> {
        let routes = self.routes.read().unwrap();

        // Exact match first
        if let Some(route) = routes.iter().find(|r| r.hostname == subdomain) {
            if route.status.is_running() {
                return Some(route.clone());
            }
        }

        // Try parent subdomains: "api.my-app" → "my-app"
        let mut parts: &str = subdomain;
        while let Some(dot_pos) = parts.find('.') {
            parts = &parts[dot_pos + 1..];
            if let Some(route) = routes.iter().find(|r| r.hostname == parts) {
                if route.status.is_running() {
                    return Some(route.clone());
                }
            }
        }

        None
    }

    /// Register a new app route.
    pub fn register(
        &self,
        hostname: String,
        port: u16,
        pid: u32,
        command: Vec<String>,
        cwd: PathBuf,
    ) -> Route {
        let route = Route {
            hostname: hostname.clone(),
            port,
            pid,
            command,
            cwd,
            started_at: Utc::now(),
            status: AppStatus::Running,
        };

        let mut routes = self.routes.write().unwrap();
        // Remove any existing route with the same hostname
        routes.retain(|r| r.hostname != hostname);
        routes.push(route.clone());
        route
    }

    /// Remove a route by hostname.
    pub fn deregister(&self, hostname: &str) -> Option<Route> {
        let mut routes = self.routes.write().unwrap();
        let pos = routes.iter().position(|r| r.hostname == hostname)?;
        Some(routes.remove(pos))
    }

    /// Update the status of a route.
    pub fn update_status(&self, hostname: &str, status: AppStatus) {
        let mut routes = self.routes.write().unwrap();
        if let Some(route) = routes.iter_mut().find(|r| r.hostname == hostname) {
            route.status = status;
        }
    }

    /// Update the PID of a route (after restart).
    pub fn update_pid(&self, hostname: &str, pid: u32) {
        let mut routes = self.routes.write().unwrap();
        if let Some(route) = routes.iter_mut().find(|r| r.hostname == hostname) {
            route.pid = pid;
            route.status = AppStatus::Running;
            route.started_at = Utc::now();
        }
    }

    /// List all registered routes.
    pub fn list(&self) -> Vec<Route> {
        self.routes.read().unwrap().clone()
    }

    /// Get a route by hostname.
    pub fn get(&self, hostname: &str) -> Option<Route> {
        self.routes
            .read()
            .unwrap()
            .iter()
            .find(|r| r.hostname == hostname)
            .cloned()
    }

    /// Get the port for a running app by name. Returns None if not found or not running.
    pub fn get_port(&self, app_name: &str) -> Option<u16> {
        self.routes
            .read()
            .unwrap()
            .iter()
            .find(|r| r.hostname == app_name && r.status.is_running())
            .map(|r| r.port)
    }

    /// Find a free port using deterministic hash + fallback.
    ///
    /// Hashes the app name to pick a port in the range [PORT_RANGE_BASE, PORT_RANGE_BASE + PORT_RANGE_SIZE).
    /// If that port is taken, linearly probes until a free one is found.
    pub fn find_free_port(&self, app_name: &str) -> u16 {
        let mut hasher = DefaultHasher::new();
        app_name.hash(&mut hasher);
        let hash = hasher.finish();
        let base_offset = (hash % PORT_RANGE_SIZE as u64) as u16;

        let routes = self.routes.read().unwrap();
        let used_ports: Vec<u16> = routes.iter().map(|r| r.port).collect();

        for i in 0..PORT_RANGE_SIZE {
            let port = PORT_RANGE_BASE + ((base_offset + i) % PORT_RANGE_SIZE);
            if !used_ports.contains(&port) && is_port_available(port) {
                return port;
            }
        }

        // Fallback: let the OS pick
        0
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a TCP port is available by attempting to bind to it.
fn is_port_available(port: u16) -> bool {
    TcpListener::bind(("127.0.0.1", port)).is_ok()
}

/// Extract the subdomain from a Host header value.
///
/// Examples:
/// - "my-app.localhost:1337" → "my-app"
/// - "api.my-app.localhost:1337" → "api.my-app"
/// - "localhost:1337" → ""
/// - "_portzero.localhost:1337" → "_portzero"
pub fn extract_subdomain(host: &str) -> &str {
    // Strip port
    let host_no_port = host.split(':').next().unwrap_or(host);

    // Strip ".localhost" suffix
    if let Some(prefix) = host_no_port.strip_suffix(".localhost") {
        prefix
    } else {
        // Not a .localhost domain, return empty
        ""
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_subdomain() {
        assert_eq!(extract_subdomain("my-app.localhost:1337"), "my-app");
        assert_eq!(extract_subdomain("api.my-app.localhost:1337"), "api.my-app");
        assert_eq!(extract_subdomain("localhost:1337"), "");
        assert_eq!(extract_subdomain("_portzero.localhost:1337"), "_portzero");
        assert_eq!(extract_subdomain("my-app.localhost"), "my-app");
    }

    #[test]
    fn test_register_and_resolve() {
        let router = Router::new();
        router.register(
            "my-app".to_string(),
            4001,
            1234,
            vec!["next".to_string(), "dev".to_string()],
            PathBuf::from("/tmp"),
        );

        let route = router.resolve("my-app").unwrap();
        assert_eq!(route.port, 4001);
        assert_eq!(route.pid, 1234);
    }

    #[test]
    fn test_nested_subdomain_resolution() {
        let router = Router::new();
        router.register(
            "my-app".to_string(),
            4001,
            1234,
            vec!["next".to_string(), "dev".to_string()],
            PathBuf::from("/tmp"),
        );

        // "api.my-app" should fall back to "my-app"
        let route = router.resolve("api.my-app").unwrap();
        assert_eq!(route.port, 4001);
    }

    #[test]
    fn test_deregister() {
        let router = Router::new();
        router.register(
            "my-app".to_string(),
            4001,
            1234,
            vec![],
            PathBuf::from("/tmp"),
        );

        assert!(router.resolve("my-app").is_some());
        router.deregister("my-app");
        assert!(router.resolve("my-app").is_none());
    }

    #[test]
    fn test_update_status() {
        let router = Router::new();
        router.register(
            "my-app".to_string(),
            4001,
            1234,
            vec![],
            PathBuf::from("/tmp"),
        );

        router.update_status(
            "my-app",
            AppStatus::Crashed {
                exit_code: 1,
                at: Utc::now(),
            },
        );

        // Crashed apps should not resolve
        assert!(router.resolve("my-app").is_none());

        // But should still be in the list
        assert_eq!(router.list().len(), 1);
    }

    #[test]
    fn test_find_free_port_deterministic() {
        let router = Router::new();
        let port1 = router.find_free_port("my-app");
        let port2 = router.find_free_port("my-app");
        assert_eq!(port1, port2); // Same name → same port

        let port3 = router.find_free_port("other-app");
        // Different name → likely different port (could collide but very unlikely)
        assert!(port1 >= PORT_RANGE_BASE && port1 < PORT_RANGE_BASE + PORT_RANGE_SIZE);
        assert!(port3 >= PORT_RANGE_BASE && port3 < PORT_RANGE_BASE + PORT_RANGE_SIZE);
    }
}
