//! SQLite persistence layer for PortZero.
//!
//! Uses rusqlite with WAL mode for concurrent reads from the API server
//! while the proxy writes captured requests.

use crate::types::{MockRule, RequestRecord, MAX_REQUEST_RECORDS};
use anyhow::Result;
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Mutex;

/// SQLite-backed store for routes, requests, mocks, and settings.
pub struct Store {
    conn: Mutex<Connection>,
}

impl Store {
    /// Open (or create) the database at the given path.
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;

        // Enable WAL mode for concurrent reads
        conn.pragma_update(None, "journal_mode", "wal")?;
        conn.pragma_update(None, "synchronous", "normal")?;
        conn.pragma_update(None, "foreign_keys", "on")?;

        let store = Self {
            conn: Mutex::new(conn),
        };
        store.migrate()?;
        Ok(store)
    }

    /// Create an in-memory database (for testing).
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "on")?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.migrate()?;
        Ok(store)
    }

    /// Run schema migrations.
    fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS requests (
                id              TEXT PRIMARY KEY,
                app_name        TEXT NOT NULL,
                timestamp       TEXT NOT NULL,
                duration_ms     INTEGER NOT NULL,

                method          TEXT NOT NULL,
                url             TEXT NOT NULL,
                path            TEXT NOT NULL,
                query_string    TEXT NOT NULL DEFAULT '',
                request_headers TEXT NOT NULL DEFAULT '{}',
                request_body    BLOB,
                request_content_type TEXT,

                status_code     INTEGER NOT NULL DEFAULT 0,
                response_headers TEXT NOT NULL DEFAULT '{}',
                response_body   BLOB,
                response_content_type TEXT,

                mocked          INTEGER NOT NULL DEFAULT 0,
                intercepted     INTEGER NOT NULL DEFAULT 0,
                parent_id       TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_requests_app_name ON requests(app_name);
            CREATE INDEX IF NOT EXISTS idx_requests_timestamp ON requests(timestamp DESC);
            CREATE INDEX IF NOT EXISTS idx_requests_status_code ON requests(status_code);
            CREATE INDEX IF NOT EXISTS idx_requests_method ON requests(method);
            CREATE INDEX IF NOT EXISTS idx_requests_path ON requests(path);

            CREATE TABLE IF NOT EXISTS mocks (
                id              TEXT PRIMARY KEY,
                app_name        TEXT NOT NULL,
                method          TEXT,
                path_pattern    TEXT NOT NULL,
                status_code     INTEGER NOT NULL DEFAULT 200,
                response_headers TEXT NOT NULL DEFAULT '{}',
                response_body   TEXT NOT NULL DEFAULT '',
                enabled         INTEGER NOT NULL DEFAULT 1
            );

            CREATE INDEX IF NOT EXISTS idx_mocks_app_name ON mocks(app_name);

            CREATE TABLE IF NOT EXISTS intercept_rules (
                id              TEXT PRIMARY KEY,
                app_name        TEXT NOT NULL,
                method          TEXT,
                path_pattern    TEXT,
                enabled         INTEGER NOT NULL DEFAULT 1
            );

            CREATE INDEX IF NOT EXISTS idx_intercept_rules_app_name ON intercept_rules(app_name);

            CREATE TABLE IF NOT EXISTS settings (
                key             TEXT PRIMARY KEY,
                value           TEXT NOT NULL
            );
            ",
        )?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Request records
    // -----------------------------------------------------------------------

    /// Insert a captured request record.
    pub fn insert_request(&self, record: &RequestRecord) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO requests (
                id, app_name, timestamp, duration_ms,
                method, url, path, query_string,
                request_headers, request_body, request_content_type,
                status_code, response_headers, response_body, response_content_type,
                mocked, intercepted, parent_id
            ) VALUES (
                ?1, ?2, ?3, ?4,
                ?5, ?6, ?7, ?8,
                ?9, ?10, ?11,
                ?12, ?13, ?14, ?15,
                ?16, ?17, ?18
            )",
            params![
                record.id,
                record.app_name,
                record.timestamp.to_rfc3339(),
                record.duration_ms,
                record.method,
                record.url,
                record.path,
                record.query_string,
                serde_json::to_string(&record.request_headers)?,
                record.request_body,
                record.request_content_type,
                record.status_code,
                serde_json::to_string(&record.response_headers)?,
                record.response_body,
                record.response_content_type,
                record.mocked as i32,
                0i32, // intercepted (removed feature, kept for schema compat)
                record.parent_id,
            ],
        )?;
        Ok(())
    }

    /// Get a request record by ID.
    pub fn get_request(&self, id: &str) -> Result<Option<RequestRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, app_name, timestamp, duration_ms,
                    method, url, path, query_string,
                    request_headers, request_body, request_content_type,
                    status_code, response_headers, response_body, response_content_type,
                    mocked, intercepted, parent_id
             FROM requests WHERE id = ?1",
        )?;

        let result = stmt.query_row(params![id], |row| Ok(Self::row_to_request_record(row)));

        match result {
            Ok(record) => Ok(Some(record?)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// List recent request records with optional filtering.
    pub fn list_requests(&self, filter: &RequestFilter) -> Result<Vec<RequestRecord>> {
        let conn = self.conn.lock().unwrap();

        let mut sql = String::from(
            "SELECT id, app_name, timestamp, duration_ms,
                    method, url, path, query_string,
                    request_headers, request_body, request_content_type,
                    status_code, response_headers, response_body, response_content_type,
                    mocked, intercepted, parent_id
             FROM requests WHERE 1=1",
        );
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(ref app) = filter.app_name {
            param_values.push(Box::new(app.clone()));
            sql.push_str(&format!(" AND app_name = ?{}", param_values.len()));
        }
        if let Some(ref method) = filter.method {
            param_values.push(Box::new(method.clone()));
            sql.push_str(&format!(" AND method = ?{}", param_values.len()));
        }
        if let Some(status) = filter.status_code {
            param_values.push(Box::new(status as i32));
            sql.push_str(&format!(" AND status_code = ?{}", param_values.len()));
        }
        if let Some(ref path) = filter.path_prefix {
            param_values.push(Box::new(format!("{}%", path)));
            sql.push_str(&format!(" AND path LIKE ?{}", param_values.len()));
        }
        if let Some(ref search) = filter.search {
            let pattern = format!("%{}%", search);
            param_values.push(Box::new(pattern));
            let idx = param_values.len();
            sql.push_str(&format!(" AND (url LIKE ?{idx} OR path LIKE ?{idx})"));
        }

        sql.push_str(" ORDER BY timestamp DESC");

        let limit = filter.limit.unwrap_or(50).min(1000);
        let offset = filter.offset.unwrap_or(0);
        param_values.push(Box::new(limit as i64));
        sql.push_str(&format!(" LIMIT ?{}", param_values.len()));
        param_values.push(Box::new(offset as i64));
        sql.push_str(&format!(" OFFSET ?{}", param_values.len()));

        let mut stmt = conn.prepare(&sql)?;
        let params_ref: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|b| b.as_ref()).collect();

        let rows = stmt.query_map(params_ref.as_slice(), |row| {
            Ok(Self::row_to_request_record(row))
        })?;

        let mut records = Vec::new();
        for row in rows {
            records.push(row??);
        }
        Ok(records)
    }

    /// List request summaries (no bodies) with optional filtering.
    ///
    /// This is much lighter than `list_requests` because it skips the
    /// request/response body and header columns that can be large.
    pub fn list_request_summaries(
        &self,
        filter: &RequestFilter,
    ) -> Result<Vec<crate::types::RequestSummary>> {
        let conn = self.conn.lock().unwrap();

        let mut sql = String::from(
            "SELECT id, app_name, timestamp, duration_ms,
                    method, path,
                    status_code, mocked
             FROM requests WHERE 1=1",
        );
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(ref app) = filter.app_name {
            param_values.push(Box::new(app.clone()));
            sql.push_str(&format!(" AND app_name = ?{}", param_values.len()));
        }
        if let Some(ref method) = filter.method {
            param_values.push(Box::new(method.clone()));
            sql.push_str(&format!(" AND method = ?{}", param_values.len()));
        }
        if let Some(status) = filter.status_code {
            param_values.push(Box::new(status as i32));
            sql.push_str(&format!(" AND status_code = ?{}", param_values.len()));
        }
        if let Some(ref path) = filter.path_prefix {
            param_values.push(Box::new(format!("{}%", path)));
            sql.push_str(&format!(" AND path LIKE ?{}", param_values.len()));
        }
        if let Some(ref search) = filter.search {
            let pattern = format!("%{}%", search);
            param_values.push(Box::new(pattern));
            let idx = param_values.len();
            sql.push_str(&format!(" AND (url LIKE ?{idx} OR path LIKE ?{idx})"));
        }

        sql.push_str(" ORDER BY timestamp DESC");

        let limit = filter.limit.unwrap_or(50).min(1000);
        let offset = filter.offset.unwrap_or(0);
        param_values.push(Box::new(limit as i64));
        sql.push_str(&format!(" LIMIT ?{}", param_values.len()));
        param_values.push(Box::new(offset as i64));
        sql.push_str(&format!(" OFFSET ?{}", param_values.len()));

        let mut stmt = conn.prepare(&sql)?;
        let params_ref: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|b| b.as_ref()).collect();

        let rows = stmt.query_map(params_ref.as_slice(), |row| {
            let timestamp_str: String = row.get(2)?;
            Ok(crate::types::RequestSummary {
                id: row.get(0)?,
                app_name: row.get(1)?,
                timestamp: chrono::DateTime::parse_from_rfc3339(&timestamp_str)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
                duration_ms: row.get::<_, i64>(3)? as u64,
                method: row.get(4)?,
                path: row.get(5)?,
                status_code: row.get::<_, i32>(6)? as u16,
                mocked: row.get::<_, i32>(7)? != 0,
            })
        })?;

        let mut summaries = Vec::new();
        for row in rows {
            summaries.push(row?);
        }
        Ok(summaries)
    }

    /// Delete old request records beyond the retention limit.
    pub fn evict_old_requests(&self) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM requests", [], |row| row.get(0))?;
        if count <= MAX_REQUEST_RECORDS as i64 {
            return Ok(0);
        }
        let to_delete = count - MAX_REQUEST_RECORDS as i64;
        let deleted = conn.execute(
            "DELETE FROM requests WHERE id IN (
                SELECT id FROM requests ORDER BY timestamp ASC LIMIT ?1
            )",
            params![to_delete],
        )?;
        Ok(deleted)
    }

    /// Clear all request records, optionally filtered by app.
    pub fn clear_requests(&self, app_name: Option<&str>) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let deleted = match app_name {
            Some(app) => conn.execute("DELETE FROM requests WHERE app_name = ?1", params![app])?,
            None => conn.execute("DELETE FROM requests", [])?,
        };
        Ok(deleted)
    }

    /// Get total request count.
    pub fn request_count(&self) -> Result<u64> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM requests", [], |row| row.get(0))?;
        Ok(count as u64)
    }

    // -----------------------------------------------------------------------
    // Mock rules
    // -----------------------------------------------------------------------

    /// Insert a mock rule.
    pub fn insert_mock(&self, mock: &MockRule) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO mocks (id, app_name, method, path_pattern, status_code, response_headers, response_body, enabled)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                mock.id,
                mock.app_name,
                mock.method,
                mock.path_pattern,
                mock.status_code as i32,
                serde_json::to_string(&mock.response_headers)?,
                mock.response_body,
                mock.enabled as i32,
            ],
        )?;
        Ok(())
    }

    /// List all mock rules, optionally filtered by app.
    pub fn list_mocks(&self, app_name: Option<&str>) -> Result<Vec<MockRule>> {
        let conn = self.conn.lock().unwrap();
        let mut mocks = Vec::new();

        let map_row = |row: &rusqlite::Row| {
            let headers_str: String = row.get(5)?;
            let headers = serde_json::from_str(&headers_str).unwrap_or_default();
            Ok(MockRule {
                id: row.get(0)?,
                app_name: row.get(1)?,
                method: row.get(2)?,
                path_pattern: row.get(3)?,
                status_code: row.get::<_, i32>(4)? as u16,
                response_headers: headers,
                response_body: row.get(6)?,
                enabled: row.get::<_, i32>(7)? != 0,
                hit_count: 0,
            })
        };

        match app_name {
            Some(app) => {
                let mut stmt = conn.prepare(
                    "SELECT id, app_name, method, path_pattern, status_code, response_headers, response_body, enabled FROM mocks WHERE app_name = ?1",
                )?;
                let rows = stmt.query_map(params![app], map_row)?;
                for row in rows {
                    mocks.push(row?);
                }
            }
            None => {
                let mut stmt = conn.prepare(
                    "SELECT id, app_name, method, path_pattern, status_code, response_headers, response_body, enabled FROM mocks",
                )?;
                let rows = stmt.query_map([], map_row)?;
                for row in rows {
                    mocks.push(row?);
                }
            }
        }

        Ok(mocks)
    }

    /// Delete a mock rule by ID.
    pub fn delete_mock(&self, id: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let deleted = conn.execute("DELETE FROM mocks WHERE id = ?1", params![id])?;
        Ok(deleted > 0)
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn row_to_request_record(row: &rusqlite::Row) -> Result<RequestRecord> {
        let req_headers_str: String = row.get(8)?;
        let resp_headers_str: String = row.get(12)?;
        let timestamp_str: String = row.get(2)?;

        Ok(RequestRecord {
            id: row.get(0)?,
            app_name: row.get(1)?,
            timestamp: chrono::DateTime::parse_from_rfc3339(&timestamp_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            duration_ms: row.get::<_, i64>(3)? as u64,
            method: row.get(4)?,
            url: row.get(5)?,
            path: row.get(6)?,
            query_string: row.get(7)?,
            request_headers: serde_json::from_str(&req_headers_str).unwrap_or_default(),
            request_body: row.get(9)?,
            request_content_type: row.get(10)?,
            status_code: row.get::<_, i32>(11)? as u16,
            status_message: String::new(),
            response_headers: serde_json::from_str(&resp_headers_str).unwrap_or_default(),
            response_body: row.get(13)?,
            response_content_type: row.get(14)?,
            mocked: row.get::<_, i32>(15)? != 0,
            // column 16 = intercepted (removed, skip)
            parent_id: row.get(17)?,
        })
    }
}

/// Filter criteria for listing requests.
#[derive(Debug, Clone, Default)]
pub struct RequestFilter {
    pub app_name: Option<String>,
    pub method: Option<String>,
    pub status_code: Option<u16>,
    pub path_prefix: Option<String>,
    pub search: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::RequestRecord;
    use chrono::Utc;
    use std::collections::HashMap;

    fn make_test_record(
        id: &str,
        app: &str,
        method: &str,
        path: &str,
        status: u16,
    ) -> RequestRecord {
        RequestRecord {
            id: id.to_string(),
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
    fn test_insert_and_get_request() {
        let store = Store::in_memory().unwrap();
        let record = make_test_record("req-1", "my-app", "GET", "/api/users", 200);
        store.insert_request(&record).unwrap();

        let fetched = store.get_request("req-1").unwrap().unwrap();
        assert_eq!(fetched.id, "req-1");
        assert_eq!(fetched.app_name, "my-app");
        assert_eq!(fetched.status_code, 200);
    }

    #[test]
    fn test_list_requests_with_filter() {
        let store = Store::in_memory().unwrap();
        store
            .insert_request(&make_test_record("r1", "web", "GET", "/", 200))
            .unwrap();
        store
            .insert_request(&make_test_record("r2", "api", "POST", "/api/users", 201))
            .unwrap();
        store
            .insert_request(&make_test_record("r3", "api", "GET", "/api/users/1", 404))
            .unwrap();

        // Filter by app
        let results = store
            .list_requests(&RequestFilter {
                app_name: Some("api".to_string()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(results.len(), 2);

        // Filter by method
        let results = store
            .list_requests(&RequestFilter {
                method: Some("POST".to_string()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "r2");

        // Filter by status
        let results = store
            .list_requests(&RequestFilter {
                status_code: Some(404),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "r3");
    }

    #[test]
    fn test_request_count_and_clear() {
        let store = Store::in_memory().unwrap();
        store
            .insert_request(&make_test_record("r1", "web", "GET", "/", 200))
            .unwrap();
        store
            .insert_request(&make_test_record("r2", "api", "GET", "/api", 200))
            .unwrap();

        assert_eq!(store.request_count().unwrap(), 2);

        store.clear_requests(Some("web")).unwrap();
        assert_eq!(store.request_count().unwrap(), 1);

        store.clear_requests(None).unwrap();
        assert_eq!(store.request_count().unwrap(), 0);
    }

    #[test]
    fn test_mock_crud() {
        let store = Store::in_memory().unwrap();
        let mock = MockRule {
            id: "m1".to_string(),
            app_name: "api".to_string(),
            method: Some("POST".to_string()),
            path_pattern: "/api/payments".to_string(),
            status_code: 500,
            response_headers: HashMap::new(),
            response_body: r#"{"error":"declined"}"#.to_string(),
            enabled: true,
            hit_count: 0,
        };

        store.insert_mock(&mock).unwrap();
        let mocks = store.list_mocks(Some("api")).unwrap();
        assert_eq!(mocks.len(), 1);
        assert_eq!(mocks[0].status_code, 500);

        store.delete_mock("m1").unwrap();
        let mocks = store.list_mocks(None).unwrap();
        assert_eq!(mocks.len(), 0);
    }
}
