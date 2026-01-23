//! Board domain models for the web Kanban UI.

use serde::{Deserialize, Serialize};

/// A Kanban board containing columns and cards.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Board {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub description: Option<String>,
    pub columns: Vec<Column>,
    pub created_at: String,
    pub updated_at: String,
}

/// A column within a board.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Column {
    pub id: String,
    pub board_id: String,
    pub name: String,
    pub position: i32,
    pub color: Option<String>,
    pub wip_limit: Option<i32>,
    pub cards: Vec<Card>,
}

/// A card within a column.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Card {
    pub id: String,
    pub column_id: String,
    pub title: String,
    pub description: Option<String>,
    pub position: i32,
    pub priority: Option<Priority>,
    pub due_date: Option<String>,
    pub labels: Vec<Label>,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

/// Card priority levels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

impl Priority {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "low" => Some(Self::Low),
            "medium" => Some(Self::Medium),
            "high" => Some(Self::High),
            "critical" => Some(Self::Critical),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }

    pub fn css_color(&self) -> &'static str {
        match self {
            Self::Low => "#6b7280",
            Self::Medium => "#3b82f6",
            Self::High => "#f59e0b",
            Self::Critical => "#ef4444",
        }
    }
}

/// A label that can be attached to cards.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    pub id: String,
    pub board_id: String,
    pub name: String,
    pub color: String,
}

/// Default column definitions for a new board.
pub const DEFAULT_COLUMNS: &[(&str, Option<i32>, &str)] = &[
    ("Backlog", None, "#6b7280"),
    ("TODO", Some(5), "#3b82f6"),
    ("In Progress", Some(2), "#f59e0b"),
    ("Review", Some(2), "#8b5cf6"),
    ("Done", None, "#10b981"),
];
