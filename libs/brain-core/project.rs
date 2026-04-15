use crate::db::Database;
use crate::error::BrainError;
use crate::types::*;
use serde_json::json;

pub fn project_create(
    db: &Database,
    name: String,
    description: Option<String>,
    path: Option<String>,
) -> Result<String, BrainError> {
    let id = db.conn.query_row(
        "INSERT INTO projects (name, description, path) VALUES (?1, ?2, ?3) RETURNING id",
        rusqlite::params![name, description, path],
        |row| row.get::<_, i64>(0),
    )?;

    Ok(serde_json::to_string(&json!({ "project_id": id })).unwrap())
}

pub fn project_list(db: &Database, status: Option<&ProjectStatus>) -> Result<String, BrainError> {
    let mut sql = String::from(
        "SELECT id, name, description, path, status, created_at, updated_at
         FROM projects WHERE 1=1",
    );
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(s) = status {
        params.push(Box::new(s.to_string()));
        sql.push_str(&format!(" AND status = ?{}", params.len()));
    }

    sql.push_str(" ORDER BY id DESC");

    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let mut stmt = db.conn.prepare(&sql)?;
    let rows = stmt.query_map(param_refs.as_slice(), |row| {
        Ok(json!({
            "id": row.get::<_, i64>(0)?,
            "name": row.get::<_, String>(1)?,
            "description": row.get::<_, Option<String>>(2)?,
            "path": row.get::<_, Option<String>>(3)?,
            "status": row.get::<_, String>(4)?,
            "created_at": row.get::<_, String>(5)?,
            "updated_at": row.get::<_, String>(6)?,
        }))
    })?;

    let results: Vec<serde_json::Value> = rows.collect::<Result<Vec<_>, _>>()?;
    Ok(serde_json::to_string(&results).unwrap())
}

pub fn project_update(
    db: &Database,
    id: ProjectId,
    name: Option<String>,
    description: Option<String>,
    status: Option<ProjectStatus>,
    path: Option<String>,
) -> Result<String, BrainError> {
    let mut sets = vec!["updated_at = datetime('now')".to_string()];
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(n) = &name {
        params.push(Box::new(n.clone()));
        sets.push(format!("name = ?{}", params.len()));
    }
    if let Some(d) = &description {
        params.push(Box::new(d.clone()));
        sets.push(format!("description = ?{}", params.len()));
    }
    if let Some(s) = &status {
        params.push(Box::new(s.to_string()));
        sets.push(format!("status = ?{}", params.len()));
    }
    if let Some(p) = &path {
        params.push(Box::new(p.clone()));
        sets.push(format!("path = ?{}", params.len()));
    }

    params.push(Box::new(id.0));
    let sql = format!(
        "UPDATE projects SET {} WHERE id = ?{}",
        sets.join(", "),
        params.len()
    );

    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let rows = db.conn.execute(&sql, param_refs.as_slice())?;

    if rows == 0 {
        return Err(BrainError::NotFound {
            entity: "project",
            id: id.0,
        });
    }

    let final_status = if let Some(s) = status {
        s.to_string()
    } else {
        db.conn
            .query_row("SELECT status FROM projects WHERE id = ?1", [id.0], |row| {
                row.get(0)
            })?
    };

    Ok(serde_json::to_string(&json!({
        "project_id": id.0,
        "status": final_status
    }))
    .unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn test_db() -> Database {
        Database::in_memory().unwrap()
    }

    #[test]
    fn project_create_returns_id() {
        let db = test_db();
        let result = project_create(&db, "project-a".to_string(), None, None).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["project_id"], 1);
    }

    #[test]
    fn project_create_with_description() {
        let db = test_db();
        let result = project_create(
            &db,
            "project-b".to_string(),
            Some("Brain CLI project".to_string()),
            None,
        )
        .unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(v["project_id"].as_i64().unwrap() > 0);
    }

    #[test]
    fn project_create_duplicate_name_returns_error() {
        let db = test_db();
        project_create(&db, "project-a".to_string(), None, None).unwrap();
        let result = project_create(&db, "project-a".to_string(), None, None);
        assert!(result.is_err());
    }

    #[test]
    fn project_create_with_path() {
        let db = test_db();
        let result = project_create(
            &db,
            "project-a".to_string(),
            None,
            Some("/tmp/brain-cli-test/project-a".to_string()),
        )
        .unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(v["project_id"].as_i64().unwrap() > 0);

        let list = project_list(&db, None).unwrap();
        let lv: serde_json::Value = serde_json::from_str(&list).unwrap();
        assert_eq!(lv[0]["path"], "/tmp/brain-cli-test/project-a");
    }

    #[test]
    fn project_create_without_path_stores_null() {
        let db = test_db();
        project_create(&db, "project-b".to_string(), None, None).unwrap();
        let list = project_list(&db, None).unwrap();
        let lv: serde_json::Value = serde_json::from_str(&list).unwrap();
        assert!(lv[0]["path"].is_null());
    }

    #[test]
    fn project_list_returns_all() {
        let db = test_db();
        project_create(&db, "project-a".to_string(), None, None).unwrap();
        project_create(&db, "project-b".to_string(), None, None).unwrap();
        let result = project_list(&db, None).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v.as_array().unwrap().len(), 2);
    }

    #[test]
    fn project_list_filters_by_status() {
        let db = test_db();
        project_create(&db, "project-a".to_string(), None, None).unwrap();
        project_create(&db, "project-b".to_string(), None, None).unwrap();
        project_update(
            &db,
            ProjectId(1),
            None,
            None,
            Some(ProjectStatus::Archived),
            None,
        )
        .unwrap();
        let result = project_list(&db, Some(&ProjectStatus::Active)).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v.as_array().unwrap().len(), 1);
        assert_eq!(v[0]["name"], "project-b");
    }

    #[test]
    fn project_update_changes_status() {
        let db = test_db();
        project_create(&db, "project-a".to_string(), None, None).unwrap();
        let result = project_update(
            &db,
            ProjectId(1),
            None,
            None,
            Some(ProjectStatus::Paused),
            None,
        )
        .unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["project_id"], 1);
        assert_eq!(v["status"], "paused");
    }

    #[test]
    fn project_update_changes_name() {
        let db = test_db();
        project_create(&db, "project-a".to_string(), None, None).unwrap();
        project_update(
            &db,
            ProjectId(1),
            Some("project-a-renamed".to_string()),
            None,
            None,
            None,
        )
        .unwrap();
        let result = project_list(&db, None).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v[0]["name"], "project-a-renamed");
    }

    #[test]
    fn project_update_sets_path() {
        let db = test_db();
        project_create(&db, "project-a".to_string(), None, None).unwrap();
        project_update(
            &db,
            ProjectId(1),
            None,
            None,
            None,
            Some("/tmp/brain-cli-test/project-a".to_string()),
        )
        .unwrap();
        let result = project_list(&db, None).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v[0]["path"], "/tmp/brain-cli-test/project-a");
    }

    #[test]
    fn project_update_not_found() {
        let db = test_db();
        let result = project_update(
            &db,
            ProjectId(999),
            None,
            None,
            Some(ProjectStatus::Archived),
            None,
        );
        assert!(result.is_err());
        let json = result.unwrap_err().to_json();
        assert!(json.contains("NOT_FOUND"));
    }
}
