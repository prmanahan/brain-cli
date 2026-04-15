use serde_json::json;

#[derive(Debug)]
pub enum BrainError {
    NotFound { entity: &'static str, id: i64 },
    DatabaseError { message: String },
    ConstraintViolation { description: String },
    ConnectionFailed { path: String, message: String },
}

impl BrainError {
    pub fn to_json(&self) -> String {
        let value = match self {
            BrainError::NotFound { entity, id } => json!({
                "error": format!("{entity} not found"),
                "code": "NOT_FOUND",
                "detail": format!("no {entity} with id {id}")
            }),
            BrainError::DatabaseError { message } => json!({
                "error": "database error",
                "code": "DATABASE_ERROR",
                "detail": message
            }),
            BrainError::ConstraintViolation { description } => json!({
                "error": "constraint violation",
                "code": "CONSTRAINT_VIOLATION",
                "detail": description
            }),
            BrainError::ConnectionFailed { path, message } => json!({
                "error": "connection failed",
                "code": "CONNECTION_FAILED",
                "detail": format!("{path}: {message}")
            }),
        };
        serde_json::to_string(&value).unwrap()
    }
}

impl From<rusqlite::Error> for BrainError {
    fn from(err: rusqlite::Error) -> Self {
        BrainError::DatabaseError {
            message: err.to_string(),
        }
    }
}

impl std::fmt::Display for BrainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_json())
    }
}

impl std::error::Error for BrainError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_serializes_to_json() {
        let err = BrainError::NotFound {
            entity: "task",
            id: 99,
        };
        let json = err.to_json();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["error"], "task not found");
        assert_eq!(v["code"], "NOT_FOUND");
        assert_eq!(v["detail"], "no task with id 99");
    }

    #[test]
    fn constraint_violation_serializes_to_json() {
        let err = BrainError::ConstraintViolation {
            description: "foreign key violation on project_id".to_string(),
        };
        let json = err.to_json();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["error"], "constraint violation");
        assert_eq!(v["code"], "CONSTRAINT_VIOLATION");
        assert_eq!(v["detail"], "foreign key violation on project_id");
    }

    #[test]
    fn connection_failed_serializes_to_json() {
        let err = BrainError::ConnectionFailed {
            path: "/bad/path.db".to_string(),
            message: "unable to open database file".to_string(),
        };
        let json = err.to_json();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["error"], "connection failed");
        assert_eq!(v["code"], "CONNECTION_FAILED");
        assert!(v["detail"].as_str().unwrap().contains("/bad/path.db"));
    }

    #[test]
    fn database_error_serializes_to_json() {
        let err = BrainError::DatabaseError {
            message: "disk I/O error".to_string(),
        };
        let json = err.to_json();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["error"], "database error");
        assert_eq!(v["code"], "DATABASE_ERROR");
        assert_eq!(v["detail"], "disk I/O error");
    }
}
