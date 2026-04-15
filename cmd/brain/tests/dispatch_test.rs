use assert_cmd::Command;

fn setup_test_db() -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.db").to_str().unwrap().to_string();

    let conn = rusqlite::Connection::open(&db_path).unwrap();
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         CREATE TABLE tasks (
             id INTEGER PRIMARY KEY, parent_id INTEGER, title TEXT NOT NULL,
             description TEXT, status TEXT NOT NULL DEFAULT 'open', priority TEXT,
             assigned_to TEXT, created_by TEXT NOT NULL, due_date TEXT,
             created_at TEXT NOT NULL DEFAULT (datetime('now')),
             updated_at TEXT NOT NULL DEFAULT (datetime('now')),
             completed_at TEXT, project_id INTEGER
         );
         CREATE TABLE dispatch_metrics (
             id INTEGER PRIMARY KEY, task_id INTEGER, agent TEXT NOT NULL,
             model TEXT NOT NULL, provider TEXT, dispatch_number INTEGER NOT NULL DEFAULT 1,
             status TEXT NOT NULL, tokens_input INTEGER, tokens_output INTEGER,
             tokens_total INTEGER, duration_ms INTEGER, cost_usd REAL,
             spec_review_passed_first_try INTEGER, tool_uses INTEGER,
             tools_external TEXT, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
             dispatch_tier TEXT, task_pattern TEXT, tier_justification TEXT,
             dispatch_context_kb REAL
         );
         CREATE TABLE dispatch_events (
             id INTEGER PRIMARY KEY, dispatch_id INTEGER NOT NULL,
             event_type TEXT NOT NULL, severity TEXT NOT NULL DEFAULT 'info',
             description TEXT NOT NULL, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
             FOREIGN KEY (dispatch_id) REFERENCES dispatch_metrics(id)
         );
         INSERT INTO tasks (title, created_by) VALUES ('Test task', 'Puck');",
    )
    .unwrap();
    drop(conn);
    (dir, db_path)
}

#[test]
fn dispatch_start_returns_json() {
    let (_dir, db_path) = setup_test_db();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "dispatch",
            "start",
            "--task-id",
            "1",
            "--agent",
            "Rune",
            "--provider",
            "anthropic",
            "--model",
            "sonnet",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v["dispatch_id"], 1);
    assert_eq!(v["dispatch_number"], 1);
}

#[test]
fn dispatch_start_invalid_task_id_exits_with_error() {
    let (_dir, db_path) = setup_test_db();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "dispatch",
            "start",
            "--task-id",
            "0",
            "--agent",
            "Rune",
            "--provider",
            "anthropic",
            "--model",
            "sonnet",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
}

#[test]
fn dispatch_complete_updates_metrics() {
    let (_dir, db_path) = setup_test_db();
    Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "dispatch",
            "start",
            "--task-id",
            "1",
            "--agent",
            "Rune",
            "--provider",
            "anthropic",
            "--model",
            "sonnet",
        ])
        .output()
        .unwrap();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "dispatch",
            "complete",
            "--id",
            "1",
            "--status",
            "completed",
            "--tokens-input",
            "1500",
            "--tokens-output",
            "3200",
            "--duration-ms",
            "120000",
            "--tool-uses",
            "45",
            "--cost",
            "0.21",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v["status"], "completed");
}

#[test]
fn dispatch_event_logs_event() {
    let (_dir, db_path) = setup_test_db();
    Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "dispatch",
            "start",
            "--task-id",
            "1",
            "--agent",
            "Rune",
            "--provider",
            "anthropic",
            "--model",
            "sonnet",
        ])
        .output()
        .unwrap();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "dispatch",
            "event",
            "--id",
            "1",
            "--type",
            "routing",
            "--severity",
            "info",
            "--description",
            "Dispatched to Rune",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(v["event_id"].as_i64().unwrap() > 0);
}

#[test]
fn dispatch_get_returns_record() {
    let (_dir, db_path) = setup_test_db();
    Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "dispatch",
            "start",
            "--task-id",
            "1",
            "--agent",
            "Rune",
            "--provider",
            "anthropic",
            "--model",
            "sonnet",
        ])
        .output()
        .unwrap();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args(["--db", &db_path, "dispatch", "get", "1"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v["agent"], "Rune");
    assert_eq!(v["provider"], "anthropic");
}

#[test]
fn dispatch_list_returns_array() {
    let (_dir, db_path) = setup_test_db();
    Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "dispatch",
            "start",
            "--task-id",
            "1",
            "--agent",
            "Rune",
            "--provider",
            "anthropic",
            "--model",
            "sonnet",
        ])
        .output()
        .unwrap();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args(["--db", &db_path, "dispatch", "list"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v.as_array().unwrap().len(), 1);
}

#[test]
fn dispatch_start_with_tier_fields_appears_in_get() {
    let (_dir, db_path) = setup_test_db();
    Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "dispatch",
            "start",
            "--task-id",
            "1",
            "--agent",
            "Forge",
            "--provider",
            "anthropic",
            "--model",
            "sonnet",
            "--tier",
            "t2",
            "--task-pattern",
            "implementation",
            "--tier-justification",
            "Sonnet fits implementation tasks",
        ])
        .output()
        .unwrap();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args(["--db", &db_path, "dispatch", "get", "1"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v["dispatch_tier"], "t2");
    assert_eq!(v["task_pattern"], "implementation");
    assert_eq!(v["tier_justification"], "Sonnet fits implementation tasks");
}

#[test]
fn dispatch_start_without_tier_flags_backward_compat() {
    let (_dir, db_path) = setup_test_db();
    Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "dispatch",
            "start",
            "--task-id",
            "1",
            "--agent",
            "Rune",
            "--provider",
            "anthropic",
            "--model",
            "sonnet",
        ])
        .output()
        .unwrap();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args(["--db", &db_path, "dispatch", "get", "1"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(v["dispatch_tier"].is_null());
    assert!(v["task_pattern"].is_null());
    assert!(v["tier_justification"].is_null());
    assert!(v["dispatch_context_kb"].is_null());
}

#[test]
fn dispatch_start_with_context_kb_flag() {
    let (_dir, db_path) = setup_test_db();
    Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "dispatch",
            "start",
            "--task-id",
            "1",
            "--agent",
            "Forge",
            "--provider",
            "anthropic",
            "--model",
            "sonnet",
            "--dispatch-context-kb",
            "14.0",
        ])
        .output()
        .unwrap();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args(["--db", &db_path, "dispatch", "get", "1"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v["dispatch_context_kb"], 14.0);
}
