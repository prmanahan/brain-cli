use assert_cmd::Command;

fn setup_test_db() -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.db").to_str().unwrap().to_string();

    let conn = rusqlite::Connection::open(&db_path).unwrap();
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         CREATE TABLE projects (
             id INTEGER PRIMARY KEY AUTOINCREMENT,
             name TEXT NOT NULL UNIQUE,
             description TEXT,
             path TEXT,
             status TEXT NOT NULL DEFAULT 'active'
                 CHECK(status IN ('active','paused','completed','archived')),
             created_at TEXT NOT NULL DEFAULT (datetime('now')),
             updated_at TEXT NOT NULL DEFAULT (datetime('now'))
         );",
    )
    .unwrap();
    drop(conn);
    (dir, db_path)
}

#[test]
fn project_create_returns_id() {
    let (_dir, db_path) = setup_test_db();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args(["--db", &db_path, "project", "create", "--name", "project-a"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v["project_id"], 1);
}

#[test]
fn project_create_with_description() {
    let (_dir, db_path) = setup_test_db();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "project",
            "create",
            "--name",
            "project-b",
            "--description",
            "Brain CLI tool",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(v["project_id"].as_i64().unwrap() > 0);
}

#[test]
fn project_add_is_alias_for_project_create() {
    let (_dir, db_path) = setup_test_db();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args(["--db", &db_path, "project", "add", "--name", "project-a"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "project add should succeed: {:?}",
        output
    );
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v["project_id"], 1);
}

#[test]
fn project_list_returns_array() {
    let (_dir, db_path) = setup_test_db();
    Command::cargo_bin("brain")
        .unwrap()
        .args(["--db", &db_path, "project", "create", "--name", "project-a"])
        .output()
        .unwrap();
    Command::cargo_bin("brain")
        .unwrap()
        .args(["--db", &db_path, "project", "create", "--name", "project-b"])
        .output()
        .unwrap();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args(["--db", &db_path, "project", "list"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v.as_array().unwrap().len(), 2);
}

#[test]
fn project_list_filters_by_status() {
    let (_dir, db_path) = setup_test_db();
    Command::cargo_bin("brain")
        .unwrap()
        .args(["--db", &db_path, "project", "create", "--name", "project-a"])
        .output()
        .unwrap();
    Command::cargo_bin("brain")
        .unwrap()
        .args(["--db", &db_path, "project", "create", "--name", "project-b"])
        .output()
        .unwrap();
    Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db", &db_path, "project", "update", "1", "--status", "archived",
        ])
        .output()
        .unwrap();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args(["--db", &db_path, "project", "list", "--status", "active"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v.as_array().unwrap().len(), 1);
    assert_eq!(v[0]["name"], "project-b");
}

#[test]
fn project_update_changes_status() {
    let (_dir, db_path) = setup_test_db();
    Command::cargo_bin("brain")
        .unwrap()
        .args(["--db", &db_path, "project", "create", "--name", "project-a"])
        .output()
        .unwrap();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db", &db_path, "project", "update", "1", "--status", "paused",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v["project_id"], 1);
    assert_eq!(v["status"], "paused");
}

#[test]
fn project_update_not_found_returns_error() {
    let (_dir, db_path) = setup_test_db();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db", &db_path, "project", "update", "999", "--status", "paused",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v["code"], "NOT_FOUND");
}

#[test]
fn invalid_project_status_rejected_by_parser() {
    let (_dir, db_path) = setup_test_db();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "project",
            "update",
            "1",
            "--status",
            "unknown_status",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
}

#[test]
fn project_create_with_path() {
    let (_dir, db_path) = setup_test_db();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "project",
            "create",
            "--name",
            "project-a",
            "--path",
            "/tmp/brain-cli-test/project-a",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v["project_id"], 1);

    // Verify path is stored by listing
    let list_output = Command::cargo_bin("brain")
        .unwrap()
        .args(["--db", &db_path, "project", "list"])
        .output()
        .unwrap();
    assert!(list_output.status.success());
    let list: serde_json::Value = serde_json::from_slice(&list_output.stdout).unwrap();
    assert_eq!(list[0]["path"], "/tmp/brain-cli-test/project-a");
}

#[test]
fn project_create_without_path() {
    let (_dir, db_path) = setup_test_db();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args(["--db", &db_path, "project", "create", "--name", "project-b"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let list_output = Command::cargo_bin("brain")
        .unwrap()
        .args(["--db", &db_path, "project", "list"])
        .output()
        .unwrap();
    assert!(list_output.status.success());
    let list: serde_json::Value = serde_json::from_slice(&list_output.stdout).unwrap();
    assert!(list[0]["path"].is_null());
}

#[test]
fn project_update_path() {
    let (_dir, db_path) = setup_test_db();
    Command::cargo_bin("brain")
        .unwrap()
        .args(["--db", &db_path, "project", "create", "--name", "project-a"])
        .output()
        .unwrap();

    let output = Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "project",
            "update",
            "1",
            "--path",
            "/tmp/brain-cli-test/project-a",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());

    let list_output = Command::cargo_bin("brain")
        .unwrap()
        .args(["--db", &db_path, "project", "list"])
        .output()
        .unwrap();
    assert!(list_output.status.success());
    let list: serde_json::Value = serde_json::from_slice(&list_output.stdout).unwrap();
    assert_eq!(list[0]["path"], "/tmp/brain-cli-test/project-a");
}

#[test]
fn project_list_includes_path() {
    let (_dir, db_path) = setup_test_db();
    Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "project",
            "create",
            "--name",
            "project-a",
            "--path",
            "/some/path",
        ])
        .output()
        .unwrap();
    Command::cargo_bin("brain")
        .unwrap()
        .args(["--db", &db_path, "project", "create", "--name", "project-b"])
        .output()
        .unwrap();

    let output = Command::cargo_bin("brain")
        .unwrap()
        .args(["--db", &db_path, "project", "list"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let list: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(list.as_array().unwrap().len(), 2);

    // Both items have a "path" key (one with value, one null)
    for item in list.as_array().unwrap() {
        assert!(item.get("path").is_some(), "path key must be present");
    }
    // Find project-a by name and verify path
    let project_a = list
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["name"] == "project-a")
        .unwrap();
    assert_eq!(project_a["path"], "/some/path");
    // Find project-b and verify null
    let project_b = list
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["name"] == "project-b")
        .unwrap();
    assert!(project_b["path"].is_null());
}
