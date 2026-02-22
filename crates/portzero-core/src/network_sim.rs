//! Network simulation engine: injects realistic network conditions at the
//! proxy layer for testing how applications behave under degraded networks.
//!
//! Supports:
//! - **Latency injection**: fixed delay + random jitter
//! - **Packet loss**: probabilistic request dropping
//! - **Bandwidth throttling**: bytes/sec limit on response bodies
//! - **Path filtering**: apply only to matching request paths
//!
//! Applied in the proxy's `upstream_request_filter` (before connection to upstream)
//! and `upstream_response_body_filter` (for bandwidth throttling).

use crate::types::NetworkProfile;
use dashmap::DashMap;
use rand::Rng;
use std::time::Duration;
use tracing;

/// The network simulation engine.
pub struct NetworkSim {
    /// Active network profiles keyed by app name.
    profiles: DashMap<String, NetworkProfile>,
}

impl NetworkSim {
    /// Create a new network simulation engine.
    pub fn new() -> Self {
        Self {
            profiles: DashMap::new(),
        }
    }

    /// Set a network profile for an app. Replaces any existing profile.
    pub fn set_profile(&self, profile: NetworkProfile) {
        tracing::info!(
            app = %profile.app_name,
            latency_ms = ?profile.latency_ms,
            jitter_ms = ?profile.jitter_ms,
            packet_loss_rate = %profile.packet_loss_rate,
            bandwidth_limit = ?profile.bandwidth_limit,
            path_filter = ?profile.path_filter,
            "Network profile set"
        );
        self.profiles.insert(profile.app_name.clone(), profile);
    }

    /// Remove the network profile for an app.
    pub fn clear_profile(&self, app_name: &str) -> bool {
        let removed = self.profiles.remove(app_name).is_some();
        if removed {
            tracing::info!(app = %app_name, "Network profile cleared");
        }
        removed
    }

    /// Get the network profile for an app.
    pub fn get_profile(&self, app_name: &str) -> Option<NetworkProfile> {
        self.profiles.get(app_name).map(|p| p.value().clone())
    }

    /// List all active profiles.
    pub fn list_profiles(&self) -> Vec<NetworkProfile> {
        self.profiles
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Calculate the delay to inject for a request.
    ///
    /// Returns `None` if no latency is configured for the app, or if the
    /// request path doesn't match the profile's path filter.
    pub fn get_delay(&self, app_name: &str, path: &str) -> Option<Duration> {
        let profile = self.profiles.get(app_name)?;
        let profile = profile.value();

        // Check path filter
        if let Some(ref filter) = profile.path_filter {
            if !path_matches_filter(path, filter) {
                return None;
            }
        }

        let base_latency = profile.latency_ms?;
        let jitter = profile.jitter_ms.unwrap_or(0);

        let delay_ms = if jitter > 0 {
            let mut rng = rand::rng();
            let jitter_val = rng.random_range(0..=(jitter * 2)) as i64 - jitter as i64;
            (base_latency as i64 + jitter_val).max(0) as u64
        } else {
            base_latency
        };

        Some(Duration::from_millis(delay_ms))
    }

    /// Check if a request should be dropped (simulated packet loss).
    ///
    /// Returns `true` if the request should be dropped based on the
    /// configured packet loss rate.
    pub fn should_drop(&self, app_name: &str, path: &str) -> bool {
        let profile = match self.profiles.get(app_name) {
            Some(p) => p,
            None => return false,
        };
        let profile = profile.value();

        if profile.packet_loss_rate <= 0.0 {
            return false;
        }

        // Check path filter
        if let Some(ref filter) = profile.path_filter {
            if !path_matches_filter(path, filter) {
                return false;
            }
        }

        let mut rng = rand::rng();
        let roll: f64 = rng.random();
        roll < profile.packet_loss_rate
    }

    /// Get the bandwidth limit for an app (in bytes/sec).
    ///
    /// Returns `None` if no bandwidth limit is configured.
    pub fn get_bandwidth_limit(&self, app_name: &str, path: &str) -> Option<u64> {
        let profile = self.profiles.get(app_name)?;
        let profile = profile.value();

        // Check path filter
        if let Some(ref filter) = profile.path_filter {
            if !path_matches_filter(path, filter) {
                return None;
            }
        }

        profile.bandwidth_limit
    }

    /// Calculate how long to sleep to throttle a chunk of data to the
    /// configured bandwidth limit.
    ///
    /// Returns `None` if no bandwidth limit is active.
    pub fn throttle_delay(
        &self,
        app_name: &str,
        path: &str,
        chunk_size: usize,
    ) -> Option<Duration> {
        let limit = self.get_bandwidth_limit(app_name, path)?;
        if limit == 0 {
            return None;
        }
        let delay_secs = chunk_size as f64 / limit as f64;
        Some(Duration::from_secs_f64(delay_secs))
    }
}

impl Default for NetworkSim {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Path filter matching
// ---------------------------------------------------------------------------

/// Simple glob-style path matching for network simulation path filters.
fn path_matches_filter(path: &str, filter: &str) -> bool {
    if filter == "*" || filter == "/**" {
        return true;
    }

    if filter == path {
        return true;
    }

    let filter_parts: Vec<&str> = filter.split('/').filter(|s| !s.is_empty()).collect();
    let path_parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    filter_match_parts(&path_parts, &filter_parts)
}

fn filter_match_parts(path: &[&str], pattern: &[&str]) -> bool {
    if pattern.is_empty() {
        return path.is_empty();
    }

    let pat = pattern[0];

    if pat == "**" {
        for i in 0..=path.len() {
            if filter_match_parts(&path[i..], &pattern[1..]) {
                return true;
            }
        }
        return false;
    }

    if path.is_empty() {
        return false;
    }

    if pat == "*" || path[0] == pat {
        return filter_match_parts(&path[1..], &pattern[1..]);
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_profile(app_name: &str) -> NetworkProfile {
        NetworkProfile {
            app_name: app_name.to_string(),
            latency_ms: None,
            jitter_ms: None,
            packet_loss_rate: 0.0,
            bandwidth_limit: None,
            path_filter: None,
        }
    }

    #[test]
    fn test_set_and_get_profile() {
        let sim = NetworkSim::new();
        let mut profile = make_profile("my-app");
        profile.latency_ms = Some(100);

        sim.set_profile(profile);

        let fetched = sim.get_profile("my-app").unwrap();
        assert_eq!(fetched.latency_ms, Some(100));
    }

    #[test]
    fn test_clear_profile() {
        let sim = NetworkSim::new();
        sim.set_profile(make_profile("my-app"));

        assert!(sim.clear_profile("my-app"));
        assert!(!sim.clear_profile("my-app")); // already cleared
        assert!(sim.get_profile("my-app").is_none());
    }

    #[test]
    fn test_list_profiles() {
        let sim = NetworkSim::new();
        sim.set_profile(make_profile("app-a"));
        sim.set_profile(make_profile("app-b"));

        let profiles = sim.list_profiles();
        assert_eq!(profiles.len(), 2);
    }

    #[test]
    fn test_get_delay_no_latency() {
        let sim = NetworkSim::new();
        sim.set_profile(make_profile("my-app"));

        assert!(sim.get_delay("my-app", "/").is_none());
    }

    #[test]
    fn test_get_delay_fixed_latency() {
        let sim = NetworkSim::new();
        let mut profile = make_profile("my-app");
        profile.latency_ms = Some(200);
        sim.set_profile(profile);

        let delay = sim.get_delay("my-app", "/").unwrap();
        assert_eq!(delay, Duration::from_millis(200));
    }

    #[test]
    fn test_get_delay_with_jitter() {
        let sim = NetworkSim::new();
        let mut profile = make_profile("my-app");
        profile.latency_ms = Some(200);
        profile.jitter_ms = Some(50);
        sim.set_profile(profile);

        // With jitter, delay should be 200 +/- 50
        let mut delays = Vec::new();
        for _ in 0..100 {
            let delay = sim.get_delay("my-app", "/").unwrap();
            delays.push(delay.as_millis());
        }

        let min_delay = *delays.iter().min().unwrap();
        let max_delay = *delays.iter().max().unwrap();

        // Should be within [150, 250] range (200 +/- 50)
        assert!(min_delay >= 150, "min_delay={} should be >= 150", min_delay);
        assert!(max_delay <= 250, "max_delay={} should be <= 250", max_delay);
        // With 100 samples, we should see some variation
        assert!(max_delay > min_delay, "Should have jitter variation");
    }

    #[test]
    fn test_should_drop_no_loss() {
        let sim = NetworkSim::new();
        sim.set_profile(make_profile("my-app"));

        // With 0.0 packet loss, should never drop
        for _ in 0..100 {
            assert!(!sim.should_drop("my-app", "/"));
        }
    }

    #[test]
    fn test_should_drop_full_loss() {
        let sim = NetworkSim::new();
        let mut profile = make_profile("my-app");
        profile.packet_loss_rate = 1.0;
        sim.set_profile(profile);

        // With 100% packet loss, should always drop
        for _ in 0..10 {
            assert!(sim.should_drop("my-app", "/"));
        }
    }

    #[test]
    fn test_should_drop_partial_loss() {
        let sim = NetworkSim::new();
        let mut profile = make_profile("my-app");
        profile.packet_loss_rate = 0.5;
        sim.set_profile(profile);

        let mut drops = 0;
        let trials = 1000;
        for _ in 0..trials {
            if sim.should_drop("my-app", "/") {
                drops += 1;
            }
        }

        // Should be roughly 50% +/- 10%
        let rate = drops as f64 / trials as f64;
        assert!(
            rate > 0.3 && rate < 0.7,
            "Drop rate {} should be ~0.5",
            rate
        );
    }

    #[test]
    fn test_path_filter() {
        let sim = NetworkSim::new();
        let mut profile = make_profile("my-app");
        profile.latency_ms = Some(100);
        profile.path_filter = Some("/api/**".to_string());
        sim.set_profile(profile);

        // Should match
        assert!(sim.get_delay("my-app", "/api/users").is_some());
        assert!(sim.get_delay("my-app", "/api/users/123").is_some());

        // Should not match
        assert!(sim.get_delay("my-app", "/static/main.js").is_none());
        assert!(sim.get_delay("my-app", "/").is_none());
    }

    #[test]
    fn test_bandwidth_limit() {
        let sim = NetworkSim::new();
        let mut profile = make_profile("my-app");
        profile.bandwidth_limit = Some(1024); // 1 KB/s
        sim.set_profile(profile);

        assert_eq!(sim.get_bandwidth_limit("my-app", "/"), Some(1024));

        // 1024 bytes at 1024 B/s = 1 second
        let delay = sim.throttle_delay("my-app", "/", 1024).unwrap();
        assert_eq!(delay, Duration::from_secs(1));

        // 512 bytes at 1024 B/s = 0.5 seconds
        let delay = sim.throttle_delay("my-app", "/", 512).unwrap();
        assert_eq!(delay, Duration::from_millis(500));
    }

    #[test]
    fn test_no_profile_returns_none() {
        let sim = NetworkSim::new();
        assert!(sim.get_delay("unknown-app", "/").is_none());
        assert!(!sim.should_drop("unknown-app", "/"));
        assert!(sim.get_bandwidth_limit("unknown-app", "/").is_none());
        assert!(sim.throttle_delay("unknown-app", "/", 100).is_none());
    }

    #[test]
    fn test_path_filter_matching() {
        assert!(path_matches_filter("/api/users", "/api/**"));
        assert!(path_matches_filter("/api/users/123", "/api/**"));
        assert!(!path_matches_filter("/static/main.js", "/api/**"));

        assert!(path_matches_filter("/api/users", "/api/users"));
        assert!(!path_matches_filter("/api/posts", "/api/users"));

        assert!(path_matches_filter("/anything", "*"));
        assert!(path_matches_filter("/any/path/here", "/**"));
    }
}
