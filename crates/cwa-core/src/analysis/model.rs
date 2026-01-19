//! Analysis domain models.

use serde::{Deserialize, Serialize};

/// An analysis result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Analysis {
    pub analysis_type: String,
    pub title: String,
    pub content: String,
    pub sources: Vec<String>,
}
