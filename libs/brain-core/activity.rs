use crate::db::Database;
use crate::error::BrainError;
use crate::types::TaskId;
use serde_json::json;

/// Append an entry to the activity log.
///
/// Returns a JSON object with the new `activity_id` on success.
pub fn activity_log(
    db: &Database,
    actor: String,
    action: String,
    task_id: Option<TaskId>,
    target_type: Option<String>,
    target_id: Option<i64>,
    summary: Option<String>,
) -> Result<String, BrainError> {
    let id = db.conn.query_row(
        "INSERT INTO activity_log (actor, action, task_id, target_type, target_id, summary)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         RETURNING id",
        rusqlite::params![
            actor,
            action,
            task_id.map(|t| t.0),
            target_type,
            target_id,
            summary,
        ],
        |row| row.get::<_, i64>(0),
    )?;

    Ok(serde_json::to_string(&json!({ "activity_id": id })).unwrap())
}

/// List activity log entries, optionally filtered by actor, task ID, and target type.
///
/// Returns a JSON array of matching activity log records in reverse insertion order.
pub fn activity_list(
    db: &Database,
    actor: Option<&str>,
    task_id: Option<&TaskId>,
    target_type: Option<&str>,
    limit: u32,
) -> Result<String, BrainError> {
    let mut sql = String::from(
        "SELECT id, actor, action, task_id, target_type, target_id, summary, created_at
         FROM activity_log WHERE 1=1",
    );
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(a) = actor {
        params.push(Box::new(a.to_string()));
        sql.push_str(&format!(" AND actor = ?{}", params.len()));
    }
    if let Some(tid) = task_id {
        params.push(Box::new(tid.0));
        sql.push_str(&format!(" AND task_id = ?{}", params.len()));
    }
    if let Some(tt) = target_type {
        params.push(Box::new(tt.to_string()));
        sql.push_str(&format!(" AND target_type = ?{}", params.len()));
    }

    params.push(Box::new(limit as i64));
    sql.push_str(&format!(" ORDER BY id DESC LIMIT ?{}", params.len()));

    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let mut stmt = db.conn.prepare(&sql)?;
    let rows = stmt.query_map(param_refs.as_slice(), |row| {
        Ok(json!({
            "id": row.get::<_, i64>(0)?,
            "actor": row.get::<_, String>(1)?,
            "action": row.get::<_, String>(2)?,
            "task_id": row.get::<_, Option<i64>>(3)?,
            "target_type": row.get::<_, Option<String>>(4)?,
            "target_id": row.get::<_, Option<i64>>(5)?,
            "summary": row.get::<_, Option<String>>(6)?,
            "created_at": row.get::<_, String>(7)?,
        }))
    })?;

    let results: Vec<serde_json::Value> = rows.collect::<Result<Vec<_>, _>>()?;
    Ok(serde_json::to_string(&results).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::types::TaskId;

    fn test_db() -> Database {
        Database::in_memory().unwrap()
    }

    #[test]
    fn activity_log_returns_id() {
        let db = test_db();
        let result = activity_log(
            &db,
            "Puck".to_string(),
            "dispatched".to_string(),
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["activity_id"], 1);
    }

    #[test]
    fn activity_log_with_all_fields() {
        let db = test_db();
        let result = activity_log(
            &db,
            "Forge".to_string(),
            "completed".to_string(),
            Some(TaskId(42)),
            Some("task".to_string()),
            Some(42),
            Some("Finished the work".to_string()),
        )
        .unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(v["activity_id"].as_i64().unwrap() > 0);
    }

    #[test]
    fn activity_list_returns_array() {
        let db = test_db();
        activity_log(
            &db,
            "Puck".to_string(),
            "dispatched".to_string(),
            None,
            None,
            None,
            None,
        )
        .unwrap();
        activity_log(
            &db,
            "Forge".to_string(),
            "completed".to_string(),
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let result = activity_list(&db, None, None, None, 20).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v.as_array().unwrap().len(), 2);
    }

    #[test]
    fn activity_list_filters_by_actor() {
        let db = test_db();
        activity_log(
            &db,
            "Puck".to_string(),
            "dispatched".to_string(),
            None,
            None,
            None,
            None,
        )
        .unwrap();
        activity_log(
            &db,
            "Forge".to_string(),
            "completed".to_string(),
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let result = activity_list(&db, Some("Puck"), None, None, 20).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v.as_array().unwrap().len(), 1);
        assert_eq!(v[0]["actor"], "Puck");
    }

    #[test]
    fn activity_list_respects_limit() {
        let db = test_db();
        for i in 0..5 {
            activity_log(
                &db,
                "Puck".to_string(),
                format!("action_{i}"),
                None,
                None,
                None,
                None,
            )
            .unwrap();
        }
        let result = activity_list(&db, None, None, None, 3).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v.as_array().unwrap().len(), 3);
    }
}
