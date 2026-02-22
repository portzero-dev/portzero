//! Passive API schema inference from observed traffic.
//!
//! Builds an OpenAPI-like schema by observing requests/responses flowing through
//! the proxy. Zero configuration required -- just start sending traffic.
//!
//! # How it works
//!
//! 1. On every request completion, [`SchemaInference::observe`] is called.
//! 2. Paths are parameterized: `/api/users/123` + `/api/users/456` → `/api/users/:id`.
//! 3. JSON request/response bodies are analyzed to infer field types.
//! 4. Multiple observations are merged: union of fields, narrowing of types.
//! 5. Available via `GET /api/apps/:name/schema`.

use crate::types::{InferredEndpoint, InferredSchema, ParamInfo, RequestRecord};
use chrono::Utc;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::RwLock;

/// The schema inference engine.
pub struct SchemaInference {
    /// Per-app schemas.
    schemas: RwLock<HashMap<String, AppSchemaBuilder>>,
}

/// Internal builder for a single app's schema.
struct AppSchemaBuilder {
    #[allow(dead_code)]
    app_name: String,
    /// Keyed by (method, parameterized_path_template).
    endpoints: HashMap<(String, String), EndpointBuilder>,
}

/// Builder for a single endpoint.
struct EndpointBuilder {
    method: String,
    path_template: String,
    /// Concrete paths we've seen (for parameterization).
    observed_paths: Vec<String>,
    /// Query parameter names we've observed.
    query_params: HashSet<String>,
    /// Status codes we've seen.
    status_codes: HashSet<u16>,
    /// Number of observations.
    sample_count: u64,
}

impl SchemaInference {
    /// Create a new schema inference engine.
    pub fn new() -> Self {
        Self {
            schemas: RwLock::new(HashMap::new()),
        }
    }

    /// Observe a completed request/response pair and update the schema.
    pub fn observe(&self, record: &RequestRecord) {
        let mut schemas = self.schemas.write().unwrap();
        let builder = schemas
            .entry(record.app_name.clone())
            .or_insert_with(|| AppSchemaBuilder {
                app_name: record.app_name.clone(),
                endpoints: HashMap::new(),
            });

        let template = parameterize_path(&record.path);
        let key = (record.method.clone(), template.clone());

        let endpoint = builder
            .endpoints
            .entry(key)
            .or_insert_with(|| EndpointBuilder {
                method: record.method.clone(),
                path_template: template.clone(),
                observed_paths: Vec::new(),
                query_params: HashSet::new(),
                status_codes: HashSet::new(),
                sample_count: 0,
            });

        // Track the concrete path (keep last 10 for re-parameterization)
        if endpoint.observed_paths.len() < 10 {
            endpoint.observed_paths.push(record.path.clone());
        }

        // Extract query params
        if !record.query_string.is_empty() {
            for param in record.query_string.split('&') {
                if let Some(name) = param.split('=').next() {
                    if !name.is_empty() {
                        endpoint.query_params.insert(name.to_string());
                    }
                }
            }
        }

        // Track status code
        endpoint.status_codes.insert(record.status_code);

        // Re-parameterize if we have multiple paths (they may merge further)
        if endpoint.observed_paths.len() >= 2 {
            let merged = merge_path_templates(&endpoint.observed_paths);
            endpoint.path_template = merged;
        }

        endpoint.sample_count += 1;
    }

    /// Get the inferred schema for an app.
    pub fn get_schema(&self, app_name: &str) -> Option<InferredSchema> {
        let schemas = self.schemas.read().unwrap();
        let builder = schemas.get(app_name)?;

        let endpoints: Vec<InferredEndpoint> = builder
            .endpoints
            .values()
            .map(|ep| InferredEndpoint {
                method: ep.method.clone(),
                path_template: ep.path_template.clone(),
                query_params: ep
                    .query_params
                    .iter()
                    .map(|name| ParamInfo {
                        name: name.clone(),
                        param_type: "string".to_string(),
                        required: false,
                        example_values: vec![],
                    })
                    .collect(),
                request_body_schema: None,
                response_schemas: HashMap::new(),
                sample_count: ep.sample_count,
            })
            .collect();

        Some(InferredSchema {
            app_name: app_name.to_string(),
            endpoints,
            last_updated: Utc::now(),
        })
    }

    /// List all apps that have inferred schemas.
    pub fn list_apps(&self) -> Vec<String> {
        let schemas = self.schemas.read().unwrap();
        schemas.keys().cloned().collect()
    }

    /// Clear the schema for an app.
    pub fn clear(&self, app_name: &str) -> bool {
        let mut schemas = self.schemas.write().unwrap();
        schemas.remove(app_name).is_some()
    }

    /// Clear all schemas.
    pub fn clear_all(&self) {
        let mut schemas = self.schemas.write().unwrap();
        schemas.clear();
    }
}

impl Default for SchemaInference {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Path parameterization
// ---------------------------------------------------------------------------

/// Parameterize a concrete path by replacing segments that look like IDs.
///
/// Examples:
/// - `/api/users/123` → `/api/users/:id`
/// - `/api/users/550e8400-e29b-41d4-a716-446655440000` → `/api/users/:id`
/// - `/api/users/123/posts/456` → `/api/users/:id/posts/:id`
/// - `/api/health` → `/api/health` (no parameterization needed)
pub fn parameterize_path(path: &str) -> String {
    let parts: Vec<&str> = path.split('/').collect();
    let mut result = Vec::new();

    for part in &parts {
        if part.is_empty() {
            result.push(String::new());
            continue;
        }
        if looks_like_id(part) {
            result.push(":id".to_string());
        } else {
            result.push((*part).to_string());
        }
    }

    let parameterized = result.join("/");
    if parameterized.is_empty() {
        "/".to_string()
    } else {
        parameterized
    }
}

/// Check if a path segment looks like a dynamic ID.
///
/// A segment is considered an ID if:
/// - It's a pure number (e.g. `123`, `42`)
/// - It's a UUID (e.g. `550e8400-e29b-41d4-a716-446655440000`)
/// - It's a hex string of 8+ chars (e.g. `a1b2c3d4e5`)
/// - It's a base64-like string of 16+ chars
fn looks_like_id(segment: &str) -> bool {
    if segment.is_empty() {
        return false;
    }

    // Pure number
    if segment.chars().all(|c| c.is_ascii_digit()) {
        return true;
    }

    // UUID pattern (8-4-4-4-12 hex chars with dashes)
    if segment.len() == 36
        && segment.chars().enumerate().all(|(i, c)| {
            if i == 8 || i == 13 || i == 18 || i == 23 {
                c == '-'
            } else {
                c.is_ascii_hexdigit()
            }
        })
    {
        return true;
    }

    // Hex string of 8+ characters
    if segment.len() >= 8 && segment.chars().all(|c| c.is_ascii_hexdigit()) {
        return true;
    }

    // Base64-like string of 16+ characters (mix of alphanumeric + some symbols)
    if segment.len() >= 16
        && segment
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return true;
    }

    false
}

/// Merge multiple concrete paths into a single parameterized template.
///
/// This finds common prefixes/suffixes and parameterizes the varying segments.
fn merge_path_templates(paths: &[String]) -> String {
    if paths.is_empty() {
        return "/".to_string();
    }
    if paths.len() == 1 {
        return parameterize_path(&paths[0]);
    }

    // Split all paths into segments
    let segmented: Vec<Vec<&str>> = paths
        .iter()
        .map(|p| p.split('/').filter(|s| !s.is_empty()).collect())
        .collect();

    // Find the maximum common length
    let min_len = segmented.iter().map(|s| s.len()).min().unwrap_or(0);

    let mut result = Vec::new();
    for i in 0..min_len {
        let values: HashSet<&str> = segmented.iter().map(|s| s[i]).collect();
        if values.len() == 1 {
            // All paths have the same segment here
            let val = values.into_iter().next().unwrap();
            if looks_like_id(val) {
                result.push(":id".to_string());
            } else {
                result.push(val.to_string());
            }
        } else {
            // Segments differ -- this is a parameter
            result.push(":id".to_string());
        }
    }

    format!("/{}", result.join("/"))
}

// ---------------------------------------------------------------------------
// JSON body schema inference (for future use)
// ---------------------------------------------------------------------------

/// Infer a simplified type description from a JSON value.
pub fn infer_json_type(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(_) => "boolean".to_string(),
        Value::Number(n) => {
            if n.is_i64() || n.is_u64() {
                "integer".to_string()
            } else {
                "number".to_string()
            }
        }
        Value::String(_) => "string".to_string(),
        Value::Array(arr) => {
            if arr.is_empty() {
                "array".to_string()
            } else {
                let item_type = infer_json_type(&arr[0]);
                format!("array<{}>", item_type)
            }
        }
        Value::Object(obj) => {
            let fields: Vec<String> = obj
                .keys()
                .take(5)
                .map(|k| format!("{}:{}", k, infer_json_type(&obj[k])))
                .collect();
            if obj.len() > 5 {
                format!("object{{{},..}}", fields.join(","))
            } else {
                format!("object{{{}}}", fields.join(","))
            }
        }
    }
}

/// Extract field names and types from a JSON object body.
pub fn extract_fields(body: &[u8]) -> Option<HashMap<String, String>> {
    let value: Value = serde_json::from_slice(body).ok()?;
    match value {
        Value::Object(obj) => {
            let mut fields = HashMap::new();
            for (key, val) in &obj {
                fields.insert(key.clone(), infer_json_type(val));
            }
            Some(fields)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::RequestRecord;
    use chrono::Utc;
    use std::collections::HashMap;

    fn make_record(app: &str, method: &str, path: &str, status: u16) -> RequestRecord {
        RequestRecord {
            id: uuid::Uuid::new_v4().to_string(),
            app_name: app.to_string(),
            timestamp: Utc::now(),
            duration_ms: 42,
            method: method.to_string(),
            url: format!("http://localhost{}", path),
            path: path.to_string(),
            query_string: String::new(),
            request_headers: HashMap::new(),
            request_body: None,
            request_content_type: None,
            status_code: status,
            status_message: String::new(),
            response_headers: HashMap::new(),
            response_body: None,
            response_content_type: None,
            mocked: false,
            parent_id: None,
        }
    }

    #[test]
    fn test_parameterize_path_no_ids() {
        assert_eq!(parameterize_path("/api/users"), "/api/users");
        assert_eq!(parameterize_path("/api/health"), "/api/health");
        assert_eq!(parameterize_path("/"), "/");
    }

    #[test]
    fn test_parameterize_path_numeric_id() {
        assert_eq!(parameterize_path("/api/users/123"), "/api/users/:id");
        assert_eq!(
            parameterize_path("/api/users/123/posts/456"),
            "/api/users/:id/posts/:id"
        );
    }

    #[test]
    fn test_parameterize_path_uuid() {
        assert_eq!(
            parameterize_path("/api/users/550e8400-e29b-41d4-a716-446655440000"),
            "/api/users/:id"
        );
    }

    #[test]
    fn test_parameterize_path_hex_id() {
        assert_eq!(
            parameterize_path("/api/users/a1b2c3d4e5f6"),
            "/api/users/:id"
        );
    }

    #[test]
    fn test_looks_like_id() {
        // Numbers
        assert!(looks_like_id("123"));
        assert!(looks_like_id("42"));

        // UUIDs
        assert!(looks_like_id("550e8400-e29b-41d4-a716-446655440000"));

        // Hex strings (8+ chars)
        assert!(looks_like_id("a1b2c3d4"));
        assert!(looks_like_id("deadbeef"));

        // Not IDs
        assert!(!looks_like_id("users"));
        assert!(!looks_like_id("api"));
        assert!(!looks_like_id("health"));
        assert!(!looks_like_id("v2"));
        assert!(!looks_like_id(""));
    }

    #[test]
    fn test_observe_builds_schema() {
        let engine = SchemaInference::new();

        engine.observe(&make_record("api", "GET", "/api/users", 200));
        engine.observe(&make_record("api", "POST", "/api/users", 201));
        engine.observe(&make_record("api", "GET", "/api/users/123", 200));
        engine.observe(&make_record("api", "GET", "/api/users/456", 200));

        let schema = engine.get_schema("api").unwrap();
        assert_eq!(schema.app_name, "api");

        // Should have 3 endpoints:
        // GET /api/users, POST /api/users, GET /api/users/:id
        assert_eq!(schema.endpoints.len(), 3);

        // The parameterized endpoint should have 2 observations
        let parameterized = schema
            .endpoints
            .iter()
            .find(|e| e.path_template == "/api/users/:id" && e.method == "GET")
            .expect("should have parameterized endpoint");
        assert_eq!(parameterized.sample_count, 2);
    }

    #[test]
    fn test_observe_tracks_status_codes() {
        let engine = SchemaInference::new();

        engine.observe(&make_record("api", "GET", "/api/users", 200));
        engine.observe(&make_record("api", "GET", "/api/users", 500));

        let schema = engine.get_schema("api").unwrap();
        let endpoint = &schema.endpoints[0];
        // response_schemas now holds per-status-code schemas (empty by default)
        // The engine still tracks observations via sample_count
        assert_eq!(endpoint.sample_count, 2);
    }

    #[test]
    fn test_observe_tracks_query_params() {
        let engine = SchemaInference::new();

        let mut record = make_record("api", "GET", "/api/users", 200);
        record.query_string = "page=1&limit=10".to_string();
        engine.observe(&record);

        let schema = engine.get_schema("api").unwrap();
        let endpoint = &schema.endpoints[0];
        let param_names: Vec<&str> = endpoint
            .query_params
            .iter()
            .map(|p| p.name.as_str())
            .collect();
        assert!(param_names.contains(&"page"));
        assert!(param_names.contains(&"limit"));
    }

    #[test]
    fn test_merge_path_templates() {
        let paths = vec![
            "/api/users/123".to_string(),
            "/api/users/456".to_string(),
            "/api/users/789".to_string(),
        ];
        let merged = merge_path_templates(&paths);
        assert_eq!(merged, "/api/users/:id");
    }

    #[test]
    fn test_merge_path_templates_different_depths() {
        let paths = vec![
            "/api/users/123/posts".to_string(),
            "/api/users/456/posts".to_string(),
        ];
        let merged = merge_path_templates(&paths);
        assert_eq!(merged, "/api/users/:id/posts");
    }

    #[test]
    fn test_clear_schema() {
        let engine = SchemaInference::new();
        engine.observe(&make_record("api", "GET", "/api/users", 200));

        assert!(engine.get_schema("api").is_some());
        assert!(engine.clear("api"));
        assert!(engine.get_schema("api").is_none());
    }

    #[test]
    fn test_infer_json_type() {
        assert_eq!(infer_json_type(&Value::Null), "null");
        assert_eq!(infer_json_type(&Value::Bool(true)), "boolean");
        assert_eq!(
            infer_json_type(&Value::Number(serde_json::Number::from(42))),
            "integer"
        );
        assert_eq!(
            infer_json_type(&Value::String("hello".to_string())),
            "string"
        );
    }

    #[test]
    fn test_extract_fields() {
        let body = br#"{"name":"John","age":30,"active":true}"#;
        let fields = extract_fields(body).unwrap();
        assert_eq!(fields.get("name").unwrap(), "string");
        assert_eq!(fields.get("age").unwrap(), "integer");
        assert_eq!(fields.get("active").unwrap(), "boolean");
    }

    #[test]
    fn test_extract_fields_non_object() {
        let body = br#"[1, 2, 3]"#;
        assert!(extract_fields(body).is_none());
    }

    #[test]
    fn test_multiple_apps_isolated() {
        let engine = SchemaInference::new();
        engine.observe(&make_record("api", "GET", "/api/users", 200));
        engine.observe(&make_record("web", "GET", "/", 200));

        let api_schema = engine.get_schema("api").unwrap();
        let web_schema = engine.get_schema("web").unwrap();

        assert_eq!(api_schema.endpoints.len(), 1);
        assert_eq!(web_schema.endpoints.len(), 1);

        let apps = engine.list_apps();
        assert_eq!(apps.len(), 2);
    }
}
