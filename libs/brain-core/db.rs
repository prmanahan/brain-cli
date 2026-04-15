use crate::error::BrainError;
use rusqlite::Connection;
use std::path::PathBuf;

pub struct Database {
    #[allow(dead_code)]
    pub(crate) conn: Connection,
}

#[derive(serde::Deserialize)]
struct BrainConfig {
    database: DatabaseConfig,
}

#[derive(serde::Deserialize)]
struct DatabaseConfig {
    path: String,
}

#[allow(dead_code)]
fn find_config_file() -> Option<PathBuf> {
    let dir = std::env::current_dir().ok()?;
    find_config_file_from(&dir)
}

pub(crate) fn find_config_file_from(start: &std::path::Path) -> Option<PathBuf> {
    let mut dir = start.to_path_buf();
    loop {
        let candidate = dir.join(".config/brain.toml");
        if candidate.exists() {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}

fn read_config_path() -> Option<String> {
    let dir = std::env::current_dir().ok()?;
    read_config_path_from(&dir)
}

pub(crate) fn read_config_path_from(start: &std::path::Path) -> Option<String> {
    let config_path = find_config_file_from(start)?;
    let contents = std::fs::read_to_string(&config_path).ok()?;
    let config: BrainConfig = toml::from_str(&contents).ok()?;
    Some(config.database.path)
}

impl std::fmt::Debug for Database {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Database").finish_non_exhaustive()
    }
}

/// Production schema, FK-safe ordered: `projects` first (parent), then everything
/// that references it. Every statement is `CREATE TABLE IF NOT EXISTS` so calling
/// `ensure_schema()` against an existing database is a no-op (idempotent).
const SCHEMA_SQL: &str = "\
CREATE TABLE IF NOT EXISTS projects (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    path TEXT,
    status TEXT NOT NULL DEFAULT 'active'
        CHECK(status IN ('active','paused','completed','archived')),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS tasks (
    id INTEGER PRIMARY KEY,
    parent_id INTEGER,
    title TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL DEFAULT 'open',
    priority TEXT,
    assigned_to TEXT,
    created_by TEXT NOT NULL,
    due_date TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT,
    project_id INTEGER
);
CREATE TABLE IF NOT EXISTS dispatch_metrics (
    id INTEGER PRIMARY KEY,
    task_id INTEGER,
    agent TEXT NOT NULL,
    model TEXT NOT NULL,
    provider TEXT,
    dispatch_number INTEGER NOT NULL DEFAULT 1,
    status TEXT NOT NULL,
    tokens_input INTEGER,
    tokens_output INTEGER,
    tokens_total INTEGER,
    duration_ms INTEGER,
    cost_usd REAL,
    spec_review_passed_first_try INTEGER,
    tool_uses INTEGER,
    tools_external TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    dispatch_tier TEXT,
    task_pattern TEXT,
    tier_justification TEXT,
    dispatch_context_kb REAL
);
CREATE TABLE IF NOT EXISTS dispatch_events (
    id INTEGER PRIMARY KEY,
    dispatch_id INTEGER NOT NULL,
    event_type TEXT NOT NULL,
    severity TEXT NOT NULL DEFAULT 'info',
    description TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (dispatch_id) REFERENCES dispatch_metrics(id)
);
CREATE TABLE IF NOT EXISTS activity_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    actor TEXT NOT NULL,
    action TEXT NOT NULL,
    task_id INTEGER,
    target_type TEXT,
    target_id INTEGER,
    summary TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS session_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_date TEXT NOT NULL,
    event_type TEXT NOT NULL,
    category TEXT,
    description TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS scope_violations (
    id INTEGER PRIMARY KEY,
    agent TEXT NOT NULL,
    task_id INTEGER,
    dispatch_id INTEGER,
    violation_type TEXT NOT NULL,
    command_attempted TEXT NOT NULL,
    context TEXT,
    reason TEXT NOT NULL,
    resolution TEXT,
    created_at INTEGER NOT NULL
);
";

impl Database {
    pub fn open(path: &str) -> Result<Self, BrainError> {
        let conn = Connection::open(path).map_err(|e| BrainError::ConnectionFailed {
            path: path.to_string(),
            message: e.to_string(),
        })?;
        let db = Self::configure(conn)?;
        db.ensure_schema()?;
        Ok(db)
    }

    pub fn in_memory() -> Result<Self, BrainError> {
        let conn = Connection::open_in_memory().map_err(|e| BrainError::ConnectionFailed {
            path: ":memory:".to_string(),
            message: e.to_string(),
        })?;
        let db = Self::configure(conn)?;
        db.ensure_schema()?;
        Ok(db)
    }

    fn configure(conn: Connection) -> Result<Self, BrainError> {
        conn.execute_batch(
            "PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL; PRAGMA busy_timeout = 5000;",
        )?;
        Ok(Database { conn })
    }

    /// Create every production table idempotently in FK-safe order.
    /// Safe to call repeatedly — every statement uses `CREATE TABLE IF NOT EXISTS`.
    pub fn ensure_schema(&self) -> Result<(), BrainError> {
        self.conn.execute_batch(SCHEMA_SQL)?;
        Ok(())
    }

    pub fn resolve_path(explicit: Option<&str>) -> String {
        if let Some(path) = explicit {
            return path.to_string();
        }
        if let Ok(path) = std::env::var("BRAIN_DB_PATH") {
            return path;
        }
        if let Some(path) = read_config_path() {
            return path;
        }
        "./vault/brain.db".to_string()
    }

    /// Like `resolve_path` but uses `start` as the base directory for config file search.
    /// Intended for testing — avoids mutating process-global `current_dir`.
    #[allow(dead_code)]
    pub(crate) fn resolve_path_from(explicit: Option<&str>, start: &std::path::Path) -> String {
        if let Some(path) = explicit {
            return path.to_string();
        }
        if let Ok(path) = std::env::var("BRAIN_DB_PATH") {
            return path;
        }
        if let Some(path) = read_config_path_from(start) {
            return path;
        }
        "./vault/brain.db".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_database_opens() {
        let db = Database::in_memory().unwrap();
        let version: String = db
            .conn
            .query_row("SELECT sqlite_version()", [], |row| row.get(0))
            .unwrap();
        assert!(!version.is_empty());
    }

    #[test]
    fn in_memory_database_has_foreign_keys_enabled() {
        let db = Database::in_memory().unwrap();
        let fk: i32 = db
            .conn
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();
        assert_eq!(fk, 1);
    }

    #[test]
    fn in_memory_database_has_wal_mode() {
        let db = Database::in_memory().unwrap();
        let mode: String = db
            .conn
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .unwrap();
        // In-memory databases use "memory" journal mode, which is expected
        assert!(mode == "wal" || mode == "memory");
    }

    #[test]
    fn open_nonexistent_path_returns_connection_failed() {
        let result = Database::open("/nonexistent/path/to/db.sqlite");
        assert!(result.is_err());
        let err = result.unwrap_err();
        let json = err.to_json();
        assert!(json.contains("CONNECTION_FAILED"));
    }

    #[test]
    fn resolve_path_uses_explicit_db_path() {
        let path = Database::resolve_path(Some("/explicit/path.db"));
        assert_eq!(path, "/explicit/path.db");
    }

    #[test]
    fn resolve_path_uses_env_var_when_no_explicit() {
        // Temporarily set env var; must restore to avoid test pollution
        // SAFETY: single-threaded test binary context; no other threads read this var concurrently
        unsafe {
            std::env::set_var("BRAIN_DB_PATH", "/env/path.db");
        }
        let path = Database::resolve_path(None);
        unsafe {
            std::env::remove_var("BRAIN_DB_PATH");
        }
        assert_eq!(path, "/env/path.db");
    }

    #[test]
    fn resolve_path_explicit_overrides_env_var() {
        // SAFETY: single-threaded test binary context; no other threads read this var concurrently
        unsafe {
            std::env::set_var("BRAIN_DB_PATH", "/env/path.db");
        }
        let path = Database::resolve_path(Some("/explicit/path.db"));
        unsafe {
            std::env::remove_var("BRAIN_DB_PATH");
        }
        assert_eq!(path, "/explicit/path.db");
    }

    #[test]
    fn resolve_path_uses_config_file_when_present() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let config_dir = dir.path().join(".config");
        std::fs::create_dir_all(&config_dir).unwrap();
        let config_path = config_dir.join("brain.toml");
        let mut f = std::fs::File::create(&config_path).unwrap();
        writeln!(f, "[database]\npath = \"/from/config.db\"").unwrap();

        // Use resolve_path_from so we don't mutate process-global current_dir
        // SAFETY: remove env var to avoid interference; safe because we are not setting it
        unsafe {
            std::env::remove_var("BRAIN_DB_PATH");
        }
        let path = Database::resolve_path_from(None, dir.path());

        assert_eq!(path, "/from/config.db");
    }

    #[test]
    fn resolve_path_uses_default_when_nothing_set() {
        // Use a temp dir with no config file and no env var
        let dir = tempfile::tempdir().unwrap();
        // SAFETY: remove env var only (not setting), safe across parallel tests
        unsafe {
            std::env::remove_var("BRAIN_DB_PATH");
        }
        let path = Database::resolve_path_from(None, dir.path());
        assert_eq!(path, "./vault/brain.db");
    }

    #[test]
    fn ensure_schema_is_idempotent_with_seed_data() {
        let db = Database::in_memory().unwrap();

        // Seed one row per production table. Order respects FK relationships:
        // projects -> tasks -> dispatch_metrics -> dispatch_events -> activity_log
        // -> session_events -> scope_violations.
        db.conn
            .execute_batch(
                "INSERT INTO projects (id, name) VALUES (1, 'seed-project');
                 INSERT INTO tasks (id, title, created_by, project_id) VALUES (1, 'seed task', 'Puck', 1);
                 INSERT INTO dispatch_metrics (id, task_id, agent, model, status) VALUES (1, 1, 'Forge', 'sonnet', 'completed');
                 INSERT INTO dispatch_events (id, dispatch_id, event_type, description) VALUES (1, 1, 'start', 'seed event');
                 INSERT INTO activity_log (id, actor, action) VALUES (1, 'Puck', 'seeded');
                 INSERT INTO session_events (id, session_date, event_type, description) VALUES (1, '2026-04-15', 'start', 'seed session');
                 INSERT INTO scope_violations (id, agent, violation_type, command_attempted, reason, created_at) VALUES (1, 'Forge', 'permission_denied', 'rm -rf /', 'blocked', 0);",
            )
            .unwrap();

        // Capture schema fingerprint and per-table row counts BEFORE re-running ensure_schema.
        let schema_before: Vec<(String, String, String)> = {
            let mut stmt = db
                .conn
                .prepare("SELECT type, name, COALESCE(sql, '') FROM sqlite_master WHERE type = 'table' ORDER BY name")
                .unwrap();
            stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
        };

        let tables = [
            "projects",
            "tasks",
            "dispatch_metrics",
            "dispatch_events",
            "activity_log",
            "session_events",
            "scope_violations",
        ];
        let counts_before: Vec<i64> = tables
            .iter()
            .map(|t| {
                db.conn
                    .query_row(&format!("SELECT COUNT(*) FROM {t}"), [], |row| row.get(0))
                    .unwrap()
            })
            .collect();

        // Re-run ensure_schema — must not error and must not touch any data.
        db.ensure_schema()
            .expect("ensure_schema must be idempotent");

        let schema_after: Vec<(String, String, String)> = {
            let mut stmt = db
                .conn
                .prepare("SELECT type, name, COALESCE(sql, '') FROM sqlite_master WHERE type = 'table' ORDER BY name")
                .unwrap();
            stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
        };

        let counts_after: Vec<i64> = tables
            .iter()
            .map(|t| {
                db.conn
                    .query_row(&format!("SELECT COUNT(*) FROM {t}"), [], |row| row.get(0))
                    .unwrap()
            })
            .collect();

        assert_eq!(schema_before, schema_after, "schema must be unchanged");
        assert_eq!(counts_before, counts_after, "row counts must be unchanged");
        for (table, count) in tables.iter().zip(counts_after.iter()) {
            assert_eq!(*count, 1, "{table} should still have exactly its seed row");
        }
    }

    #[test]
    fn resolve_path_silently_ignores_malformed_config() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let config_dir = dir.path().join(".config");
        std::fs::create_dir_all(&config_dir).unwrap();
        let config_path = config_dir.join("brain.toml");
        let mut f = std::fs::File::create(&config_path).unwrap();
        writeln!(f, "this is not valid toml ===").unwrap();

        // SAFETY: remove env var only (not setting), safe across parallel tests
        unsafe {
            std::env::remove_var("BRAIN_DB_PATH");
        }
        let path = Database::resolve_path_from(None, dir.path());

        assert_eq!(path, "./vault/brain.db");
    }
}
