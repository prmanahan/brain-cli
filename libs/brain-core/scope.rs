use crate::db::Database;
use crate::error::BrainError;
use crate::types::{Agent, DispatchId, TaskId, ViolationType};
use chrono::Utc;
use serde_json::json;

/// Log a scope violation.
///
/// Returns a JSON object with the new `violation_id` on success.
#[allow(clippy::too_many_arguments)]
pub fn scope_log(
    db: &Database,
    agent: Agent,
    task_id: Option<TaskId>,
    dispatch_id: Option<DispatchId>,
    violation_type: ViolationType,
    command_attempted: String,
    context: Option<String>,
    reason: String,
    resolution: Option<String>,
) -> Result<String, BrainError> {
    let created_at = Utc::now().timestamp_millis();

    let id = db.conn.query_row(
        "INSERT INTO scope_violations
             (agent, task_id, dispatch_id, violation_type, command_attempted, context, reason, resolution, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
         RETURNING id",
        rusqlite::params![
            agent.to_string(),
            task_id.map(|t| t.0),
            dispatch_id.map(|d| d.0),
            violation_type.to_string(),
            command_attempted,
            context,
            reason,
            resolution,
            created_at,
        ],
        |row| row.get::<_, i64>(0),
    )?;

    Ok(serde_json::to_string(&json!({ "violation_id": id })).unwrap())
}

/// List scope violations, optionally filtered by agent and/or violation type.
///
/// Results are ordered by id DESC and limited to `limit` rows.
/// Returns a JSON array of violation records.
pub fn scope_list(
    db: &Database,
    agent: Option<&Agent>,
    violation_type: Option<&ViolationType>,
    limit: u32,
) -> Result<String, BrainError> {
    let mut sql = String::from(
        "SELECT id, agent, task_id, dispatch_id, violation_type, command_attempted, context, reason, resolution, created_at
         FROM scope_violations WHERE 1=1",
    );
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(a) = agent {
        params.push(Box::new(a.to_string()));
        sql.push_str(&format!(" AND agent = ?{}", params.len()));
    }
    if let Some(vt) = violation_type {
        params.push(Box::new(vt.to_string()));
        sql.push_str(&format!(" AND violation_type = ?{}", params.len()));
    }

    params.push(Box::new(limit as i64));
    sql.push_str(&format!(" ORDER BY id DESC LIMIT ?{}", params.len()));

    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let mut stmt = db.conn.prepare(&sql)?;
    let rows = stmt.query_map(param_refs.as_slice(), |row| {
        Ok(json!({
            "id": row.get::<_, i64>(0)?,
            "agent": row.get::<_, String>(1)?,
            "task_id": row.get::<_, Option<i64>>(2)?,
            "dispatch_id": row.get::<_, Option<i64>>(3)?,
            "violation_type": row.get::<_, String>(4)?,
            "command_attempted": row.get::<_, String>(5)?,
            "context": row.get::<_, Option<String>>(6)?,
            "reason": row.get::<_, String>(7)?,
            "resolution": row.get::<_, Option<String>>(8)?,
            "created_at": row.get::<_, i64>(9)?,
        }))
    })?;

    let results: Vec<serde_json::Value> = rows.collect::<Result<Vec<_>, _>>()?;
    Ok(serde_json::to_string(&results).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::types::{Agent, DispatchId, TaskId, ViolationType};

    fn test_db() -> Database {
        Database::in_memory().unwrap()
    }

    #[test]
    fn scope_log_returns_id() {
        let db = test_db();
        let result = scope_log(
            &db,
            Agent("Forge".to_string()),
            Some(TaskId(42)),
            Some(DispatchId(7)),
            ViolationType::PermissionDenied,
            "cd /path && git status".to_string(),
            Some("Working on task 42".to_string()),
            "Bash prefix match failed".to_string(),
            Some("Split into two separate commands".to_string()),
        )
        .unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["violation_id"], 1);
    }

    #[test]
    fn scope_log_works_without_optional_fields() {
        let db = test_db();
        let result = scope_log(
            &db,
            Agent("Rune".to_string()),
            None,
            None,
            ViolationType::Scope,
            "npm install lodash".to_string(),
            None,
            "Package installation not in scope".to_string(),
            None,
        )
        .unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(v["violation_id"].as_i64().unwrap() > 0);
    }

    #[test]
    fn scope_log_with_resolution() {
        let db = test_db();
        let result = scope_log(
            &db,
            Agent("Stacks".to_string()),
            None,
            None,
            ViolationType::PermissionDenied,
            "git push --force".to_string(),
            None,
            "Force push blocked by policy".to_string(),
            Some("Used git push --force-with-lease instead".to_string()),
        )
        .unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        let violation_id = v["violation_id"].as_i64().unwrap();
        assert!(violation_id > 0);

        // Verify resolution was stored
        let list_result = scope_list(&db, None, None, 10).unwrap();
        let list: serde_json::Value = serde_json::from_str(&list_result).unwrap();
        assert_eq!(
            list[0]["resolution"],
            "Used git push --force-with-lease instead"
        );
    }

    #[test]
    fn scope_list_returns_array() {
        let db = test_db();
        scope_log(
            &db,
            Agent("Forge".to_string()),
            None,
            None,
            ViolationType::PermissionDenied,
            "rm -rf /".to_string(),
            None,
            "Destructive command blocked".to_string(),
            None,
        )
        .unwrap();
        scope_log(
            &db,
            Agent("Rune".to_string()),
            None,
            None,
            ViolationType::Scope,
            "pip install requests".to_string(),
            None,
            "Out of scope for task".to_string(),
            None,
        )
        .unwrap();
        let result = scope_list(&db, None, None, 20).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v.as_array().unwrap().len(), 2);
    }

    #[test]
    fn scope_list_filters_by_agent() {
        let db = test_db();
        scope_log(
            &db,
            Agent("Forge".to_string()),
            None,
            None,
            ViolationType::PermissionDenied,
            "git reset --hard".to_string(),
            None,
            "Hard reset blocked".to_string(),
            None,
        )
        .unwrap();
        scope_log(
            &db,
            Agent("Rune".to_string()),
            None,
            None,
            ViolationType::PermissionDenied,
            "git push --force".to_string(),
            None,
            "Force push blocked".to_string(),
            None,
        )
        .unwrap();
        let forge = Agent("Forge".to_string());
        let result = scope_list(&db, Some(&forge), None, 20).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v.as_array().unwrap().len(), 1);
        assert_eq!(v[0]["agent"], "Forge");
    }

    #[test]
    fn scope_list_filters_by_violation_type() {
        let db = test_db();
        scope_log(
            &db,
            Agent("Forge".to_string()),
            None,
            None,
            ViolationType::Scope,
            "npm install".to_string(),
            None,
            "Not in scope".to_string(),
            None,
        )
        .unwrap();
        scope_log(
            &db,
            Agent("Forge".to_string()),
            None,
            None,
            ViolationType::CdRequired,
            "cd /project && make build".to_string(),
            None,
            "Chained cd not allowed".to_string(),
            None,
        )
        .unwrap();
        let result = scope_list(&db, None, Some(&ViolationType::CdRequired), 20).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v.as_array().unwrap().len(), 1);
        assert_eq!(v[0]["violation_type"], "cd_required");
    }

    #[test]
    fn scope_list_respects_limit() {
        let db = test_db();
        for i in 0..5 {
            scope_log(
                &db,
                Agent("Forge".to_string()),
                None,
                None,
                ViolationType::PermissionDenied,
                format!("command_{i}"),
                None,
                format!("Reason {i}"),
                None,
            )
            .unwrap();
        }
        let result = scope_list(&db, None, None, 3).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v.as_array().unwrap().len(), 3);
    }
}
