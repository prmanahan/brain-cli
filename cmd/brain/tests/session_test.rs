use assert_cmd::Command;

fn setup_test_db() -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.db").to_str().unwrap().to_string();

    let conn = rusqlite::Connection::open(&db_path).unwrap();
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         CREATE TABLE session_events (
             id INTEGER PRIMARY KEY AUTOINCREMENT,
             session_date TEXT NOT NULL,
             event_type TEXT NOT NULL,
             category TEXT,
             description TEXT NOT NULL,
             created_at TEXT NOT NULL DEFAULT (datetime('now'))
         );",
    )
    .unwrap();
    drop(conn);
    (dir, db_path)
}

#[test]
fn session_event_without_category_returns_id() {
    let (_dir, db_path) = setup_test_db();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "session",
            "event",
            "--date",
            "2026-03-29",
            "--type",
            "clarification",
            "--description",
            "Asked about scope of task",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v["event_id"], 1);
}

#[test]
fn session_event_with_category() {
    let (_dir, db_path) = setup_test_db();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "session",
            "event",
            "--date",
            "2026-03-29",
            "--type",
            "revision",
            "--description",
            "Changed acceptance criteria after delivery",
            "--category",
            "acceptance_criteria",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(v["event_id"].as_i64().unwrap() > 0);
}

#[test]
fn session_list_returns_array() {
    let (_dir, db_path) = setup_test_db();
    Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "session",
            "event",
            "--date",
            "2026-03-29",
            "--type",
            "clarification",
            "--description",
            "First event",
        ])
        .output()
        .unwrap();
    Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "session",
            "event",
            "--date",
            "2026-03-29",
            "--type",
            "revision",
            "--description",
            "Second event",
        ])
        .output()
        .unwrap();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args(["--db", &db_path, "session", "list"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v.as_array().unwrap().len(), 2);
}

#[test]
fn session_list_filters_by_date() {
    let (_dir, db_path) = setup_test_db();
    Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "session",
            "event",
            "--date",
            "2026-03-29",
            "--type",
            "clarification",
            "--description",
            "Today",
        ])
        .output()
        .unwrap();
    Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "session",
            "event",
            "--date",
            "2026-03-28",
            "--type",
            "revision",
            "--description",
            "Yesterday",
        ])
        .output()
        .unwrap();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args(["--db", &db_path, "session", "list", "--date", "2026-03-29"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v.as_array().unwrap().len(), 1);
    assert_eq!(v[0]["session_date"], "2026-03-29");
}

#[test]
fn session_list_filters_by_type() {
    let (_dir, db_path) = setup_test_db();
    Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "session",
            "event",
            "--date",
            "2026-03-29",
            "--type",
            "clarification",
            "--description",
            "A clarification",
        ])
        .output()
        .unwrap();
    Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "session",
            "event",
            "--date",
            "2026-03-29",
            "--type",
            "course_correction",
            "--description",
            "A course correction",
        ])
        .output()
        .unwrap();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "session",
            "list",
            "--type",
            "clarification",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v.as_array().unwrap().len(), 1);
    assert_eq!(v[0]["event_type"], "clarification");
}

#[test]
fn session_list_respects_limit() {
    let (_dir, db_path) = setup_test_db();
    for i in 0..5 {
        Command::cargo_bin("brain")
            .unwrap()
            .args([
                "--db",
                &db_path,
                "session",
                "event",
                "--date",
                "2026-03-29",
                "--type",
                "clarification",
                "--description",
                &format!("Event {i}"),
            ])
            .output()
            .unwrap();
    }
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args(["--db", &db_path, "session", "list", "--limit", "3"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v.as_array().unwrap().len(), 3);
}

#[test]
fn invalid_event_type_rejected_by_parser() {
    let (_dir, db_path) = setup_test_db();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "session",
            "event",
            "--date",
            "2026-03-29",
            "--type",
            "invalid_type",
            "--description",
            "Should fail",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
}

#[test]
fn session_event_rejects_invalid_date_format() {
    let (_dir, db_path) = setup_test_db();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "session",
            "event",
            "--date",
            "29-03-2026",
            "--type",
            "clarification",
            "--description",
            "Bad date format",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v["code"], "CONSTRAINT_VIOLATION");
}
