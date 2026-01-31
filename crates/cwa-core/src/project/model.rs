//! Project domain models.

use serde::{Deserialize, Serialize};
use cwa_db::queries::projects::ProjectRow;

/// A CWA project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub constitution_path: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

impl Project {
    /// Create from database row.
    pub fn from_row(row: ProjectRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            description: row.description,
            constitution_path: row.constitution_path,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// Extended project metadata for context management.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub name: String,
    pub description: String,
    pub tech_stack: Vec<String>,
    pub main_features: Vec<String>,
    pub constraints: Vec<String>,
    pub updated_at: String,
}

impl ProjectInfo {
    /// Create a new ProjectInfo with current timestamp.
    pub fn new(
        name: String,
        description: String,
        tech_stack: Vec<String>,
        main_features: Vec<String>,
        constraints: Vec<String>,
    ) -> Self {
        Self {
            name,
            description,
            tech_stack,
            main_features,
            constraints,
            updated_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Create from JSON string.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Serialize to JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Format as compact markdown for MCP responses and CLAUDE.md.
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str(&format!("# {}\n\n", self.name));

        if !self.description.is_empty() {
            md.push_str(&format!("{}\n\n", self.description));
        }

        if !self.tech_stack.is_empty() {
            md.push_str("## Tech Stack\n\n");
            for tech in &self.tech_stack {
                md.push_str(&format!("- {}\n", tech));
            }
            md.push('\n');
        }

        if !self.main_features.is_empty() {
            md.push_str("## Key Features\n\n");
            for feature in &self.main_features {
                md.push_str(&format!("- {}\n", feature));
            }
            md.push('\n');
        }

        if !self.constraints.is_empty() {
            md.push_str("## Constraints\n\n");
            for constraint in &self.constraints {
                md.push_str(&format!("- {}\n", constraint));
            }
            md.push('\n');
        }

        md.push_str(&format!("_Last updated: {}_\n", self.updated_at));
        md.push_str("\n> Run `cwa update` to refresh project context.\n");

        md
    }
}
