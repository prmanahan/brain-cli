use clap::Parser;
use std::process;

use brain_core::activity;
use brain_core::db::Database;
use brain_core::dispatch;
use brain_core::error::BrainError;
use brain_core::project;
use brain_core::scope;
use brain_core::session;
use brain_core::task;
use brain_core::types::*;

#[derive(Parser)]
#[command(name = "brain", about = "CLI for interacting with brain.db")]
struct Cli {
    /// Path to the SQLite database (overrides BRAIN_DB_PATH and default)
    #[arg(long, global = true)]
    db: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Dispatch metrics operations
    Dispatch {
        #[command(subcommand)]
        action: DispatchAction,
    },
    /// Task management operations
    Task {
        #[command(subcommand)]
        action: TaskAction,
    },
    /// Activity log operations
    Activity {
        #[command(subcommand)]
        action: ActivityAction,
    },
    /// Session event operations
    Session {
        #[command(subcommand)]
        action: SessionAction,
    },
    /// Scope violation tracking
    Scope {
        #[command(subcommand)]
        action: ScopeAction,
    },
    /// Project management operations
    Project {
        #[command(subcommand)]
        action: ProjectAction,
    },
}

#[derive(clap::Subcommand)]
enum DispatchAction {
    /// Start a new dispatch and get the dispatch ID
    Start {
        #[arg(long)]
        task_id: TaskId,
        #[arg(long)]
        agent: Agent,
        #[arg(long)]
        provider: Provider,
        #[arg(long)]
        model: Model,
        /// Dispatch tier: t1 (Opus), t2 (Sonnet), t3 (Haiku)
        #[arg(long)]
        tier: Option<DispatchTier>,
        /// Task pattern classification label (e.g. implementation, code_review)
        #[arg(long)]
        task_pattern: Option<String>,
        /// Justification for the tier selection
        #[arg(long)]
        tier_justification: Option<String>,
        /// Estimated dispatch context size in KB
        #[arg(long)]
        dispatch_context_kb: Option<f64>,
    },
    /// Complete a dispatch with final metrics
    Complete {
        #[arg(long)]
        id: DispatchId,
        #[arg(long)]
        status: DispatchStatus,
        #[arg(long)]
        tokens_input: Option<u64>,
        #[arg(long)]
        tokens_output: Option<u64>,
        #[arg(long)]
        tokens_total: Option<u64>,
        #[arg(long)]
        duration_ms: Option<u64>,
        #[arg(long)]
        tool_uses: Option<u32>,
        #[arg(long)]
        cost: Option<f64>,
    },
    /// Log a dispatch event
    Event {
        #[arg(long)]
        id: DispatchId,
        #[arg(long = "type")]
        event_type: EventType,
        #[arg(long)]
        severity: Severity,
        #[arg(long)]
        description: String,
    },
    /// Get a dispatch record by ID
    Get {
        /// Dispatch ID (positional or --id)
        #[arg(value_name = "ID")]
        id: DispatchId,
    },
    /// List dispatch records with optional filters
    List {
        #[arg(long)]
        task_id: Option<TaskId>,
        #[arg(long)]
        agent: Option<Agent>,
        #[arg(long)]
        status: Option<DispatchStatus>,
        /// Filter by dispatch tier: t1, t2, t3
        #[arg(long)]
        tier: Option<DispatchTier>,
    },
}

/// Output format for list commands
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, clap::ValueEnum)]
enum OutputFormat {
    #[default]
    Json,
    Table,
}

#[derive(clap::Subcommand)]
enum TaskAction {
    /// Create a new task
    #[command(alias = "add")]
    Create {
        #[arg(long)]
        title: String,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        assigned_to: Option<Agent>,
        #[arg(long)]
        priority: Option<Priority>,
        #[arg(long)]
        project_id: Option<ProjectId>,
        #[arg(long)]
        parent_id: Option<TaskId>,
        #[arg(long, default_value = "Puck")]
        created_by: String,
    },
    /// Update an existing task
    Update {
        /// Task ID (positional or --id)
        #[arg(value_name = "ID")]
        id: TaskId,
        #[arg(long)]
        status: Option<TaskStatus>,
        #[arg(long)]
        assigned_to: Option<Agent>,
        #[arg(long)]
        priority: Option<Priority>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        project_id: Option<ProjectId>,
    },
    /// Get a task by ID
    Get {
        /// Task ID (positional or --id)
        #[arg(value_name = "ID")]
        id: TaskId,
    },
    /// List tasks with optional filters
    List {
        #[arg(long)]
        status: Option<TaskStatus>,
        #[arg(long)]
        assigned_to: Option<Agent>,
        #[arg(long)]
        project_id: Option<ProjectId>,
        /// Filter by direct children of this parent task ID
        #[arg(long, conflicts_with = "tree")]
        parent_id: Option<TaskId>,
        /// Display tasks as an indented parent/child tree
        #[arg(long, conflicts_with = "parent_id")]
        tree: bool,
        /// Output format: json (default) or table
        #[arg(long, default_value = "json")]
        format: OutputFormat,
    },
}

#[derive(clap::Subcommand)]
enum ActivityAction {
    /// Log an activity entry
    Log {
        #[arg(long)]
        actor: String,
        #[arg(long)]
        action: String,
        #[arg(long)]
        task_id: Option<TaskId>,
        #[arg(long)]
        target_type: Option<String>,
        #[arg(long)]
        target_id: Option<i64>,
        #[arg(long)]
        summary: Option<String>,
    },
    /// List activity log entries with optional filters
    List {
        #[arg(long)]
        actor: Option<String>,
        #[arg(long)]
        task_id: Option<TaskId>,
        #[arg(long)]
        target_type: Option<String>,
        #[arg(long, default_value = "20")]
        limit: u32,
    },
}

#[derive(clap::Subcommand)]
enum SessionAction {
    /// Log a session event
    Event {
        #[arg(long)]
        date: String,
        #[arg(long = "type")]
        event_type: SessionEventType,
        #[arg(long)]
        description: String,
        #[arg(long)]
        category: Option<SessionCategory>,
    },
    /// List session events with optional filters
    List {
        #[arg(long)]
        date: Option<String>,
        #[arg(long = "type")]
        event_type: Option<SessionEventType>,
        #[arg(long)]
        category: Option<SessionCategory>,
        #[arg(long, default_value = "20")]
        limit: u32,
    },
}

#[derive(clap::Subcommand)]
enum ScopeAction {
    /// Log a scope violation
    Log {
        #[arg(long)]
        agent: Agent,
        #[arg(long)]
        task_id: Option<TaskId>,
        #[arg(long)]
        dispatch_id: Option<DispatchId>,
        #[arg(long = "type")]
        violation_type: ViolationType,
        #[arg(long)]
        command: String,
        #[arg(long)]
        context: Option<String>,
        #[arg(long)]
        reason: String,
        #[arg(long)]
        resolution: Option<String>,
    },
    /// List scope violations
    List {
        #[arg(long)]
        agent: Option<Agent>,
        #[arg(long = "type")]
        violation_type: Option<ViolationType>,
        #[arg(long, default_value = "20")]
        limit: u32,
    },
}

#[derive(clap::Subcommand)]
enum ProjectAction {
    /// Create a new project
    #[command(alias = "add")]
    Create {
        #[arg(long)]
        name: String,
        #[arg(long)]
        description: Option<String>,
        /// Filesystem path to the project root
        #[arg(long)]
        path: Option<String>,
    },
    /// List projects with optional filters
    List {
        #[arg(long)]
        status: Option<ProjectStatus>,
    },
    /// Update an existing project
    Update {
        /// Project ID (positional)
        #[arg(value_name = "ID")]
        id: ProjectId,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        status: Option<ProjectStatus>,
        /// Filesystem path to the project root
        #[arg(long)]
        path: Option<String>,
    },
}

/// Render a JSON task-list array as a human-readable table.
///
/// Columns are sized to their widest value so output stays aligned regardless
/// of task titles or status strings. A PARENT column is included (after ID,
/// before TITLE) when any returned task has a non-null parent_id.
fn render_task_table(json: &str) -> Result<String, BrainError> {
    let tasks: Vec<serde_json::Value> =
        serde_json::from_str(json).map_err(|e| BrainError::DatabaseError {
            message: e.to_string(),
        })?;

    if tasks.is_empty() {
        return Ok("No tasks found.".to_string());
    }

    // Helpers that extract a displayable string from a value, treating null as "-"
    let get = |v: &serde_json::Value, key: &str| -> String {
        match v.get(key) {
            Some(serde_json::Value::String(s)) => s.clone(),
            Some(serde_json::Value::Number(n)) => n.to_string(),
            Some(serde_json::Value::Null) | None => "-".to_string(),
            Some(other) => other.to_string(),
        }
    };

    // Determine whether any task has a non-null parent_id
    let show_parent = tasks
        .iter()
        .any(|t| t.get("parent_id").is_some_and(|v| !v.is_null()));

    // Column widths — seed with header lengths
    let mut w_id = 2usize; // "ID"
    let mut w_parent = 6usize; // "PARENT"
    let mut w_title = 5usize; // "TITLE"
    let mut w_status = 6usize; // "STATUS"
    let mut w_priority = 8usize; // "PRIORITY"
    let mut w_assigned = 8usize; // "ASSIGNED"

    for t in &tasks {
        w_id = w_id.max(get(t, "id").len());
        if show_parent {
            w_parent = w_parent.max(get(t, "parent_id").len());
        }
        w_title = w_title.max(get(t, "title").len());
        w_status = w_status.max(get(t, "status").len());
        w_priority = w_priority.max(get(t, "priority").len());
        w_assigned = w_assigned.max(get(t, "assigned_to").len());
    }

    // Build separator and rows, conditionally including PARENT column
    let sep = if show_parent {
        format!(
            "+-{}-+-{}-+-{}-+-{}-+-{}-+-{}-+",
            "-".repeat(w_id),
            "-".repeat(w_parent),
            "-".repeat(w_title),
            "-".repeat(w_status),
            "-".repeat(w_priority),
            "-".repeat(w_assigned),
        )
    } else {
        format!(
            "+-{}-+-{}-+-{}-+-{}-+-{}-+",
            "-".repeat(w_id),
            "-".repeat(w_title),
            "-".repeat(w_status),
            "-".repeat(w_priority),
            "-".repeat(w_assigned),
        )
    };

    let mut out = String::new();
    out.push_str(&sep);
    out.push('\n');

    if show_parent {
        out.push_str(&format!(
            "| {:<w_id$} | {:<w_parent$} | {:<w_title$} | {:<w_status$} | {:<w_priority$} | {:<w_assigned$} |",
            "ID", "PARENT", "TITLE", "STATUS", "PRIORITY", "ASSIGNED",
            w_id = w_id,
            w_parent = w_parent,
            w_title = w_title,
            w_status = w_status,
            w_priority = w_priority,
            w_assigned = w_assigned,
        ));
    } else {
        out.push_str(&format!(
            "| {:<w_id$} | {:<w_title$} | {:<w_status$} | {:<w_priority$} | {:<w_assigned$} |",
            "ID",
            "TITLE",
            "STATUS",
            "PRIORITY",
            "ASSIGNED",
            w_id = w_id,
            w_title = w_title,
            w_status = w_status,
            w_priority = w_priority,
            w_assigned = w_assigned,
        ));
    }
    out.push('\n');
    out.push_str(&sep);
    out.push('\n');

    for t in &tasks {
        if show_parent {
            out.push_str(&format!(
                "| {:<w_id$} | {:<w_parent$} | {:<w_title$} | {:<w_status$} | {:<w_priority$} | {:<w_assigned$} |",
                get(t, "id"),
                get(t, "parent_id"),
                get(t, "title"),
                get(t, "status"),
                get(t, "priority"),
                get(t, "assigned_to"),
                w_id = w_id,
                w_parent = w_parent,
                w_title = w_title,
                w_status = w_status,
                w_priority = w_priority,
                w_assigned = w_assigned,
            ));
        } else {
            out.push_str(&format!(
                "| {:<w_id$} | {:<w_title$} | {:<w_status$} | {:<w_priority$} | {:<w_assigned$} |",
                get(t, "id"),
                get(t, "title"),
                get(t, "status"),
                get(t, "priority"),
                get(t, "assigned_to"),
                w_id = w_id,
                w_title = w_title,
                w_status = w_status,
                w_priority = w_priority,
                w_assigned = w_assigned,
            ));
        }
        out.push('\n');
    }
    out.push_str(&sep);

    Ok(out)
}

/// Render a JSON tree array (from task_list_tree) as an indented table.
///
/// Top-level tasks are shown at root. Children are shown with `└─` prefix
/// and 2-space indentation per level. The PARENT column is omitted in tree
/// mode because hierarchy is conveyed visually.
fn render_task_tree_table(json: &str) -> Result<String, BrainError> {
    // Flatten tree into (indent_level, task_value) pairs for display
    let roots: Vec<serde_json::Value> =
        serde_json::from_str(json).map_err(|e| BrainError::DatabaseError {
            message: e.to_string(),
        })?;

    if roots.is_empty() {
        return Ok("No tasks found.".to_string());
    }

    let get = |v: &serde_json::Value, key: &str| -> String {
        match v.get(key) {
            Some(serde_json::Value::String(s)) => s.clone(),
            Some(serde_json::Value::Number(n)) => n.to_string(),
            Some(serde_json::Value::Null) | None => "-".to_string(),
            Some(other) => other.to_string(),
        }
    };

    // Build flat rows: (display_title_with_indent, id, status, priority, assigned)
    struct Row {
        id: String,
        title: String,
        status: String,
        priority: String,
        assigned: String,
    }

    let mut rows: Vec<Row> = Vec::new();

    for root in &roots {
        rows.push(Row {
            id: get(root, "id"),
            title: get(root, "title"),
            status: get(root, "status"),
            priority: get(root, "priority"),
            assigned: get(root, "assigned_to"),
        });
        if let Some(children) = root["children"].as_array() {
            for child in children {
                rows.push(Row {
                    id: get(child, "id"),
                    title: format!("  └─ {}", get(child, "title")),
                    status: get(child, "status"),
                    priority: get(child, "priority"),
                    assigned: get(child, "assigned_to"),
                });
            }
        }
    }

    let mut w_id = 2usize;
    let mut w_title = 5usize;
    let mut w_status = 6usize;
    let mut w_priority = 8usize;
    let mut w_assigned = 8usize;

    for r in &rows {
        w_id = w_id.max(r.id.len());
        w_title = w_title.max(r.title.len());
        w_status = w_status.max(r.status.len());
        w_priority = w_priority.max(r.priority.len());
        w_assigned = w_assigned.max(r.assigned.len());
    }

    let sep = format!(
        "+-{}-+-{}-+-{}-+-{}-+-{}-+",
        "-".repeat(w_id),
        "-".repeat(w_title),
        "-".repeat(w_status),
        "-".repeat(w_priority),
        "-".repeat(w_assigned),
    );

    let mut out = String::new();
    out.push_str(&sep);
    out.push('\n');
    out.push_str(&format!(
        "| {:<w_id$} | {:<w_title$} | {:<w_status$} | {:<w_priority$} | {:<w_assigned$} |",
        "ID",
        "TITLE",
        "STATUS",
        "PRIORITY",
        "ASSIGNED",
        w_id = w_id,
        w_title = w_title,
        w_status = w_status,
        w_priority = w_priority,
        w_assigned = w_assigned,
    ));
    out.push('\n');
    out.push_str(&sep);
    out.push('\n');

    for r in &rows {
        out.push_str(&format!(
            "| {:<w_id$} | {:<w_title$} | {:<w_status$} | {:<w_priority$} | {:<w_assigned$} |",
            r.id,
            r.title,
            r.status,
            r.priority,
            r.assigned,
            w_id = w_id,
            w_title = w_title,
            w_status = w_status,
            w_priority = w_priority,
            w_assigned = w_assigned,
        ));
        out.push('\n');
    }
    out.push_str(&sep);

    Ok(out)
}

fn run(cli: Cli) -> Result<String, BrainError> {
    let db_path = Database::resolve_path(cli.db.as_deref());
    let db = Database::open(&db_path)?;

    match cli.command {
        Commands::Dispatch { action } => match action {
            DispatchAction::Start {
                task_id,
                agent,
                provider,
                model,
                tier,
                task_pattern,
                tier_justification,
                dispatch_context_kb,
            } => dispatch::dispatch_start(
                &db,
                task_id,
                agent,
                provider,
                model,
                tier,
                task_pattern,
                tier_justification,
                dispatch_context_kb,
            ),
            DispatchAction::Complete {
                id,
                status,
                tokens_input,
                tokens_output,
                tokens_total,
                duration_ms,
                tool_uses,
                cost,
            } => dispatch::dispatch_complete(
                &db,
                id,
                status,
                tokens_input,
                tokens_output,
                tokens_total,
                duration_ms,
                tool_uses,
                cost,
            ),
            DispatchAction::Event {
                id,
                event_type,
                severity,
                description,
            } => dispatch::dispatch_event(&db, id, event_type, severity, description),
            DispatchAction::Get { id } => dispatch::dispatch_get(&db, id),
            DispatchAction::List {
                task_id,
                agent,
                status,
                tier,
            } => dispatch::dispatch_list(
                &db,
                task_id.as_ref(),
                agent.as_ref(),
                status.as_ref(),
                tier.as_ref(),
            ),
        },
        Commands::Task { action } => match action {
            TaskAction::Create {
                title,
                description,
                assigned_to,
                priority,
                project_id,
                parent_id,
                created_by,
            } => task::task_create(
                &db,
                title,
                description,
                assigned_to,
                priority,
                project_id,
                parent_id,
                created_by,
            ),
            TaskAction::Update {
                id,
                status,
                assigned_to,
                priority,
                description,
                project_id,
            } => task::task_update(
                &db,
                id,
                status,
                assigned_to,
                priority,
                description,
                project_id,
            ),
            TaskAction::Get { id } => task::task_get(&db, id),
            TaskAction::List {
                status,
                assigned_to,
                project_id,
                parent_id,
                tree,
                format,
            } => {
                if tree {
                    let json = task::task_list_tree(
                        &db,
                        status.as_ref(),
                        assigned_to.as_ref(),
                        project_id.as_ref(),
                    )?;
                    match format {
                        OutputFormat::Json => Ok(json),
                        OutputFormat::Table => render_task_tree_table(&json),
                    }
                } else {
                    let json = task::task_list(
                        &db,
                        status.as_ref(),
                        assigned_to.as_ref(),
                        project_id.as_ref(),
                        parent_id.as_ref(),
                    )?;
                    match format {
                        OutputFormat::Json => Ok(json),
                        OutputFormat::Table => render_task_table(&json),
                    }
                }
            }
        },
        Commands::Activity { action } => match action {
            ActivityAction::Log {
                actor,
                action,
                task_id,
                target_type,
                target_id,
                summary,
            } => {
                activity::activity_log(&db, actor, action, task_id, target_type, target_id, summary)
            }
            ActivityAction::List {
                actor,
                task_id,
                target_type,
                limit,
            } => activity::activity_list(
                &db,
                actor.as_deref(),
                task_id.as_ref(),
                target_type.as_deref(),
                limit,
            ),
        },
        Commands::Session { action } => match action {
            SessionAction::Event {
                date,
                event_type,
                description,
                category,
            } => session::session_event(&db, date, event_type, description, category),
            SessionAction::List {
                date,
                event_type,
                category,
                limit,
            } => session::session_list(
                &db,
                date.as_deref(),
                event_type.as_ref(),
                category.as_ref(),
                limit,
            ),
        },
        Commands::Scope { action } => match action {
            ScopeAction::Log {
                agent,
                task_id,
                dispatch_id,
                violation_type,
                command,
                context,
                reason,
                resolution,
            } => scope::scope_log(
                &db,
                agent,
                task_id,
                dispatch_id,
                violation_type,
                command,
                context,
                reason,
                resolution,
            ),
            ScopeAction::List {
                agent,
                violation_type,
                limit,
            } => scope::scope_list(&db, agent.as_ref(), violation_type.as_ref(), limit),
        },
        Commands::Project { action } => match action {
            ProjectAction::Create {
                name,
                description,
                path,
            } => project::project_create(&db, name, description, path),
            ProjectAction::List { status } => project::project_list(&db, status.as_ref()),
            ProjectAction::Update {
                id,
                name,
                description,
                status,
                path,
            } => project::project_update(&db, id, name, description, status, path),
        },
    }
}

fn main() {
    let cli = Cli::parse();
    match run(cli) {
        Ok(json) => {
            println!("{json}");
        }
        Err(e) => {
            println!("{}", e.to_json());
            process::exit(1);
        }
    }
}
