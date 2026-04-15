use assert_cmd::Command;

fn setup_test_db() -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.db").to_str().unwrap().to_string();

    let conn = rusqlite::Connection::open(&db_path).unwrap();
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         CREATE TABLE activity_log (
             id INTEGER PRIMARY KEY AUTOINCREMENT,
             actor TEXT NOT NULL,
             action TEXT NOT NULL,
             task_id INTEGER,
             target_type TEXT,
             target_id INTEGER,
             summary TEXT,
             created_at TEXT NOT NULL DEFAULT (datetime('now'))
         );",
    )
    .unwrap();
    drop(conn);
    (dir, db_path)
}

#[test]
fn activity_log_minimal_args_returns_id() {
    let (_dir, db_path) = setup_test_db();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "activity",
            "log",
            "--actor",
            "Puck",
            "--action",
            "dispatched",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v["activity_id"], 1);
}

#[test]
fn activity_log_full_args() {
    let (_dir, db_path) = setup_test_db();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "activity",
            "log",
            "--actor",
            "Forge",
            "--action",
            "completed",
            "--task-id",
            "42",
            "--target-type",
            "task",
            "--target-id",
            "42",
            "--summary",
            "Finished all the work",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(v["activity_id"].as_i64().unwrap() > 0);
}

#[test]
fn activity_list_returns_array() {
    let (_dir, db_path) = setup_test_db();
    Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "activity",
            "log",
            "--actor",
            "Puck",
            "--action",
            "dispatched",
        ])
        .output()
        .unwrap();
    Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "activity",
            "log",
            "--actor",
            "Forge",
            "--action",
            "completed",
        ])
        .output()
        .unwrap();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args(["--db", &db_path, "activity", "list"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v.as_array().unwrap().len(), 2);
}

#[test]
fn activity_list_filters_by_actor() {
    let (_dir, db_path) = setup_test_db();
    Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "activity",
            "log",
            "--actor",
            "Puck",
            "--action",
            "dispatched",
        ])
        .output()
        .unwrap();
    Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "activity",
            "log",
            "--actor",
            "Forge",
            "--action",
            "completed",
        ])
        .output()
        .unwrap();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args(["--db", &db_path, "activity", "list", "--actor", "Puck"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v.as_array().unwrap().len(), 1);
    assert_eq!(v[0]["actor"], "Puck");
}

#[test]
fn activity_list_respects_limit() {
    let (_dir, db_path) = setup_test_db();
    for i in 0..5 {
        Command::cargo_bin("brain")
            .unwrap()
            .args([
                "--db",
                &db_path,
                "activity",
                "log",
                "--actor",
                "Puck",
                "--action",
                &format!("action_{i}"),
            ])
            .output()
            .unwrap();
    }
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args(["--db", &db_path, "activity", "list", "--limit", "3"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v.as_array().unwrap().len(), 3);
}
