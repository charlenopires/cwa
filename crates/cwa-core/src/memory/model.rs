//! Memory domain models.

use serde::{Deserialize, Serialize};
use cwa_db::queries::memory::{MemoryRow, SessionRow};

/// A memory entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub project_id: String,
    pub session_id: Option<String>,
    pub entry_type: String,
    pub content: String,
    pub importance: String,
    pub tags: Vec<String>,
    pub related_entity_type: Option<String>,
    pub related_entity_id: Option<String>,
    pub created_at: String,
    pub expires_at: Option<String>,
}

impl MemoryEntry {
    /// Create from database row.
    pub fn from_row(row: MemoryRow) -> Self {
        let tags: Vec<String> = row
            .tags
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        Self {
            id: row.id,
            project_id: row.project_id,
            session_id: row.session_id,
            entry_type: row.entry_type,
            content: row.content,
            importance: row.importance,
            tags,
            related_entity_type: row.related_entity_type,
            related_entity_id: row.related_entity_id,
            created_at: row.created_at,
            expires_at: row.expires_at,
        }
    }
}

/// A development session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub project_id: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub summary: Option<String>,
    pub goals: Vec<String>,
    pub accomplishments: Vec<String>,
}

impl Session {
    /// Create from database row.
    pub fn from_row(row: SessionRow) -> Self {
        let goals: Vec<String> = row
            .goals
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        let accomplishments: Vec<String> = row
            .accomplishments
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        Self {
            id: row.id,
            project_id: row.project_id,
            started_at: row.started_at,
            ended_at: row.ended_at,
            summary: row.summary,
            goals,
            accomplishments,
        }
    }
}

/// A compact context summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSummary {
    pub project_name: String,
    pub current_task: Option<String>,
    pub active_spec: Option<String>,
    pub task_counts: super::TaskCounts,
    pub recent_decisions: Vec<String>,
    pub recent_insights: Vec<String>,
}

impl ContextSummary {
    /// Format as compact text for CLAUDE.md.
    pub fn to_compact_string(&self) -> String {
        let mut output = String::new();

        output.push_str(&format!("# {}\n\n", self.project_name));

        output.push_str("## Current Focus\n");
        if let Some(task) = &self.current_task {
            output.push_str(&format!("- **Task**: {}\n", task));
        } else {
            output.push_str("- **Task**: None in progress\n");
        }

        if let Some(spec) = &self.active_spec {
            output.push_str(&format!("- **Spec**: {}\n", spec));
        }

        output.push_str(&format!(
            "\n## Board: {} backlog | {} todo | {} in progress | {} review | {} done\n",
            self.task_counts.backlog,
            self.task_counts.todo,
            self.task_counts.in_progress,
            self.task_counts.review,
            self.task_counts.done
        ));

        if !self.recent_decisions.is_empty() {
            output.push_str("\n## Key Decisions\n");
            for decision in &self.recent_decisions {
                output.push_str(&format!("- {}\n", decision));
            }
        }

        if !self.recent_insights.is_empty() {
            output.push_str("\n## Recent Insights\n");
            for insight in &self.recent_insights {
                output.push_str(&format!("- {}\n", insight));
            }
        }

        output
    }
}
