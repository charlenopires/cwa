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
