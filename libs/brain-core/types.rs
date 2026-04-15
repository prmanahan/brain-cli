use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

// --- Newtype IDs ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskId(pub i64);

impl FromStr for TaskId {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let n: i64 = s
            .parse()
            .map_err(|_| format!("invalid task id '{s}', expected positive integer"))?;
        if n <= 0 {
            return Err(format!("invalid task id '{n}', must be positive"));
        }
        Ok(TaskId(n))
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatchId(pub i64);

impl FromStr for DispatchId {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let n: i64 = s
            .parse()
            .map_err(|_| format!("invalid dispatch id '{s}', expected positive integer"))?;
        if n <= 0 {
            return Err(format!("invalid dispatch id '{n}', must be positive"));
        }
        Ok(DispatchId(n))
    }
}

impl fmt::Display for DispatchId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectId(pub i64);

impl FromStr for ProjectId {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let n: i64 = s
            .parse()
            .map_err(|_| format!("invalid project id '{s}', expected positive integer"))?;
        if n <= 0 {
            return Err(format!("invalid project id '{n}', must be positive"));
        }
        Ok(ProjectId(n))
    }
}

impl fmt::Display for ProjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// --- Newtype Strings ---

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Agent(pub String);

impl FromStr for Agent {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err("agent name cannot be empty".to_string());
        }
        Ok(Agent(trimmed.to_string()))
    }
}

impl fmt::Display for Agent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model(pub String);

impl FromStr for Model {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err("model name cannot be empty".to_string());
        }
        Ok(Model(trimmed.to_string()))
    }
}

impl fmt::Display for Model {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// --- Enums ---

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Provider {
    Anthropic,
    Google,
    OpenAi,
    Other(String),
}

impl FromStr for Provider {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err("provider cannot be empty".to_string());
        }
        match trimmed.to_lowercase().as_str() {
            "anthropic" => Ok(Provider::Anthropic),
            "google" => Ok(Provider::Google),
            "openai" => Ok(Provider::OpenAi),
            _ => Ok(Provider::Other(trimmed.to_string())),
        }
    }
}

impl fmt::Display for Provider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Provider::Anthropic => write!(f, "anthropic"),
            Provider::Google => write!(f, "google"),
            Provider::OpenAi => write!(f, "openai"),
            Provider::Other(s) => write!(f, "{s}"),
        }
    }
}

impl Serialize for Provider {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Provider {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Open,
    InProgress,
    Done,
    Blocked,
}

impl FromStr for TaskStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "open" => Ok(TaskStatus::Open),
            "in_progress" => Ok(TaskStatus::InProgress),
            "done" => Ok(TaskStatus::Done),
            "blocked" => Ok(TaskStatus::Blocked),
            _ => Err(format!(
                "invalid status '{s}', expected one of: open, in_progress, done, blocked"
            )),
        }
    }
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskStatus::Open => write!(f, "open"),
            TaskStatus::InProgress => write!(f, "in_progress"),
            TaskStatus::Done => write!(f, "done"),
            TaskStatus::Blocked => write!(f, "blocked"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchStatus {
    InProgress,
    Completed,
    Failed,
}

impl FromStr for DispatchStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "in_progress" => Ok(DispatchStatus::InProgress),
            "completed" => Ok(DispatchStatus::Completed),
            "failed" => Ok(DispatchStatus::Failed),
            _ => Err(format!(
                "invalid dispatch status '{s}', expected one of: in_progress, completed, failed"
            )),
        }
    }
}

impl fmt::Display for DispatchStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DispatchStatus::InProgress => write!(f, "in_progress"),
            DispatchStatus::Completed => write!(f, "completed"),
            DispatchStatus::Failed => write!(f, "failed"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    Routing,
    Decision,
    Error,
    SelfCorrection,
    ApproachChange,
    ReviewFailure,
}

impl FromStr for EventType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "routing" => Ok(EventType::Routing),
            "decision" => Ok(EventType::Decision),
            "error" => Ok(EventType::Error),
            "self_correction" => Ok(EventType::SelfCorrection),
            "approach_change" => Ok(EventType::ApproachChange),
            "review_failure" => Ok(EventType::ReviewFailure),
            _ => Err(format!(
                "invalid event type '{s}', expected one of: routing, decision, error, self_correction, approach_change, review_failure"
            )),
        }
    }
}

impl fmt::Display for EventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventType::Routing => write!(f, "routing"),
            EventType::Decision => write!(f, "decision"),
            EventType::Error => write!(f, "error"),
            EventType::SelfCorrection => write!(f, "self_correction"),
            EventType::ApproachChange => write!(f, "approach_change"),
            EventType::ReviewFailure => write!(f, "review_failure"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

impl FromStr for Severity {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "info" => Ok(Severity::Info),
            "warning" => Ok(Severity::Warning),
            "error" => Ok(Severity::Error),
            _ => Err(format!(
                "invalid severity '{s}', expected one of: info, warning, error"
            )),
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Info => write!(f, "info"),
            Severity::Warning => write!(f, "warning"),
            Severity::Error => write!(f, "error"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

impl FromStr for Priority {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "low" => Ok(Priority::Low),
            "medium" => Ok(Priority::Medium),
            "high" => Ok(Priority::High),
            "critical" => Ok(Priority::Critical),
            _ => Err(format!(
                "invalid priority '{s}', expected one of: low, medium, high, critical"
            )),
        }
    }
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Priority::Low => write!(f, "low"),
            Priority::Medium => write!(f, "medium"),
            Priority::High => write!(f, "high"),
            Priority::Critical => write!(f, "critical"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionEventType {
    Clarification,
    Revision,
    CourseCorrection,
}

impl FromStr for SessionEventType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "clarification" => Ok(SessionEventType::Clarification),
            "revision" => Ok(SessionEventType::Revision),
            "course_correction" => Ok(SessionEventType::CourseCorrection),
            _ => Err(format!(
                "invalid session event type '{s}', expected one of: clarification, revision, course_correction"
            )),
        }
    }
}

impl fmt::Display for SessionEventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionEventType::Clarification => write!(f, "clarification"),
            SessionEventType::Revision => write!(f, "revision"),
            SessionEventType::CourseCorrection => write!(f, "course_correction"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionCategory {
    Scope,
    AcceptanceCriteria,
    Context,
    Routing,
    Priority,
}

impl FromStr for SessionCategory {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "scope" => Ok(SessionCategory::Scope),
            "acceptance_criteria" => Ok(SessionCategory::AcceptanceCriteria),
            "context" => Ok(SessionCategory::Context),
            "routing" => Ok(SessionCategory::Routing),
            "priority" => Ok(SessionCategory::Priority),
            _ => Err(format!(
                "invalid session category '{s}', expected one of: scope, acceptance_criteria, context, routing, priority"
            )),
        }
    }
}

impl fmt::Display for SessionCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionCategory::Scope => write!(f, "scope"),
            SessionCategory::AcceptanceCriteria => write!(f, "acceptance_criteria"),
            SessionCategory::Context => write!(f, "context"),
            SessionCategory::Routing => write!(f, "routing"),
            SessionCategory::Priority => write!(f, "priority"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViolationType {
    Scope,
    PermissionDenied,
    CdRequired,
}

impl FromStr for ViolationType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "scope" => Ok(ViolationType::Scope),
            "permission_denied" => Ok(ViolationType::PermissionDenied),
            "cd_required" => Ok(ViolationType::CdRequired),
            _ => Err(format!(
                "invalid violation type '{s}', expected one of: scope, permission_denied, cd_required"
            )),
        }
    }
}

impl fmt::Display for ViolationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ViolationType::Scope => write!(f, "scope"),
            ViolationType::PermissionDenied => write!(f, "permission_denied"),
            ViolationType::CdRequired => write!(f, "cd_required"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchTier {
    T1,
    T2,
    T3,
}

impl FromStr for DispatchTier {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "t1" => Ok(DispatchTier::T1),
            "t2" => Ok(DispatchTier::T2),
            "t3" => Ok(DispatchTier::T3),
            _ => Err(format!(
                "invalid dispatch tier '{s}', expected one of: t1, t2, t3"
            )),
        }
    }
}

impl fmt::Display for DispatchTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DispatchTier::T1 => write!(f, "t1"),
            DispatchTier::T2 => write!(f, "t2"),
            DispatchTier::T3 => write!(f, "t3"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectStatus {
    Active,
    Paused,
    Completed,
    Archived,
}

impl FromStr for ProjectStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(ProjectStatus::Active),
            "paused" => Ok(ProjectStatus::Paused),
            "completed" => Ok(ProjectStatus::Completed),
            "archived" => Ok(ProjectStatus::Archived),
            _ => Err(format!(
                "invalid project status '{s}', expected one of: active, paused, completed, archived"
            )),
        }
    }
}

impl fmt::Display for ProjectStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProjectStatus::Active => write!(f, "active"),
            ProjectStatus::Paused => write!(f, "paused"),
            ProjectStatus::Completed => write!(f, "completed"),
            ProjectStatus::Archived => write!(f, "archived"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_id_parses_positive_integer() {
        assert_eq!("42".parse::<TaskId>().unwrap(), TaskId(42));
    }

    #[test]
    fn task_id_rejects_zero() {
        assert!("0".parse::<TaskId>().is_err());
    }

    #[test]
    fn task_id_rejects_negative() {
        assert!("-1".parse::<TaskId>().is_err());
    }

    #[test]
    fn task_id_rejects_non_numeric() {
        assert!("abc".parse::<TaskId>().is_err());
    }

    #[test]
    fn dispatch_id_parses_positive_integer() {
        assert_eq!("7".parse::<DispatchId>().unwrap(), DispatchId(7));
    }

    #[test]
    fn dispatch_id_rejects_zero() {
        assert!("0".parse::<DispatchId>().is_err());
    }

    #[test]
    fn project_id_parses_positive_integer() {
        assert_eq!("1".parse::<ProjectId>().unwrap(), ProjectId(1));
    }

    #[test]
    fn project_id_rejects_zero() {
        assert!("0".parse::<ProjectId>().is_err());
    }

    #[test]
    fn agent_parses_non_empty_string() {
        assert_eq!("Rune".parse::<Agent>().unwrap(), Agent("Rune".to_string()));
    }

    #[test]
    fn agent_rejects_empty_string() {
        assert!("".parse::<Agent>().is_err());
    }

    #[test]
    fn agent_trims_whitespace() {
        assert_eq!(
            "  Rune  ".parse::<Agent>().unwrap(),
            Agent("Rune".to_string())
        );
    }

    #[test]
    fn agent_rejects_whitespace_only() {
        assert!("   ".parse::<Agent>().is_err());
    }

    #[test]
    fn model_parses_non_empty_string() {
        assert_eq!(
            "claude-sonnet-4-6".parse::<Model>().unwrap(),
            Model("claude-sonnet-4-6".to_string())
        );
    }

    #[test]
    fn model_rejects_empty_string() {
        assert!("".parse::<Model>().is_err());
    }

    #[test]
    fn provider_parses_known_variants_case_insensitive() {
        assert_eq!(
            "anthropic".parse::<Provider>().unwrap(),
            Provider::Anthropic
        );
        assert_eq!("GOOGLE".parse::<Provider>().unwrap(), Provider::Google);
        assert_eq!("OpenAI".parse::<Provider>().unwrap(), Provider::OpenAi);
    }

    #[test]
    fn provider_parses_unknown_as_other() {
        assert_eq!(
            "deepseek".parse::<Provider>().unwrap(),
            Provider::Other("deepseek".to_string())
        );
    }

    #[test]
    fn provider_rejects_empty() {
        assert!("".parse::<Provider>().is_err());
    }

    #[test]
    fn task_status_parses_snake_case() {
        assert_eq!("open".parse::<TaskStatus>().unwrap(), TaskStatus::Open);
        assert_eq!(
            "in_progress".parse::<TaskStatus>().unwrap(),
            TaskStatus::InProgress
        );
        assert_eq!("done".parse::<TaskStatus>().unwrap(), TaskStatus::Done);
        assert_eq!(
            "blocked".parse::<TaskStatus>().unwrap(),
            TaskStatus::Blocked
        );
    }

    #[test]
    fn task_status_rejects_unknown() {
        assert!("pending".parse::<TaskStatus>().is_err());
    }

    #[test]
    fn dispatch_status_parses_snake_case() {
        assert_eq!(
            "in_progress".parse::<DispatchStatus>().unwrap(),
            DispatchStatus::InProgress
        );
        assert_eq!(
            "completed".parse::<DispatchStatus>().unwrap(),
            DispatchStatus::Completed
        );
        assert_eq!(
            "failed".parse::<DispatchStatus>().unwrap(),
            DispatchStatus::Failed
        );
    }

    #[test]
    fn event_type_parses_snake_case() {
        assert_eq!("routing".parse::<EventType>().unwrap(), EventType::Routing);
        assert_eq!(
            "decision".parse::<EventType>().unwrap(),
            EventType::Decision
        );
        assert_eq!("error".parse::<EventType>().unwrap(), EventType::Error);
        assert_eq!(
            "self_correction".parse::<EventType>().unwrap(),
            EventType::SelfCorrection
        );
        assert_eq!(
            "approach_change".parse::<EventType>().unwrap(),
            EventType::ApproachChange
        );
        assert_eq!(
            "review_failure".parse::<EventType>().unwrap(),
            EventType::ReviewFailure
        );
    }

    #[test]
    fn severity_parses_lowercase() {
        assert_eq!("info".parse::<Severity>().unwrap(), Severity::Info);
        assert_eq!("warning".parse::<Severity>().unwrap(), Severity::Warning);
        assert_eq!("error".parse::<Severity>().unwrap(), Severity::Error);
    }

    #[test]
    fn priority_parses_lowercase() {
        assert_eq!("low".parse::<Priority>().unwrap(), Priority::Low);
        assert_eq!("medium".parse::<Priority>().unwrap(), Priority::Medium);
        assert_eq!("high".parse::<Priority>().unwrap(), Priority::High);
        assert_eq!("critical".parse::<Priority>().unwrap(), Priority::Critical);
    }

    #[test]
    fn session_event_type_parses_snake_case() {
        assert_eq!(
            "clarification".parse::<SessionEventType>().unwrap(),
            SessionEventType::Clarification
        );
        assert_eq!(
            "revision".parse::<SessionEventType>().unwrap(),
            SessionEventType::Revision
        );
        assert_eq!(
            "course_correction".parse::<SessionEventType>().unwrap(),
            SessionEventType::CourseCorrection
        );
    }

    #[test]
    fn session_event_type_rejects_unknown() {
        assert!("unknown_type".parse::<SessionEventType>().is_err());
    }

    #[test]
    fn session_event_type_displays_snake_case() {
        assert_eq!(
            SessionEventType::CourseCorrection.to_string(),
            "course_correction"
        );
        assert_eq!(SessionEventType::Clarification.to_string(), "clarification");
    }

    #[test]
    fn session_category_parses_snake_case() {
        assert_eq!(
            "scope".parse::<SessionCategory>().unwrap(),
            SessionCategory::Scope
        );
        assert_eq!(
            "acceptance_criteria".parse::<SessionCategory>().unwrap(),
            SessionCategory::AcceptanceCriteria
        );
        assert_eq!(
            "context".parse::<SessionCategory>().unwrap(),
            SessionCategory::Context
        );
        assert_eq!(
            "routing".parse::<SessionCategory>().unwrap(),
            SessionCategory::Routing
        );
        assert_eq!(
            "priority".parse::<SessionCategory>().unwrap(),
            SessionCategory::Priority
        );
    }

    #[test]
    fn session_category_rejects_unknown() {
        assert!("unknown_cat".parse::<SessionCategory>().is_err());
    }

    #[test]
    fn session_category_displays_snake_case() {
        assert_eq!(
            SessionCategory::AcceptanceCriteria.to_string(),
            "acceptance_criteria"
        );
    }

    #[test]
    fn task_status_serializes_to_snake_case() {
        assert_eq!(
            serde_json::to_string(&TaskStatus::InProgress).unwrap(),
            "\"in_progress\""
        );
    }

    #[test]
    fn provider_serializes_other_variant() {
        assert_eq!(
            serde_json::to_string(&Provider::Other("deepseek".to_string())).unwrap(),
            "\"deepseek\""
        );
    }

    #[test]
    fn provider_serializes_known_variant() {
        assert_eq!(
            serde_json::to_string(&Provider::Anthropic).unwrap(),
            "\"anthropic\""
        );
    }

    #[test]
    fn violation_type_parses_snake_case() {
        assert_eq!(
            "scope".parse::<ViolationType>().unwrap(),
            ViolationType::Scope
        );
        assert_eq!(
            "permission_denied".parse::<ViolationType>().unwrap(),
            ViolationType::PermissionDenied
        );
        assert_eq!(
            "cd_required".parse::<ViolationType>().unwrap(),
            ViolationType::CdRequired
        );
    }

    #[test]
    fn violation_type_rejects_unknown() {
        assert!("unknown_type".parse::<ViolationType>().is_err());
    }

    #[test]
    fn violation_type_displays_snake_case() {
        assert_eq!(ViolationType::Scope.to_string(), "scope");
        assert_eq!(
            ViolationType::PermissionDenied.to_string(),
            "permission_denied"
        );
        assert_eq!(ViolationType::CdRequired.to_string(), "cd_required");
    }
}
