//! Specification domain models.

use serde::{Deserialize, Serialize};
use cwa_db::queries::specs::SpecRow;

/// A specification (SDD).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spec {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: SpecStatus,
    pub priority: Priority,
    pub acceptance_criteria: Vec<String>,
    pub dependencies: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub archived_at: Option<String>,
}

impl Spec {
    /// Create a Spec from a database row.
    pub fn from_row(row: SpecRow) -> Self {
        let acceptance_criteria: Vec<String> = row
            .acceptance_criteria
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        let dependencies: Vec<String> = row
            .dependencies
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        Self {
            id: row.id,
            project_id: row.project_id,
            title: row.title,
            description: row.description,
            status: SpecStatus::from_str(&row.status),
            priority: Priority::from_str(&row.priority),
            acceptance_criteria,
            dependencies,
            created_at: row.created_at,
            updated_at: row.updated_at,
            archived_at: row.archived_at,
        }
    }
}

/// Specification status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpecStatus {
    Draft,
    Active,
    Validated,
    Archived,
}

impl SpecStatus {
    /// Parse from string.
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "active" => Self::Active,
            "validated" => Self::Validated,
            "archived" => Self::Archived,
            _ => Self::Draft,
        }
    }

    /// Convert to string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Active => "active",
            Self::Validated => "validated",
            Self::Archived => "archived",
        }
    }

    /// Check if transition to another status is valid.
    pub fn can_transition_to(&self, to: &Self) -> bool {
        match (self, to) {
            // Draft can become active
            (Self::Draft, Self::Active) => true,
            // Active can be validated or archived
            (Self::Active, Self::Validated) => true,
            (Self::Active, Self::Archived) => true,
            // Validated can be archived
            (Self::Validated, Self::Archived) => true,
            // Same state is always valid
            (a, b) if a == b => true,
            _ => false,
        }
    }
}

/// Priority level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

impl Priority {
    /// Parse from string.
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "low" => Self::Low,
            "high" => Self::High,
            "critical" => Self::Critical,
            _ => Self::Medium,
        }
    }

    /// Convert to string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }
}
