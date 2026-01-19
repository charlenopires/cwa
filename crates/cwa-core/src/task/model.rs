//! Task domain models.

use serde::{Deserialize, Serialize};
use cwa_db::queries::tasks::TaskRow;

/// A Kanban task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub project_id: String,
    pub spec_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub priority: String,
    pub assignee: Option<String>,
    pub labels: Vec<String>,
    pub estimated_effort: Option<String>,
    pub actual_effort: Option<String>,
    pub blocked_by: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

impl Task {
    /// Create a Task from a database row.
    pub fn from_row(row: TaskRow) -> Self {
        let labels: Vec<String> = row
            .labels
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        let blocked_by: Vec<String> = row
            .blocked_by
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        Self {
            id: row.id,
            project_id: row.project_id,
            spec_id: row.spec_id,
            title: row.title,
            description: row.description,
            status: TaskStatus::from_str(&row.status),
            priority: row.priority,
            assignee: row.assignee,
            labels,
            estimated_effort: row.estimated_effort,
            actual_effort: row.actual_effort,
            blocked_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
            started_at: row.started_at,
            completed_at: row.completed_at,
        }
    }
}

/// Task status (Kanban column).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Backlog,
    Todo,
    InProgress,
    Review,
    Done,
}

impl TaskStatus {
    /// Parse from string.
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "todo" => Self::Todo,
            "in_progress" => Self::InProgress,
            "review" => Self::Review,
            "done" => Self::Done,
            _ => Self::Backlog,
        }
    }

    /// Convert to string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Backlog => "backlog",
            Self::Todo => "todo",
            Self::InProgress => "in_progress",
            Self::Review => "review",
            Self::Done => "done",
        }
    }

    /// Check if transition to another status is valid.
    pub fn can_transition_to(&self, to: &Self) -> bool {
        match (self, to) {
            // Backlog -> Todo
            (Self::Backlog, Self::Todo) => true,
            // Todo -> InProgress or Backlog
            (Self::Todo, Self::InProgress) => true,
            (Self::Todo, Self::Backlog) => true,
            // InProgress -> Review or Done or Todo (blocked)
            (Self::InProgress, Self::Review) => true,
            (Self::InProgress, Self::Done) => true,
            (Self::InProgress, Self::Todo) => true,
            // Review -> Done or InProgress (needs fixes)
            (Self::Review, Self::Done) => true,
            (Self::Review, Self::InProgress) => true,
            // Done can go back to Todo (reopened)
            (Self::Done, Self::Todo) => true,
            // Same state is always valid
            (a, b) if a == b => true,
            _ => false,
        }
    }
}

/// A Kanban board.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Board {
    pub columns: Vec<BoardColumn>,
}

/// A column on the Kanban board.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardColumn {
    pub name: String,
    pub wip_limit: Option<i64>,
    pub tasks: Vec<Task>,
}

/// WIP status overview.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WipStatus {
    pub columns: Vec<ColumnWipStatus>,
}

/// WIP status for a single column.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnWipStatus {
    pub name: String,
    pub limit: Option<i64>,
    pub current: i64,
    pub is_exceeded: bool,
}
