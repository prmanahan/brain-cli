use crate::db::Database;
use crate::error::BrainError;
use crate::types::*;
use serde_json::json;

#[allow(clippy::too_many_arguments)]
pub fn dispatch_start(
    db: &Database,
    task_id: TaskId,
    agent: Agent,
    provider: Provider,
    model: Model,
    tier: Option<DispatchTier>,
    task_pattern: Option<String>,
    tier_justification: Option<String>,
    dispatch_context_kb: Option<f64>,
) -> Result<String, BrainError> {
    let dispatch_number: i64 = db.conn.query_row(
        "SELECT COALESCE(MAX(dispatch_number), 0) + 1 FROM dispatch_metrics WHERE task_id = ?1",
        [task_id.0],
        |row| row.get(0),
    )?;

    let id = db.conn.query_row(
        "INSERT INTO dispatch_metrics (task_id, agent, provider, model, dispatch_number, status,
                                       dispatch_tier, task_pattern, tier_justification, dispatch_context_kb)
         VALUES (?1, ?2, ?3, ?4, ?5, 'in_progress', ?6, ?7, ?8, ?9)
         RETURNING id",
        rusqlite::params![
            task_id.0,
            agent.0,
            provider.to_string(),
            model.0,
            dispatch_number,
            tier.map(|t| t.to_string()),
            task_pattern,
            tier_justification,
            dispatch_context_kb,
        ],
        |row| row.get::<_, i64>(0),
    )?;

    Ok(serde_json::to_string(&json!({
        "dispatch_id": id,
        "dispatch_number": dispatch_number
    }))
    .unwrap())
}

#[allow(clippy::too_many_arguments)]
pub fn dispatch_complete(
    db: &Database,
    id: DispatchId,
    status: DispatchStatus,
    tokens_input: Option<u64>,
    tokens_output: Option<u64>,
    tokens_total: Option<u64>,
    duration_ms: Option<u64>,
    tool_uses: Option<u32>,
    cost_usd: Option<f64>,
) -> Result<String, BrainError> {
    let tokens_total = match (tokens_input, tokens_output, tokens_total) {
        (Some(i), Some(o), _) => Some(i + o),
        (_, _, Some(t)) => Some(t),
        _ => None,
    };

    let rows = db.conn.execute(
        "UPDATE dispatch_metrics
         SET status = ?1, tokens_input = ?2, tokens_output = ?3, tokens_total = ?4,
             duration_ms = ?5, tool_uses = ?6, cost_usd = ?7
         WHERE id = ?8",
        rusqlite::params![
            status.to_string(),
            tokens_input.map(|v| v as i64),
            tokens_output.map(|v| v as i64),
            tokens_total.map(|v| v as i64),
            duration_ms.map(|v| v as i64),
            tool_uses.map(|v| v as i32),
            cost_usd,
            id.0,
        ],
    )?;

    if rows == 0 {
        return Err(BrainError::NotFound {
            entity: "dispatch",
            id: id.0,
        });
    }

    Ok(serde_json::to_string(&json!({
        "dispatch_id": id.0,
        "status": status.to_string()
    }))
    .unwrap())
}

pub fn dispatch_event(
    db: &Database,
    dispatch_id: DispatchId,
    event_type: EventType,
    severity: Severity,
    description: String,
) -> Result<String, BrainError> {
    let id = db.conn.query_row(
        "INSERT INTO dispatch_events (dispatch_id, event_type, severity, description)
         VALUES (?1, ?2, ?3, ?4)
         RETURNING id",
        rusqlite::params![
            dispatch_id.0,
            event_type.to_string(),
            severity.to_string(),
            description,
        ],
        |row| row.get::<_, i64>(0),
    )?;

    Ok(serde_json::to_string(&json!({ "event_id": id })).unwrap())
}

pub fn dispatch_get(db: &Database, id: DispatchId) -> Result<String, BrainError> {
    let result = db.conn.query_row(
        "SELECT id, task_id, agent, provider, model, dispatch_number, status,
                tokens_input, tokens_output, tokens_total, duration_ms,
                cost_usd, tool_uses, created_at,
                dispatch_tier, task_pattern, tier_justification, dispatch_context_kb
         FROM dispatch_metrics WHERE id = ?1",
        [id.0],
        |row| {
            Ok(json!({
                "id": row.get::<_, i64>(0)?,
                "task_id": row.get::<_, Option<i64>>(1)?,
                "agent": row.get::<_, String>(2)?,
                "provider": row.get::<_, Option<String>>(3)?,
                "model": row.get::<_, String>(4)?,
                "dispatch_number": row.get::<_, i64>(5)?,
                "status": row.get::<_, String>(6)?,
                "tokens_input": row.get::<_, Option<i64>>(7)?,
                "tokens_output": row.get::<_, Option<i64>>(8)?,
                "tokens_total": row.get::<_, Option<i64>>(9)?,
                "duration_ms": row.get::<_, Option<i64>>(10)?,
                "cost_usd": row.get::<_, Option<f64>>(11)?,
                "tool_uses": row.get::<_, Option<i32>>(12)?,
                "created_at": row.get::<_, String>(13)?,
                "dispatch_tier": row.get::<_, Option<String>>(14)?,
                "task_pattern": row.get::<_, Option<String>>(15)?,
                "tier_justification": row.get::<_, Option<String>>(16)?,
                "dispatch_context_kb": row.get::<_, Option<f64>>(17)?,
            }))
        },
    );

    match result {
        Ok(v) => Ok(serde_json::to_string(&v).unwrap()),
        Err(rusqlite::Error::QueryReturnedNoRows) => Err(BrainError::NotFound {
            entity: "dispatch",
            id: id.0,
        }),
        Err(e) => Err(BrainError::from(e)),
    }
}

pub fn dispatch_list(
    db: &Database,
    task_id: Option<&TaskId>,
    agent: Option<&Agent>,
    status: Option<&DispatchStatus>,
    tier: Option<&DispatchTier>,
) -> Result<String, BrainError> {
    let mut sql = String::from(
        "SELECT id, task_id, agent, provider, model, dispatch_number, status,
                tokens_input, tokens_output, tokens_total, duration_ms,
                cost_usd, tool_uses, created_at,
                dispatch_tier, task_pattern, tier_justification, dispatch_context_kb
         FROM dispatch_metrics WHERE 1=1",
    );
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(tid) = task_id {
        params.push(Box::new(tid.0));
        sql.push_str(&format!(" AND task_id = ?{}", params.len()));
    }
    if let Some(a) = agent {
        params.push(Box::new(a.0.clone()));
        sql.push_str(&format!(" AND agent = ?{}", params.len()));
    }
    if let Some(s) = status {
        params.push(Box::new(s.to_string()));
        sql.push_str(&format!(" AND status = ?{}", params.len()));
    }
    if let Some(t) = tier {
        params.push(Box::new(t.to_string()));
        sql.push_str(&format!(" AND dispatch_tier = ?{}", params.len()));
    }

    sql.push_str(" ORDER BY id DESC");

    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let mut stmt = db.conn.prepare(&sql)?;
    let rows = stmt.query_map(param_refs.as_slice(), |row| {
        Ok(json!({
            "id": row.get::<_, i64>(0)?,
            "task_id": row.get::<_, Option<i64>>(1)?,
            "agent": row.get::<_, String>(2)?,
            "provider": row.get::<_, Option<String>>(3)?,
            "model": row.get::<_, String>(4)?,
            "dispatch_number": row.get::<_, i64>(5)?,
            "status": row.get::<_, String>(6)?,
            "tokens_input": row.get::<_, Option<i64>>(7)?,
            "tokens_output": row.get::<_, Option<i64>>(8)?,
            "tokens_total": row.get::<_, Option<i64>>(9)?,
            "duration_ms": row.get::<_, Option<i64>>(10)?,
            "cost_usd": row.get::<_, Option<f64>>(11)?,
            "tool_uses": row.get::<_, Option<i32>>(12)?,
            "created_at": row.get::<_, String>(13)?,
            "dispatch_tier": row.get::<_, Option<String>>(14)?,
            "task_pattern": row.get::<_, Option<String>>(15)?,
            "tier_justification": row.get::<_, Option<String>>(16)?,
            "dispatch_context_kb": row.get::<_, Option<f64>>(17)?,
        }))
    })?;

    let results: Vec<serde_json::Value> = rows.collect::<Result<Vec<_>, _>>()?;
    Ok(serde_json::to_string(&results).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn test_db() -> Database {
        let db = Database::in_memory().unwrap();
        db.conn
            .execute_batch(
                "INSERT INTO tasks (id, title, created_by) VALUES (1, 'Test task', 'Puck');",
            )
            .unwrap();
        db
    }

    #[test]
    fn dispatch_start_returns_id_and_dispatch_number() {
        let db = test_db();
        let result = dispatch_start(
            &db,
            TaskId(1),
            Agent("Rune".to_string()),
            Provider::Anthropic,
            Model("sonnet".to_string()),
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["dispatch_id"], 1);
        assert_eq!(v["dispatch_number"], 1);
    }

    #[test]
    fn dispatch_start_increments_dispatch_number() {
        let db = test_db();
        dispatch_start(
            &db,
            TaskId(1),
            Agent("Rune".to_string()),
            Provider::Anthropic,
            Model("sonnet".to_string()),
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let result = dispatch_start(
            &db,
            TaskId(1),
            Agent("Rune".to_string()),
            Provider::Anthropic,
            Model("sonnet".to_string()),
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["dispatch_number"], 2);
    }

    #[test]
    fn dispatch_complete_updates_status() {
        let db = test_db();
        dispatch_start(
            &db,
            TaskId(1),
            Agent("Rune".to_string()),
            Provider::Anthropic,
            Model("sonnet".to_string()),
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let result = dispatch_complete(
            &db,
            DispatchId(1),
            DispatchStatus::Completed,
            Some(1500),
            Some(3200),
            None,
            Some(120_000),
            Some(45),
            Some(0.21),
        )
        .unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["dispatch_id"], 1);
        assert_eq!(v["status"], "completed");
    }

    #[test]
    fn dispatch_complete_not_found() {
        let db = test_db();
        let result = dispatch_complete(
            &db,
            DispatchId(999),
            DispatchStatus::Completed,
            None,
            None,
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
    fn dispatch_event_returns_event_id() {
        let db = test_db();
        dispatch_start(
            &db,
            TaskId(1),
            Agent("Rune".to_string()),
            Provider::Anthropic,
            Model("sonnet".to_string()),
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let result = dispatch_event(
            &db,
            DispatchId(1),
            EventType::Routing,
            Severity::Info,
            "Dispatched to Rune for Rust implementation".to_string(),
        )
        .unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["event_id"], 1);
    }

    #[test]
    fn dispatch_get_returns_full_record() {
        let db = test_db();
        dispatch_start(
            &db,
            TaskId(1),
            Agent("Rune".to_string()),
            Provider::Anthropic,
            Model("sonnet".to_string()),
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let result = dispatch_get(&db, DispatchId(1)).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["id"], 1);
        assert_eq!(v["agent"], "Rune");
        assert_eq!(v["provider"], "anthropic");
        assert_eq!(v["model"], "sonnet");
        assert_eq!(v["status"], "in_progress");
    }

    #[test]
    fn dispatch_get_not_found() {
        let db = test_db();
        let result = dispatch_get(&db, DispatchId(999));
        assert!(result.is_err());
    }

    #[test]
    fn dispatch_list_returns_array() {
        let db = test_db();
        dispatch_start(
            &db,
            TaskId(1),
            Agent("Rune".to_string()),
            Provider::Anthropic,
            Model("sonnet".to_string()),
            None,
            None,
            None,
            None,
        )
        .unwrap();
        dispatch_start(
            &db,
            TaskId(1),
            Agent("Sage".to_string()),
            Provider::Google,
            Model("gemini-2.0-flash".to_string()),
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let result = dispatch_list(&db, None, None, None, None).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v.as_array().unwrap().len(), 2);
    }

    #[test]
    fn dispatch_list_filters_by_agent() {
        let db = test_db();
        dispatch_start(
            &db,
            TaskId(1),
            Agent("Rune".to_string()),
            Provider::Anthropic,
            Model("sonnet".to_string()),
            None,
            None,
            None,
            None,
        )
        .unwrap();
        dispatch_start(
            &db,
            TaskId(1),
            Agent("Sage".to_string()),
            Provider::Google,
            Model("gemini-2.0-flash".to_string()),
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let result =
            dispatch_list(&db, None, Some(&Agent("Rune".to_string())), None, None).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v.as_array().unwrap().len(), 1);
        assert_eq!(v[0]["agent"], "Rune");
    }

    #[test]
    fn dispatch_start_with_tier_fields_persists_to_get() {
        let db = test_db();
        dispatch_start(
            &db,
            TaskId(1),
            Agent("Forge".to_string()),
            Provider::Anthropic,
            Model("sonnet".to_string()),
            Some(DispatchTier::T2),
            Some("implementation".to_string()),
            Some("Sonnet fits implementation tasks".to_string()),
            None,
        )
        .unwrap();
        let result = dispatch_get(&db, DispatchId(1)).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["dispatch_tier"], "t2");
        assert_eq!(v["task_pattern"], "implementation");
        assert_eq!(v["tier_justification"], "Sonnet fits implementation tasks");
    }

    #[test]
    fn dispatch_start_without_tier_fields_has_null_in_get() {
        let db = test_db();
        dispatch_start(
            &db,
            TaskId(1),
            Agent("Rune".to_string()),
            Provider::Anthropic,
            Model("sonnet".to_string()),
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let result = dispatch_get(&db, DispatchId(1)).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(v["dispatch_tier"].is_null());
        assert!(v["task_pattern"].is_null());
        assert!(v["tier_justification"].is_null());
        assert!(v["dispatch_context_kb"].is_null());
    }

    #[test]
    fn dispatch_list_filters_by_tier() {
        let db = test_db();
        dispatch_start(
            &db,
            TaskId(1),
            Agent("Rune".to_string()),
            Provider::Anthropic,
            Model("opus".to_string()),
            Some(DispatchTier::T1),
            Some("code_review".to_string()),
            None,
            None,
        )
        .unwrap();
        dispatch_start(
            &db,
            TaskId(1),
            Agent("Forge".to_string()),
            Provider::Anthropic,
            Model("sonnet".to_string()),
            Some(DispatchTier::T2),
            Some("implementation".to_string()),
            None,
            None,
        )
        .unwrap();
        let result = dispatch_list(&db, None, None, None, Some(&DispatchTier::T2)).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v.as_array().unwrap().len(), 1);
        assert_eq!(v[0]["dispatch_tier"], "t2");
    }

    #[test]
    fn dispatch_start_with_context_kb_persists() {
        let db = test_db();
        dispatch_start(
            &db,
            TaskId(1),
            Agent("Forge".to_string()),
            Provider::Anthropic,
            Model("sonnet".to_string()),
            None,
            None,
            None,
            Some(11.5),
        )
        .unwrap();
        let result = dispatch_get(&db, DispatchId(1)).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["dispatch_context_kb"], 11.5);
    }
}
