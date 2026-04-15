use crate::db::Database;
use crate::error::BrainError;
use crate::types::{SessionCategory, SessionEventType};
use chrono::NaiveDate;
use serde_json::json;

/// Validate that a date string is in `YYYY-MM-DD` format.
fn parse_session_date(date: &str) -> Result<(), BrainError> {
    NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|_| BrainError::ConstraintViolation {
        description: format!("invalid session_date '{date}': expected format YYYY-MM-DD"),
    })?;
    Ok(())
}

/// Log a session event for the given date.
///
/// Returns a JSON object with the new `event_id` on success.
/// Returns a `ConstraintViolation` error if `session_date` is not in `YYYY-MM-DD` format.
pub fn session_event(
    db: &Database,
    session_date: String,
    event_type: SessionEventType,
    description: String,
    category: Option<SessionCategory>,
) -> Result<String, BrainError> {
    parse_session_date(&session_date)?;

    let id = db.conn.query_row(
        "INSERT INTO session_events (session_date, event_type, category, description)
         VALUES (?1, ?2, ?3, ?4)
         RETURNING id",
        rusqlite::params![
            session_date,
            event_type.to_string(),
            category.map(|c| c.to_string()),
            description,
        ],
        |row| row.get::<_, i64>(0),
    )?;

    Ok(serde_json::to_string(&json!({ "event_id": id })).unwrap())
}

/// List session events, optionally filtered by date, event type, and category.
///
/// `date` must be in `YYYY-MM-DD` format if provided.
/// Returns a JSON array of matching session event records.
pub fn session_list(
    db: &Database,
    date: Option<&str>,
    event_type: Option<&SessionEventType>,
    category: Option<&SessionCategory>,
    limit: u32,
) -> Result<String, BrainError> {
    if let Some(d) = date {
        parse_session_date(d)?;
    }
    let mut sql = String::from(
        "SELECT id, session_date, event_type, category, description, created_at
         FROM session_events WHERE 1=1",
    );
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(d) = date {
        params.push(Box::new(d.to_string()));
        sql.push_str(&format!(" AND session_date = ?{}", params.len()));
    }
    if let Some(et) = event_type {
        params.push(Box::new(et.to_string()));
        sql.push_str(&format!(" AND event_type = ?{}", params.len()));
    }
    if let Some(cat) = category {
        params.push(Box::new(cat.to_string()));
        sql.push_str(&format!(" AND category = ?{}", params.len()));
    }

    params.push(Box::new(limit as i64));
    sql.push_str(&format!(" ORDER BY id DESC LIMIT ?{}", params.len()));

    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let mut stmt = db.conn.prepare(&sql)?;
    let rows = stmt.query_map(param_refs.as_slice(), |row| {
        Ok(json!({
            "id": row.get::<_, i64>(0)?,
            "session_date": row.get::<_, String>(1)?,
            "event_type": row.get::<_, String>(2)?,
            "category": row.get::<_, Option<String>>(3)?,
            "description": row.get::<_, String>(4)?,
            "created_at": row.get::<_, String>(5)?,
        }))
    })?;

    let results: Vec<serde_json::Value> = rows.collect::<Result<Vec<_>, _>>()?;
    Ok(serde_json::to_string(&results).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::types::{SessionCategory, SessionEventType};

    fn test_db() -> Database {
        Database::in_memory().unwrap()
    }

    #[test]
    fn session_event_returns_id() {
        let db = test_db();
        let result = session_event(
            &db,
            "2026-03-29".to_string(),
            SessionEventType::Clarification,
            "Asked about scope".to_string(),
            None,
        )
        .unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["event_id"], 1);
    }

    #[test]
    fn session_event_with_category() {
        let db = test_db();
        let result = session_event(
            &db,
            "2026-03-29".to_string(),
            SessionEventType::Revision,
            "Changed the acceptance criteria".to_string(),
            Some(SessionCategory::AcceptanceCriteria),
        )
        .unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(v["event_id"].as_i64().unwrap() > 0);
    }

    #[test]
    fn session_list_returns_array() {
        let db = test_db();
        session_event(
            &db,
            "2026-03-29".to_string(),
            SessionEventType::Clarification,
            "First event".to_string(),
            None,
        )
        .unwrap();
        session_event(
            &db,
            "2026-03-29".to_string(),
            SessionEventType::Revision,
            "Second event".to_string(),
            None,
        )
        .unwrap();
        let result = session_list(&db, None, None, None, 20).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v.as_array().unwrap().len(), 2);
    }

    #[test]
    fn session_list_filters_by_date() {
        let db = test_db();
        session_event(
            &db,
            "2026-03-29".to_string(),
            SessionEventType::Clarification,
            "Today".to_string(),
            None,
        )
        .unwrap();
        session_event(
            &db,
            "2026-03-28".to_string(),
            SessionEventType::Revision,
            "Yesterday".to_string(),
            None,
        )
        .unwrap();
        let result = session_list(&db, Some("2026-03-29"), None, None, 20).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v.as_array().unwrap().len(), 1);
        assert_eq!(v[0]["session_date"], "2026-03-29");
    }

    #[test]
    fn session_list_filters_by_type() {
        let db = test_db();
        session_event(
            &db,
            "2026-03-29".to_string(),
            SessionEventType::Clarification,
            "A clarification".to_string(),
            None,
        )
        .unwrap();
        session_event(
            &db,
            "2026-03-29".to_string(),
            SessionEventType::Revision,
            "A revision".to_string(),
            None,
        )
        .unwrap();
        let result =
            session_list(&db, None, Some(&SessionEventType::Clarification), None, 20).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v.as_array().unwrap().len(), 1);
        assert_eq!(v[0]["event_type"], "clarification");
    }

    #[test]
    fn session_list_filters_by_category() {
        let db = test_db();
        session_event(
            &db,
            "2026-03-29".to_string(),
            SessionEventType::Clarification,
            "A scope question".to_string(),
            Some(SessionCategory::Scope),
        )
        .unwrap();
        session_event(
            &db,
            "2026-03-29".to_string(),
            SessionEventType::Clarification,
            "A routing question".to_string(),
            Some(SessionCategory::Routing),
        )
        .unwrap();
        let result = session_list(&db, None, None, Some(&SessionCategory::Scope), 20).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v.as_array().unwrap().len(), 1);
        assert_eq!(v[0]["category"], "scope");
    }

    #[test]
    fn session_list_respects_limit() {
        let db = test_db();
        for i in 0..5 {
            session_event(
                &db,
                "2026-03-29".to_string(),
                SessionEventType::Clarification,
                format!("Event {i}"),
                None,
            )
            .unwrap();
        }
        let result = session_list(&db, None, None, None, 3).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v.as_array().unwrap().len(), 3);
    }

    #[test]
    fn session_event_rejects_invalid_date_format() {
        let db = test_db();
        let result = session_event(
            &db,
            "29-03-2026".to_string(),
            SessionEventType::Clarification,
            "Bad date".to_string(),
            None,
        );
        assert!(result.is_err());
        let json = result.unwrap_err().to_json();
        assert!(json.contains("CONSTRAINT_VIOLATION"));
        assert!(json.contains("YYYY-MM-DD"));
    }

    #[test]
    fn session_event_rejects_non_date_string() {
        let db = test_db();
        let result = session_event(
            &db,
            "today".to_string(),
            SessionEventType::Clarification,
            "Bad date".to_string(),
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn session_list_rejects_invalid_date_filter() {
        let db = test_db();
        let result = session_list(&db, Some("2026/03/29"), None, None, 20);
        assert!(result.is_err());
        let json = result.unwrap_err().to_json();
        assert!(json.contains("CONSTRAINT_VIOLATION"));
    }
}
