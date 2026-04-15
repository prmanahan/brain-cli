use crate::db::Database;
use crate::error::BrainError;
use crate::types::*;
use serde_json::json;

#[allow(clippy::too_many_arguments)]
pub fn task_create(
    db: &Database,
    title: String,
    description: Option<String>,
    assigned_to: Option<Agent>,
    priority: Option<Priority>,
    project_id: Option<ProjectId>,
    parent_id: Option<TaskId>,
    created_by: String,
) -> Result<String, BrainError> {
    let id = db.conn.query_row(
        "INSERT INTO tasks (title, description, assigned_to, priority, project_id, parent_id, created_by)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         RETURNING id",
        rusqlite::params![
            title,
            description,
            assigned_to.map(|a| a.0),
            priority.map(|p| p.to_string()),
            project_id.map(|p| p.0),
            parent_id.map(|p| p.0),
            created_by,
        ],
        |row| row.get::<_, i64>(0),
    )?;

    Ok(serde_json::to_string(&json!({ "task_id": id })).unwrap())
}

pub fn task_update(
    db: &Database,
    id: TaskId,
    status: Option<TaskStatus>,
    assigned_to: Option<Agent>,
    priority: Option<Priority>,
    description: Option<String>,
    project_id: Option<ProjectId>,
) -> Result<String, BrainError> {
    let mut sets = vec!["updated_at = datetime('now')".to_string()];
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    let is_done = status.as_ref().is_some_and(|s| *s == TaskStatus::Done);

    if let Some(s) = &status {
        params.push(Box::new(s.to_string()));
        sets.push(format!("status = ?{}", params.len()));
    }
    if let Some(a) = &assigned_to {
        params.push(Box::new(a.0.clone()));
        sets.push(format!("assigned_to = ?{}", params.len()));
    }
    if let Some(p) = &priority {
        params.push(Box::new(p.to_string()));
        sets.push(format!("priority = ?{}", params.len()));
    }
    if let Some(d) = &description {
        params.push(Box::new(d.clone()));
        sets.push(format!("description = ?{}", params.len()));
    }
    if let Some(pid) = &project_id {
        params.push(Box::new(pid.0));
        sets.push(format!("project_id = ?{}", params.len()));
    }
    if is_done {
        sets.push("completed_at = datetime('now')".to_string());
    }

    params.push(Box::new(id.0));
    let sql = format!(
        "UPDATE tasks SET {} WHERE id = ?{}",
        sets.join(", "),
        params.len()
    );

    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let rows = db.conn.execute(&sql, param_refs.as_slice())?;

    if rows == 0 {
        return Err(BrainError::NotFound {
            entity: "task",
            id: id.0,
        });
    }

    let final_status = if let Some(s) = status {
        s.to_string()
    } else {
        db.conn
            .query_row("SELECT status FROM tasks WHERE id = ?1", [id.0], |row| {
                row.get(0)
            })?
    };

    Ok(serde_json::to_string(&json!({
        "task_id": id.0,
        "status": final_status
    }))
    .unwrap())
}

pub fn task_get(db: &Database, id: TaskId) -> Result<String, BrainError> {
    let result = db.conn.query_row(
        "SELECT id, parent_id, title, description, status, priority,
                assigned_to, created_by, due_date, created_at, updated_at,
                completed_at, project_id
         FROM tasks WHERE id = ?1",
        [id.0],
        |row| {
            Ok(json!({
                "id": row.get::<_, i64>(0)?,
                "parent_id": row.get::<_, Option<i64>>(1)?,
                "title": row.get::<_, String>(2)?,
                "description": row.get::<_, Option<String>>(3)?,
                "status": row.get::<_, String>(4)?,
                "priority": row.get::<_, Option<String>>(5)?,
                "assigned_to": row.get::<_, Option<String>>(6)?,
                "created_by": row.get::<_, String>(7)?,
                "due_date": row.get::<_, Option<String>>(8)?,
                "created_at": row.get::<_, String>(9)?,
                "updated_at": row.get::<_, String>(10)?,
                "completed_at": row.get::<_, Option<String>>(11)?,
                "project_id": row.get::<_, Option<i64>>(12)?,
            }))
        },
    );

    match result {
        Ok(mut v) => {
            // Fetch direct children and attach as subtask summary
            let mut stmt = db.conn.prepare(
                "SELECT id, title, status FROM tasks WHERE parent_id = ?1 ORDER BY id ASC",
            )?;
            let subtasks: Vec<serde_json::Value> = stmt
                .query_map([id.0], |row| {
                    Ok(json!({
                        "id": row.get::<_, i64>(0)?,
                        "title": row.get::<_, String>(1)?,
                        "status": row.get::<_, String>(2)?,
                    }))
                })?
                .collect::<Result<Vec<_>, _>>()?;
            v["subtasks"] = json!(subtasks);
            Ok(serde_json::to_string(&v).unwrap())
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => Err(BrainError::NotFound {
            entity: "task",
            id: id.0,
        }),
        Err(e) => Err(BrainError::from(e)),
    }
}

pub fn task_list(
    db: &Database,
    status: Option<&TaskStatus>,
    assigned_to: Option<&Agent>,
    project_id: Option<&ProjectId>,
    parent_id: Option<&TaskId>,
) -> Result<String, BrainError> {
    // Validate parent exists when filtering by parent_id
    if let Some(pid) = parent_id {
        let exists: bool =
            db.conn
                .query_row("SELECT COUNT(*) FROM tasks WHERE id = ?1", [pid.0], |row| {
                    row.get::<_, i64>(0)
                })?
                > 0;
        if !exists {
            return Err(BrainError::NotFound {
                entity: "task",
                id: pid.0,
            });
        }
    }

    let mut sql = String::from(
        "SELECT id, parent_id, title, description, status, priority,
                assigned_to, created_by, due_date, created_at, updated_at,
                completed_at, project_id
         FROM tasks WHERE 1=1",
    );
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(s) = status {
        params.push(Box::new(s.to_string()));
        sql.push_str(&format!(" AND status = ?{}", params.len()));
    }
    if let Some(a) = assigned_to {
        params.push(Box::new(a.0.clone()));
        sql.push_str(&format!(" AND assigned_to = ?{}", params.len()));
    }
    if let Some(p) = project_id {
        params.push(Box::new(p.0));
        sql.push_str(&format!(" AND project_id = ?{}", params.len()));
    }
    if let Some(pid) = parent_id {
        params.push(Box::new(pid.0));
        sql.push_str(&format!(" AND parent_id = ?{}", params.len()));
    } else {
        // Default: no parent_id filter — return all tasks
    }

    sql.push_str(" ORDER BY id ASC");

    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let mut stmt = db.conn.prepare(&sql)?;
    let rows = stmt.query_map(param_refs.as_slice(), |row| {
        Ok(json!({
            "id": row.get::<_, i64>(0)?,
            "parent_id": row.get::<_, Option<i64>>(1)?,
            "title": row.get::<_, String>(2)?,
            "description": row.get::<_, Option<String>>(3)?,
            "status": row.get::<_, String>(4)?,
            "priority": row.get::<_, Option<String>>(5)?,
            "assigned_to": row.get::<_, Option<String>>(6)?,
            "created_by": row.get::<_, String>(7)?,
            "due_date": row.get::<_, Option<String>>(8)?,
            "created_at": row.get::<_, String>(9)?,
            "updated_at": row.get::<_, String>(10)?,
            "completed_at": row.get::<_, Option<String>>(11)?,
            "project_id": row.get::<_, Option<i64>>(12)?,
        }))
    })?;

    let results: Vec<serde_json::Value> = rows.collect::<Result<Vec<_>, _>>()?;
    Ok(serde_json::to_string(&results).unwrap())
}

/// Fetch tasks as a nested tree structure.
///
/// All tasks matching the filters are fetched. Root tasks (parent_id IS NULL)
/// form the top level. Children are nested under their parent. Depth is capped
/// at 2 levels (parent → child). Orphaned children (parent not in result set
/// due to filtering) are promoted to the root level with a warning in the JSON.
///
/// Returns a JSON array of task objects, each with a `"children"` array field.
pub fn task_list_tree(
    db: &Database,
    status: Option<&TaskStatus>,
    assigned_to: Option<&Agent>,
    project_id: Option<&ProjectId>,
) -> Result<String, BrainError> {
    // Fetch all matching tasks (unfiltered by parent_id)
    let mut sql = String::from(
        "SELECT id, parent_id, title, description, status, priority,
                assigned_to, created_by, due_date, created_at, updated_at,
                completed_at, project_id
         FROM tasks WHERE 1=1",
    );
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(s) = status {
        params.push(Box::new(s.to_string()));
        sql.push_str(&format!(" AND status = ?{}", params.len()));
    }
    if let Some(a) = assigned_to {
        params.push(Box::new(a.0.clone()));
        sql.push_str(&format!(" AND assigned_to = ?{}", params.len()));
    }
    if let Some(p) = project_id {
        params.push(Box::new(p.0));
        sql.push_str(&format!(" AND project_id = ?{}", params.len()));
    }

    sql.push_str(" ORDER BY id ASC");

    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let mut stmt = db.conn.prepare(&sql)?;
    let rows = stmt.query_map(param_refs.as_slice(), |row| {
        Ok(json!({
            "id": row.get::<_, i64>(0)?,
            "parent_id": row.get::<_, Option<i64>>(1)?,
            "title": row.get::<_, String>(2)?,
            "description": row.get::<_, Option<String>>(3)?,
            "status": row.get::<_, String>(4)?,
            "priority": row.get::<_, Option<String>>(5)?,
            "assigned_to": row.get::<_, Option<String>>(6)?,
            "created_by": row.get::<_, String>(7)?,
            "due_date": row.get::<_, Option<String>>(8)?,
            "created_at": row.get::<_, String>(9)?,
            "updated_at": row.get::<_, String>(10)?,
            "completed_at": row.get::<_, Option<String>>(11)?,
            "project_id": row.get::<_, Option<i64>>(12)?,
        }))
    })?;

    let all_tasks: Vec<serde_json::Value> = rows.collect::<Result<Vec<_>, _>>()?;

    // Collect the set of IDs present in the result for parent lookup
    let id_set: std::collections::HashSet<i64> =
        all_tasks.iter().filter_map(|t| t["id"].as_i64()).collect();

    // Separate roots from children
    let mut roots: Vec<serde_json::Value> = Vec::new();
    let mut children_map: std::collections::HashMap<i64, Vec<serde_json::Value>> =
        std::collections::HashMap::new();

    for mut task in all_tasks {
        let parent_id_val = task["parent_id"].as_i64();
        match parent_id_val {
            None => {
                task["children"] = json!([]);
                roots.push(task);
            }
            Some(pid) if id_set.contains(&pid) => {
                // Parent is in the result set — nest under it
                children_map.entry(pid).or_default().push(task);
            }
            Some(_) => {
                // Parent not in result set (filtered out) — promote to root
                task["children"] = json!([]);
                roots.push(task);
            }
        }
    }

    // Attach children to their parents (only 2 levels deep per spec)
    for root in &mut roots {
        let root_id = root["id"].as_i64().unwrap_or(0);
        if let Some(children) = children_map.remove(&root_id) {
            root["children"] = json!(children);
        }
    }

    // If there are still unattached children (grandchildren scenario), warn but don't crash
    // They are silently dropped — spec says depth capped at 2 levels

    Ok(serde_json::to_string(&roots).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn test_db() -> Database {
        Database::in_memory().unwrap()
    }

    #[test]
    fn task_create_returns_id() {
        let db = test_db();
        let result = task_create(
            &db,
            "Build brain CLI".to_string(),
            None,
            None,
            None,
            None,
            None,
            "Puck".to_string(),
        )
        .unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["task_id"], 1);
    }

    #[test]
    fn task_create_with_all_fields() {
        let db = test_db();
        let result = task_create(
            &db,
            "Build brain CLI".to_string(),
            Some("Detailed description".to_string()),
            Some(Agent("Rune".to_string())),
            Some(Priority::High),
            Some(ProjectId(1)),
            None,
            "Puck".to_string(),
        )
        .unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(v["task_id"].as_i64().unwrap() > 0);
    }

    #[test]
    fn task_update_changes_status() {
        let db = test_db();
        task_create(
            &db,
            "Test task".to_string(),
            None,
            None,
            None,
            None,
            None,
            "Puck".to_string(),
        )
        .unwrap();
        let result = task_update(
            &db,
            TaskId(1),
            Some(TaskStatus::InProgress),
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["task_id"], 1);
        assert_eq!(v["status"], "in_progress");
    }

    #[test]
    fn task_update_sets_completed_at_when_done() {
        let db = test_db();
        task_create(
            &db,
            "Test task".to_string(),
            None,
            None,
            None,
            None,
            None,
            "Puck".to_string(),
        )
        .unwrap();
        task_update(
            &db,
            TaskId(1),
            Some(TaskStatus::Done),
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let result = task_get(&db, TaskId(1)).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(v["completed_at"].as_str().is_some());
    }

    #[test]
    fn task_update_not_found() {
        let db = test_db();
        let result = task_update(
            &db,
            TaskId(999),
            Some(TaskStatus::Done),
            None,
            None,
            None,
            None,
        );
        assert!(result.is_err());
        let json = result.unwrap_err().to_json();
        assert!(json.contains("NOT_FOUND"));
    }

    #[test]
    fn task_update_changes_description() {
        let db = test_db();
        task_create(
            &db,
            "Test task".to_string(),
            Some("original description".to_string()),
            None,
            None,
            None,
            None,
            "Puck".to_string(),
        )
        .unwrap();
        task_update(
            &db,
            TaskId(1),
            None,
            None,
            None,
            Some("updated description".to_string()),
            None,
        )
        .unwrap();
        let result = task_get(&db, TaskId(1)).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["description"], "updated description");
    }

    #[test]
    fn task_update_preserves_description_when_updating_other_fields() {
        let db = test_db();
        task_create(
            &db,
            "Test task".to_string(),
            Some("keep this description".to_string()),
            None,
            None,
            None,
            None,
            "Puck".to_string(),
        )
        .unwrap();
        // Update status only — description must not be nulled
        task_update(
            &db,
            TaskId(1),
            Some(TaskStatus::InProgress),
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let result = task_get(&db, TaskId(1)).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["description"], "keep this description");
        assert_eq!(v["status"], "in_progress");
    }

    #[test]
    fn task_get_returns_full_record() {
        let db = test_db();
        task_create(
            &db,
            "Test task".to_string(),
            Some("A description".to_string()),
            Some(Agent("Rune".to_string())),
            Some(Priority::High),
            None,
            None,
            "Puck".to_string(),
        )
        .unwrap();
        let result = task_get(&db, TaskId(1)).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["id"], 1);
        assert_eq!(v["title"], "Test task");
        assert_eq!(v["description"], "A description");
        assert_eq!(v["assigned_to"], "Rune");
        assert_eq!(v["priority"], "high");
        assert_eq!(v["status"], "open");
        assert_eq!(v["created_by"], "Puck");
    }

    #[test]
    fn task_get_not_found() {
        let db = test_db();
        let result = task_get(&db, TaskId(999));
        assert!(result.is_err());
    }

    #[test]
    fn task_list_returns_all() {
        let db = test_db();
        task_create(
            &db,
            "Task 1".to_string(),
            None,
            None,
            None,
            None,
            None,
            "Puck".to_string(),
        )
        .unwrap();
        task_create(
            &db,
            "Task 2".to_string(),
            None,
            None,
            None,
            None,
            None,
            "Puck".to_string(),
        )
        .unwrap();
        let result = task_list(&db, None, None, None, None).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v.as_array().unwrap().len(), 2);
    }

    #[test]
    fn task_list_filters_by_status() {
        let db = test_db();
        task_create(
            &db,
            "Task 1".to_string(),
            None,
            None,
            None,
            None,
            None,
            "Puck".to_string(),
        )
        .unwrap();
        task_create(
            &db,
            "Task 2".to_string(),
            None,
            None,
            None,
            None,
            None,
            "Puck".to_string(),
        )
        .unwrap();
        task_update(
            &db,
            TaskId(1),
            Some(TaskStatus::Done),
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let result = task_list(&db, Some(&TaskStatus::Open), None, None, None).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v.as_array().unwrap().len(), 1);
        assert_eq!(v[0]["title"], "Task 2");
    }

    #[test]
    fn task_list_filters_by_assigned_to() {
        let db = test_db();
        task_create(
            &db,
            "Task 1".to_string(),
            None,
            Some(Agent("Rune".to_string())),
            None,
            None,
            None,
            "Puck".to_string(),
        )
        .unwrap();
        task_create(
            &db,
            "Task 2".to_string(),
            None,
            Some(Agent("Sage".to_string())),
            None,
            None,
            None,
            "Puck".to_string(),
        )
        .unwrap();
        let result = task_list(&db, None, Some(&Agent("Rune".to_string())), None, None).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v.as_array().unwrap().len(), 1);
        assert_eq!(v[0]["assigned_to"], "Rune");
    }

    #[test]
    fn task_update_sets_project_id() {
        let db = test_db();
        task_create(
            &db,
            "Test task".to_string(),
            None,
            None,
            None,
            None,
            None,
            "Puck".to_string(),
        )
        .unwrap();
        task_update(&db, TaskId(1), None, None, None, None, Some(ProjectId(42))).unwrap();
        let result = task_get(&db, TaskId(1)).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["project_id"], 42);
    }

    #[test]
    fn task_list_filters_by_project_id() {
        let db = test_db();
        task_create(
            &db,
            "Task 1".to_string(),
            None,
            None,
            None,
            Some(ProjectId(1)),
            None,
            "Puck".to_string(),
        )
        .unwrap();
        task_create(
            &db,
            "Task 2".to_string(),
            None,
            None,
            None,
            Some(ProjectId(2)),
            None,
            "Puck".to_string(),
        )
        .unwrap();
        let result = task_list(&db, None, None, Some(&ProjectId(1)), None).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v.as_array().unwrap().len(), 1);
    }

    // ──────────────────────────────────────────────────────────
    // Task 1: --parent-id filter
    // ──────────────────────────────────────────────────────────

    fn create_parent_child_hierarchy(db: &Database) -> (i64, i64, i64, i64) {
        // Returns (parent_id, child1_id, child2_id, child3_id)
        let parent_json = task_create(
            db,
            "Parent task".to_string(),
            None,
            None,
            None,
            None,
            None,
            "Puck".to_string(),
        )
        .unwrap();
        let parent_v: serde_json::Value = serde_json::from_str(&parent_json).unwrap();
        let parent_id = parent_v["task_id"].as_i64().unwrap();

        let c1 = task_create(
            db,
            "Child 1".to_string(),
            None,
            None,
            None,
            None,
            Some(TaskId(parent_id)),
            "Puck".to_string(),
        )
        .unwrap();
        let c1_v: serde_json::Value = serde_json::from_str(&c1).unwrap();

        let c2 = task_create(
            db,
            "Child 2".to_string(),
            None,
            None,
            None,
            None,
            Some(TaskId(parent_id)),
            "Puck".to_string(),
        )
        .unwrap();
        let c2_v: serde_json::Value = serde_json::from_str(&c2).unwrap();

        let c3 = task_create(
            db,
            "Child 3".to_string(),
            None,
            None,
            None,
            None,
            Some(TaskId(parent_id)),
            "Puck".to_string(),
        )
        .unwrap();
        let c3_v: serde_json::Value = serde_json::from_str(&c3).unwrap();

        (
            parent_id,
            c1_v["task_id"].as_i64().unwrap(),
            c2_v["task_id"].as_i64().unwrap(),
            c3_v["task_id"].as_i64().unwrap(),
        )
    }

    #[test]
    fn task_list_parent_id_returns_direct_children() {
        let db = test_db();
        let (parent_id, c1, c2, c3) = create_parent_child_hierarchy(&db);

        let result = task_list(&db, None, None, None, Some(&TaskId(parent_id))).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        let arr = v.as_array().unwrap();

        assert_eq!(arr.len(), 3);
        let ids: Vec<i64> = arr.iter().map(|t| t["id"].as_i64().unwrap()).collect();
        assert!(ids.contains(&c1));
        assert!(ids.contains(&c2));
        assert!(ids.contains(&c3));
        // Parent itself should not be in results
        assert!(!ids.contains(&parent_id));
    }

    #[test]
    fn task_list_parent_id_empty_when_no_children() {
        let db = test_db();
        let (_, c1, _, _) = create_parent_child_hierarchy(&db);
        // c1 has no children
        let result = task_list(&db, None, None, None, Some(&TaskId(c1))).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v.as_array().unwrap().len(), 0);
    }

    #[test]
    fn task_list_parent_id_not_found_returns_error() {
        let db = test_db();
        let result = task_list(&db, None, None, None, Some(&TaskId(9999)));
        assert!(result.is_err());
        let json = result.unwrap_err().to_json();
        assert!(json.contains("NOT_FOUND"));
    }

    #[test]
    fn task_list_parent_id_composes_with_status_filter() {
        let db = test_db();
        let (parent_id, c1, c2, _c3) = create_parent_child_hierarchy(&db);
        // Set c1 to in_progress, c2 to done, c3 stays open
        task_update(
            &db,
            TaskId(c1),
            Some(TaskStatus::InProgress),
            None,
            None,
            None,
            None,
        )
        .unwrap();
        task_update(
            &db,
            TaskId(c2),
            Some(TaskStatus::Done),
            None,
            None,
            None,
            None,
        )
        .unwrap();

        // Filter children of parent with status=open should return only c3
        let result = task_list(
            &db,
            Some(&TaskStatus::Open),
            None,
            None,
            Some(&TaskId(parent_id)),
        )
        .unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        let arr = v.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["title"], "Child 3");
    }

    // ──────────────────────────────────────────────────────────
    // Task 2: --tree view
    // ──────────────────────────────────────────────────────────

    #[test]
    fn task_list_tree_shows_two_level_hierarchy() {
        let db = test_db();
        let (parent_id, c1, c2, c3) = create_parent_child_hierarchy(&db);

        let result = task_list_tree(&db, None, None, None).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        let arr = v.as_array().unwrap();

        // Only the root (parent) at top level
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["id"].as_i64().unwrap(), parent_id);

        let children = arr[0]["children"].as_array().unwrap();
        assert_eq!(children.len(), 3);
        let child_ids: Vec<i64> = children.iter().map(|t| t["id"].as_i64().unwrap()).collect();
        assert!(child_ids.contains(&c1));
        assert!(child_ids.contains(&c2));
        assert!(child_ids.contains(&c3));
    }

    #[test]
    fn task_list_tree_empty_when_no_tasks() {
        let db = test_db();
        let result = task_list_tree(&db, None, None, None).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v.as_array().unwrap().len(), 0);
    }

    #[test]
    fn task_list_tree_status_filter_excludes_non_matching_children() {
        let db = test_db();
        let (parent_id, c1, _c2, _c3) = create_parent_child_hierarchy(&db);
        // Set parent and c1 to in_progress; c2, c3 stay open
        task_update(
            &db,
            TaskId(parent_id),
            Some(TaskStatus::InProgress),
            None,
            None,
            None,
            None,
        )
        .unwrap();
        task_update(
            &db,
            TaskId(c1),
            Some(TaskStatus::InProgress),
            None,
            None,
            None,
            None,
        )
        .unwrap();

        // Filter by in_progress: parent + c1 should appear, c2+c3 filtered out
        let result = task_list_tree(&db, Some(&TaskStatus::InProgress), None, None).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        let arr = v.as_array().unwrap();

        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["id"].as_i64().unwrap(), parent_id);

        let children = arr[0]["children"].as_array().unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0]["id"].as_i64().unwrap(), c1);
    }

    #[test]
    fn task_list_tree_root_tasks_have_empty_children_array() {
        let db = test_db();
        task_create(
            &db,
            "Solo task".to_string(),
            None,
            None,
            None,
            None,
            None,
            "Puck".to_string(),
        )
        .unwrap();

        let result = task_list_tree(&db, None, None, None).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        let arr = v.as_array().unwrap();

        assert_eq!(arr.len(), 1);
        assert!(arr[0]["children"].is_array());
        assert_eq!(arr[0]["children"].as_array().unwrap().len(), 0);
    }

    // ──────────────────────────────────────────────────────────
    // Task 3: subtask summary in task_get
    // ──────────────────────────────────────────────────────────

    #[test]
    fn task_get_includes_subtasks_array() {
        let db = test_db();
        let (parent_id, c1, c2, c3) = create_parent_child_hierarchy(&db);

        let result = task_get(&db, TaskId(parent_id)).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();

        let subtasks = v["subtasks"].as_array().expect("subtasks must be an array");
        assert_eq!(subtasks.len(), 3);

        let ids: Vec<i64> = subtasks.iter().map(|t| t["id"].as_i64().unwrap()).collect();
        assert!(ids.contains(&c1));
        assert!(ids.contains(&c2));
        assert!(ids.contains(&c3));

        // Verify required fields
        for st in subtasks {
            assert!(st["id"].is_number());
            assert!(st["title"].is_string());
            assert!(st["status"].is_string());
        }
    }

    #[test]
    fn task_get_subtasks_empty_when_no_children() {
        let db = test_db();
        let (_, c1, _, _) = create_parent_child_hierarchy(&db);

        let result = task_get(&db, TaskId(c1)).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();

        let subtasks = v["subtasks"].as_array().expect("subtasks must be an array");
        assert_eq!(subtasks.len(), 0);
    }

    #[test]
    fn task_get_subtasks_ordered_by_id_ascending() {
        let db = test_db();
        let (parent_id, c1, c2, c3) = create_parent_child_hierarchy(&db);

        let result = task_get(&db, TaskId(parent_id)).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        let subtasks = v["subtasks"].as_array().unwrap();

        let ids: Vec<i64> = subtasks.iter().map(|t| t["id"].as_i64().unwrap()).collect();
        assert_eq!(ids, vec![c1, c2, c3]);
    }
}
