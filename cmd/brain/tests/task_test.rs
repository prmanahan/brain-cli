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
         );",
    )
    .unwrap();
    drop(conn);
    (dir, db_path)
}

#[test]
fn task_create_returns_id() {
    let (_dir, db_path) = setup_test_db();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "task",
            "create",
            "--title",
            "Build brain CLI",
            "--created-by",
            "Puck",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v["task_id"], 1);
}

#[test]
fn task_create_with_optional_fields() {
    let (_dir, db_path) = setup_test_db();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "task",
            "create",
            "--title",
            "Build brain CLI",
            "--description",
            "Rust CLI for puck.db",
            "--assigned-to",
            "Rune",
            "--priority",
            "high",
            "--project-id",
            "1",
            "--created-by",
            "Puck",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
}

#[test]
fn task_update_changes_status() {
    let (_dir, db_path) = setup_test_db();
    Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "task",
            "create",
            "--title",
            "Test",
            "--created-by",
            "Puck",
        ])
        .output()
        .unwrap();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "task",
            "update",
            "1",
            "--status",
            "in_progress",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v["status"], "in_progress");
}

#[test]
fn task_get_returns_full_record() {
    let (_dir, db_path) = setup_test_db();
    Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "task",
            "create",
            "--title",
            "Test task",
            "--created-by",
            "Puck",
        ])
        .output()
        .unwrap();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args(["--db", &db_path, "task", "get", "1"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v["title"], "Test task");
    assert_eq!(v["status"], "open");
}

#[test]
fn task_list_returns_array() {
    let (_dir, db_path) = setup_test_db();
    Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "task",
            "create",
            "--title",
            "Task 1",
            "--created-by",
            "Puck",
        ])
        .output()
        .unwrap();
    Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "task",
            "create",
            "--title",
            "Task 2",
            "--created-by",
            "Puck",
        ])
        .output()
        .unwrap();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args(["--db", &db_path, "task", "list"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v.as_array().unwrap().len(), 2);
}

#[test]
fn task_list_filters_by_status() {
    let (_dir, db_path) = setup_test_db();
    Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "task",
            "create",
            "--title",
            "Task 1",
            "--created-by",
            "Puck",
        ])
        .output()
        .unwrap();
    Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "task",
            "create",
            "--title",
            "Task 2",
            "--created-by",
            "Puck",
        ])
        .output()
        .unwrap();
    Command::cargo_bin("brain")
        .unwrap()
        .args(["--db", &db_path, "task", "update", "1", "--status", "done"])
        .output()
        .unwrap();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args(["--db", &db_path, "task", "list", "--status", "open"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v.as_array().unwrap().len(), 1);
}

#[test]
fn task_get_not_found_returns_error() {
    let (_dir, db_path) = setup_test_db();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args(["--db", &db_path, "task", "get", "999"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v["code"], "NOT_FOUND");
}

#[test]
fn invalid_status_rejected_by_parser() {
    let (_dir, db_path) = setup_test_db();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "task",
            "update",
            "1",
            "--status",
            "pending_review",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
}

// --- #175: task add alias ---

#[test]
fn task_add_is_alias_for_task_create() {
    let (_dir, db_path) = setup_test_db();
    // "task add" should behave identically to "task create"
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "task",
            "add",
            "--title",
            "Added via alias",
            "--created-by",
            "Puck",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "task add should succeed: {:?}",
        output
    );
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v["task_id"], 1);
}

// --- #174: --format flag ---

#[test]
fn task_list_format_json_is_default() {
    let (_dir, db_path) = setup_test_db();
    Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "task",
            "create",
            "--title",
            "Format test",
            "--created-by",
            "Puck",
        ])
        .output()
        .unwrap();
    // No --format flag: should return parseable JSON array
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args(["--db", &db_path, "task", "list"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(v.is_array());
}

#[test]
fn task_list_format_json_explicit() {
    let (_dir, db_path) = setup_test_db();
    Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "task",
            "create",
            "--title",
            "Format test",
            "--created-by",
            "Puck",
        ])
        .output()
        .unwrap();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args(["--db", &db_path, "task", "list", "--format", "json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(v.is_array());
}

#[test]
fn task_list_format_table_outputs_header_row() {
    let (_dir, db_path) = setup_test_db();
    Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "task",
            "create",
            "--title",
            "My table task",
            "--created-by",
            "Puck",
            "--priority",
            "high",
            "--assigned-to",
            "Rune",
        ])
        .output()
        .unwrap();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args(["--db", &db_path, "task", "list", "--format", "table"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    // Header must be present
    assert!(
        text.contains("ID"),
        "expected ID column header, got: {text}"
    );
    assert!(
        text.contains("TITLE"),
        "expected TITLE column header, got: {text}"
    );
    assert!(
        text.contains("STATUS"),
        "expected STATUS column header, got: {text}"
    );
    // Task data must appear
    assert!(
        text.contains("My table task"),
        "expected task title, got: {text}"
    );
    assert!(text.contains("open"), "expected status, got: {text}");
    // Must not be valid JSON
    assert!(
        serde_json::from_str::<serde_json::Value>(&text).is_err(),
        "table output should not be valid JSON"
    );
}

#[test]
fn task_list_format_table_empty_shows_no_results_message() {
    let (_dir, db_path) = setup_test_db();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args(["--db", &db_path, "task", "list", "--format", "table"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    // Should still emit a header or a "no tasks" message — not an error
    assert!(!text.is_empty(), "output should not be empty");
}

#[test]
fn task_list_invalid_format_rejected() {
    let (_dir, db_path) = setup_test_db();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args(["--db", &db_path, "task", "list", "--format", "csv"])
        .output()
        .unwrap();
    assert!(!output.status.success(), "csv format should be rejected");
}

// --- task update --project-id ---

#[test]
fn task_update_project_id_flag() {
    let (_dir, db_path) = setup_test_db();
    Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "task",
            "create",
            "--title",
            "Project ID test",
            "--created-by",
            "Puck",
        ])
        .output()
        .unwrap();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "task",
            "update",
            "1",
            "--project-id",
            "42",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "task update --project-id should succeed: {:?}",
        output
    );
    // Verify the project_id was stored
    let get_output = Command::cargo_bin("brain")
        .unwrap()
        .args(["--db", &db_path, "task", "get", "1"])
        .output()
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&get_output.stdout).unwrap();
    assert_eq!(v["project_id"], 42);
}

// --- subtask support: --parent-id, --tree, subtask summary, PARENT column ---

fn create_task(db_path: &str, title: &str) -> i64 {
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            db_path,
            "task",
            "create",
            "--title",
            title,
            "--created-by",
            "Puck",
        ])
        .output()
        .unwrap();
    assert!(output.status.success(), "create_task failed: {:?}", output);
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    v["task_id"].as_i64().unwrap()
}

fn create_child_task(db_path: &str, title: &str, parent_id: i64) -> i64 {
    let pid = parent_id.to_string();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            db_path,
            "task",
            "create",
            "--title",
            title,
            "--created-by",
            "Puck",
            "--parent-id",
            &pid,
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "create_child_task failed: {:?}",
        output
    );
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    v["task_id"].as_i64().unwrap()
}

#[test]
fn task_list_parent_id_returns_direct_children() {
    let (_dir, db_path) = setup_test_db();
    let parent_id = create_task(&db_path, "Parent");
    let c1 = create_child_task(&db_path, "Child 1", parent_id);
    let c2 = create_child_task(&db_path, "Child 2", parent_id);
    let c3 = create_child_task(&db_path, "Child 3", parent_id);

    let pid_str = parent_id.to_string();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args(["--db", &db_path, "task", "list", "--parent-id", &pid_str])
        .output()
        .unwrap();
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let arr = v.as_array().unwrap();
    assert_eq!(arr.len(), 3);
    let ids: Vec<i64> = arr.iter().map(|t| t["id"].as_i64().unwrap()).collect();
    assert!(ids.contains(&c1));
    assert!(ids.contains(&c2));
    assert!(ids.contains(&c3));
    assert!(!ids.contains(&parent_id));
}

#[test]
fn task_list_parent_id_not_found_returns_error() {
    let (_dir, db_path) = setup_test_db();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args(["--db", &db_path, "task", "list", "--parent-id", "9999"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(v["code"], "NOT_FOUND");
}

#[test]
fn task_list_tree_and_parent_id_mutual_exclusion() {
    let (_dir, db_path) = setup_test_db();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "task",
            "list",
            "--tree",
            "--parent-id",
            "1",
        ])
        .output()
        .unwrap();
    // clap should reject this combination
    assert!(!output.status.success());
}

#[test]
fn task_list_tree_returns_nested_json() {
    let (_dir, db_path) = setup_test_db();
    let parent_id = create_task(&db_path, "Parent");
    let c1 = create_child_task(&db_path, "Child 1", parent_id);
    let c2 = create_child_task(&db_path, "Child 2", parent_id);

    let output = Command::cargo_bin("brain")
        .unwrap()
        .args(["--db", &db_path, "task", "list", "--tree"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let arr = v.as_array().unwrap();

    // One root
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["id"].as_i64().unwrap(), parent_id);

    let children = arr[0]["children"].as_array().unwrap();
    assert_eq!(children.len(), 2);
    let child_ids: Vec<i64> = children.iter().map(|t| t["id"].as_i64().unwrap()).collect();
    assert!(child_ids.contains(&c1));
    assert!(child_ids.contains(&c2));
}

#[test]
fn task_list_tree_table_format_shows_indented_children() {
    let (_dir, db_path) = setup_test_db();
    let parent_id = create_task(&db_path, "Root task");
    create_child_task(&db_path, "Child task", parent_id);

    let output = Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db", &db_path, "task", "list", "--tree", "--format", "table",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("Root task"), "expected root task: {text}");
    assert!(text.contains("└─"), "expected tree indent marker: {text}");
    assert!(text.contains("Child task"), "expected child task: {text}");
}

#[test]
fn task_get_includes_subtasks_field() {
    let (_dir, db_path) = setup_test_db();
    let parent_id = create_task(&db_path, "Parent");
    create_child_task(&db_path, "Child A", parent_id);
    create_child_task(&db_path, "Child B", parent_id);

    let pid_str = parent_id.to_string();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args(["--db", &db_path, "task", "get", &pid_str])
        .output()
        .unwrap();
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let subtasks = v["subtasks"].as_array().expect("subtasks must be array");
    assert_eq!(subtasks.len(), 2);
    // Each subtask must have id, title, status
    for st in subtasks {
        assert!(st["id"].is_number());
        assert!(st["title"].is_string());
        assert!(st["status"].is_string());
    }
}

#[test]
fn task_get_subtasks_empty_for_leaf_task() {
    let (_dir, db_path) = setup_test_db();
    create_task(&db_path, "Lone task");

    let output = Command::cargo_bin("brain")
        .unwrap()
        .args(["--db", &db_path, "task", "get", "1"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let v: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let subtasks = v["subtasks"].as_array().expect("subtasks must be array");
    assert_eq!(subtasks.len(), 0);
}

#[test]
fn task_list_table_shows_parent_column_when_children_present() {
    let (_dir, db_path) = setup_test_db();
    let parent_id = create_task(&db_path, "Parent task");
    create_child_task(&db_path, "Child task", parent_id);

    let output = Command::cargo_bin("brain")
        .unwrap()
        .args(["--db", &db_path, "task", "list", "--format", "table"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(
        text.contains("PARENT"),
        "expected PARENT column when subtasks present: {text}"
    );
}

#[test]
fn task_list_table_omits_parent_column_for_root_only_tasks() {
    let (_dir, db_path) = setup_test_db();
    create_task(&db_path, "Root only task");

    let output = Command::cargo_bin("brain")
        .unwrap()
        .args(["--db", &db_path, "task", "list", "--format", "table"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(
        !text.contains("PARENT"),
        "PARENT column should be absent for root-only tasks: {text}"
    );
}

// --- task update --description ---

#[test]
fn task_update_description_flag() {
    let (_dir, db_path) = setup_test_db();
    Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "task",
            "create",
            "--title",
            "Desc test",
            "--created-by",
            "Puck",
        ])
        .output()
        .unwrap();
    let output = Command::cargo_bin("brain")
        .unwrap()
        .args([
            "--db",
            &db_path,
            "task",
            "update",
            "1",
            "--description",
            "new description text",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "task update --description should succeed: {:?}",
        output
    );
    // Verify the description was stored
    let get_output = Command::cargo_bin("brain")
        .unwrap()
        .args(["--db", &db_path, "task", "get", "1"])
        .output()
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&get_output.stdout).unwrap();
    assert_eq!(v["description"], "new description text");
}
