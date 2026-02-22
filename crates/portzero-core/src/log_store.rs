//! SQLite-backed log storage for daemon-managed app logs.
//!
//! Each app gets its own set of log lines stored in an `app_logs` table.
//! A ring-buffer behavior is maintained by pruning old entries when the
//! per-app count exceeds `max_lines`. Falls back to in-memory storage
//! if no database path is provided.

use crate::types::{LogLine, LogStream};
use chrono::Utc;
use rusqlite::{params, Connection};
use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::sync::Mutex;

/// Default maximum log lines per app.
const DEFAULT_MAX_LINES: usize = 5000;

/// Thread-safe log storage with SQLite persistence.
pub struct LogStore {
    /// SQLite connection (None = in-memory-only mode for tests).
    db: Option<Mutex<Connection>>,
    /// In-memory fallback when no DB is provided (also used as a write-through
    /// cache for the hot path — reads go to SQLite first when available).
    memory: Mutex<HashMap<String, VecDeque<LogLine>>>,
    max_lines: usize,
}

impl LogStore {
    /// Create a new in-memory-only log store (no persistence).
    pub fn new() -> Self {
        Self {
            db: None,
            memory: Mutex::new(HashMap::new()),
            max_lines: DEFAULT_MAX_LINES,
        }
    }

    /// Create a log store backed by SQLite at the given path.
    /// The `app_logs` table is created if it doesn't exist.
    pub fn open(db_path: &Path) -> anyhow::Result<Self> {
        let conn = Connection::open(db_path)?;
        conn.pragma_update(None, "journal_mode", "wal")?;
        conn.pragma_update(None, "synchronous", "normal")?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS app_logs (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                app_name    TEXT NOT NULL,
                timestamp   TEXT NOT NULL,
                stream      TEXT NOT NULL,
                content     TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_app_logs_app_name ON app_logs(app_name);
            CREATE INDEX IF NOT EXISTS idx_app_logs_id_app ON app_logs(app_name, id DESC);
            ",
        )?;

        Ok(Self {
            db: Some(Mutex::new(conn)),
            memory: Mutex::new(HashMap::new()),
            max_lines: DEFAULT_MAX_LINES,
        })
    }

    /// Append a log line for an app.
    pub fn append(&self, app_name: &str, stream: LogStream, content: String) {
        let timestamp = Utc::now();
        let log_line = LogLine {
            timestamp,
            stream,
            content: content.clone(),
        };

        // Write to SQLite if available
        if let Some(ref db) = self.db {
            let conn = db.lock().unwrap();
            let stream_str = match stream {
                LogStream::Stdout => "stdout",
                LogStream::Stderr => "stderr",
            };
            let ts = timestamp.to_rfc3339();

            let _ = conn.execute(
                "INSERT INTO app_logs (app_name, timestamp, stream, content)
                 VALUES (?1, ?2, ?3, ?4)",
                params![app_name, ts, stream_str, content],
            );

            // Prune old entries if over the limit
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM app_logs WHERE app_name = ?1",
                    params![app_name],
                    |row| row.get(0),
                )
                .unwrap_or(0);

            if count > self.max_lines as i64 {
                let excess = count - self.max_lines as i64;
                let _ = conn.execute(
                    "DELETE FROM app_logs WHERE id IN (
                        SELECT id FROM app_logs WHERE app_name = ?1
                        ORDER BY id ASC LIMIT ?2
                    )",
                    params![app_name, excess],
                );
            }
        }

        // Also write to memory (for fast access / non-persistent mode)
        let mut map = self.memory.lock().unwrap();
        let buf = map
            .entry(app_name.to_string())
            .or_insert_with(|| VecDeque::with_capacity(self.max_lines));
        if buf.len() >= self.max_lines {
            buf.pop_front();
        }
        buf.push_back(log_line);
    }

    /// Get the last `n` log lines for an app. If `n` is None, returns all
    /// (up to `max_lines`).
    pub fn get_logs(&self, app_name: &str, n: Option<usize>) -> Vec<LogLine> {
        let limit = n.unwrap_or(self.max_lines);

        // Prefer SQLite if available (survives restarts)
        if let Some(ref db) = self.db {
            let conn = db.lock().unwrap();
            // Use a subquery to get the last N rows ordered ascending
            let mut stmt = match conn.prepare(
                "SELECT timestamp, stream, content FROM (
                    SELECT timestamp, stream, content FROM app_logs
                    WHERE app_name = ?1
                    ORDER BY id DESC
                    LIMIT ?2
                ) sub ORDER BY rowid ASC",
            ) {
                Ok(s) => s,
                Err(_) => return self.get_logs_from_memory(app_name, limit),
            };

            let rows = stmt
                .query_map(params![app_name, limit as i64], |row| {
                    let ts_str: String = row.get(0)?;
                    let stream_str: String = row.get(1)?;
                    let content: String = row.get(2)?;
                    Ok((ts_str, stream_str, content))
                })
                .ok();

            if let Some(rows) = rows {
                let mut result = Vec::new();
                for row in rows.flatten() {
                    let (ts_str, stream_str, content) = row;
                    let timestamp = chrono::DateTime::parse_from_rfc3339(&ts_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now());
                    let stream = match stream_str.as_str() {
                        "stderr" => LogStream::Stderr,
                        _ => LogStream::Stdout,
                    };
                    result.push(LogLine {
                        timestamp,
                        stream,
                        content,
                    });
                }
                return result;
            }
        }

        self.get_logs_from_memory(app_name, limit)
    }

    /// In-memory fallback for get_logs.
    fn get_logs_from_memory(&self, app_name: &str, limit: usize) -> Vec<LogLine> {
        let map = self.memory.lock().unwrap();
        match map.get(app_name) {
            Some(buf) => {
                let count = limit.min(buf.len());
                buf.iter().skip(buf.len() - count).cloned().collect()
            }
            None => vec![],
        }
    }

    /// Clear logs for an app.
    pub fn clear(&self, app_name: &str) {
        // Clear from SQLite
        if let Some(ref db) = self.db {
            let conn = db.lock().unwrap();
            let _ = conn.execute(
                "DELETE FROM app_logs WHERE app_name = ?1",
                params![app_name],
            );
        }

        // Clear from memory
        let mut map = self.memory.lock().unwrap();
        map.remove(app_name);
    }
}

impl Default for LogStore {
    fn default() -> Self {
        Self::new()
    }
}
