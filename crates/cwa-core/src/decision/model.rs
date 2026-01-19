//! Decision (ADR) domain models.

use serde::{Deserialize, Serialize};
use cwa_db::queries::decisions::DecisionRow;

/// An Architectural Decision Record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub status: DecisionStatus,
    pub context: String,
    pub decision: String,
    pub consequences: Vec<String>,
    pub alternatives: Vec<Alternative>,
    pub related_specs: Vec<String>,
    pub superseded_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl Decision {
    /// Create from database row.
    pub fn from_row(row: DecisionRow) -> Self {
        let consequences: Vec<String> = row
            .consequences
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        let alternatives: Vec<Alternative> = row
            .alternatives
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        let related_specs: Vec<String> = row
            .related_specs
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        Self {
            id: row.id,
            project_id: row.project_id,
            title: row.title,
            status: DecisionStatus::from_str(&row.status),
            context: row.context,
            decision: row.decision,
            consequences,
            alternatives,
            related_specs,
            superseded_by: row.superseded_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// Decision status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionStatus {
    Proposed,
    Accepted,
    Deprecated,
    Superseded,
}

impl DecisionStatus {
    /// Parse from string.
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "accepted" => Self::Accepted,
            "deprecated" => Self::Deprecated,
            "superseded" => Self::Superseded,
            _ => Self::Proposed,
        }
    }

    /// Convert to string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Proposed => "proposed",
            Self::Accepted => "accepted",
            Self::Deprecated => "deprecated",
            Self::Superseded => "superseded",
        }
    }
}

/// An alternative considered but not chosen.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alternative {
    pub title: String,
    pub description: String,
    pub reason_rejected: String,
}
