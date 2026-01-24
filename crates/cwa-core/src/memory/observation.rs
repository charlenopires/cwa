//! Observation domain model.
//!
//! Structured capture of development activity following the claude-mem pattern:
//! observations with types, facts, concepts, and confidence lifecycle.

use serde::{Deserialize, Serialize};
use cwa_db::queries::observations::{ObservationRow, ObservationIndexRow, SummaryRow};

/// Types of observations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ObservationType {
    Bugfix,
    Feature,
    Refactor,
    Discovery,
    Decision,
    Change,
    Insight,
}

impl ObservationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Bugfix => "bugfix",
            Self::Feature => "feature",
            Self::Refactor => "refactor",
            Self::Discovery => "discovery",
            Self::Decision => "decision",
            Self::Change => "change",
            Self::Insight => "insight",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "bugfix" => Some(Self::Bugfix),
            "feature" => Some(Self::Feature),
            "refactor" => Some(Self::Refactor),
            "discovery" => Some(Self::Discovery),
            "decision" => Some(Self::Decision),
            "change" => Some(Self::Change),
            "insight" => Some(Self::Insight),
            _ => None,
        }
    }

    pub fn all_variants() -> &'static [&'static str] {
        &["bugfix", "feature", "refactor", "discovery", "decision", "change", "insight"]
    }
}

/// Concept types for observations (how knowledge is categorized).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ObservationConcept {
    HowItWorks,
    WhyItExists,
    WhatChanged,
    ProblemSolution,
    Gotcha,
    Pattern,
    TradeOff,
}

impl ObservationConcept {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::HowItWorks => "how-it-works",
            Self::WhyItExists => "why-it-exists",
            Self::WhatChanged => "what-changed",
            Self::ProblemSolution => "problem-solution",
            Self::Gotcha => "gotcha",
            Self::Pattern => "pattern",
            Self::TradeOff => "trade-off",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "how-it-works" => Some(Self::HowItWorks),
            "why-it-exists" => Some(Self::WhyItExists),
            "what-changed" => Some(Self::WhatChanged),
            "problem-solution" => Some(Self::ProblemSolution),
            "gotcha" => Some(Self::Gotcha),
            "pattern" => Some(Self::Pattern),
            "trade-off" => Some(Self::TradeOff),
            _ => None,
        }
    }

    pub fn all_variants() -> &'static [&'static str] {
        &["how-it-works", "why-it-exists", "what-changed", "problem-solution", "gotcha", "pattern", "trade-off"]
    }
}

/// Full observation with all details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub id: String,
    pub project_id: String,
    pub session_id: Option<String>,
    pub obs_type: String,
    pub title: String,
    pub narrative: Option<String>,
    pub facts: Vec<String>,
    pub concepts: Vec<String>,
    pub files_modified: Vec<String>,
    pub files_read: Vec<String>,
    pub related_entity_type: Option<String>,
    pub related_entity_id: Option<String>,
    pub confidence: f64,
    pub embedding_id: Option<String>,
    pub created_at: String,
}

impl Observation {
    pub fn from_row(row: ObservationRow) -> Self {
        let facts: Vec<String> = row.facts
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        let concepts: Vec<String> = row.concepts
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        let files_modified: Vec<String> = row.files_modified
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        let files_read: Vec<String> = row.files_read
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        Self {
            id: row.id,
            project_id: row.project_id,
            session_id: row.session_id,
            obs_type: row.obs_type,
            title: row.title,
            narrative: row.narrative,
            facts,
            concepts,
            files_modified,
            files_read,
            related_entity_type: row.related_entity_type,
            related_entity_id: row.related_entity_id,
            confidence: row.confidence,
            embedding_id: row.embedding_id,
            created_at: row.created_at,
        }
    }
}

/// Compact observation for progressive disclosure (index only, ~50 tokens).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationIndex {
    pub id: String,
    pub obs_type: String,
    pub title: String,
    pub confidence: f64,
    pub created_at: String,
}

impl ObservationIndex {
    pub fn from_row(row: ObservationIndexRow) -> Self {
        Self {
            id: row.id,
            obs_type: row.obs_type,
            title: row.title,
            confidence: row.confidence,
            created_at: row.created_at,
        }
    }
}

/// Session/time-range summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    pub id: String,
    pub project_id: String,
    pub session_id: Option<String>,
    pub content: String,
    pub observations_count: i64,
    pub key_facts: Vec<String>,
    pub time_range_start: Option<String>,
    pub time_range_end: Option<String>,
    pub created_at: String,
}

impl Summary {
    pub fn from_row(row: SummaryRow) -> Self {
        let key_facts: Vec<String> = row.key_facts
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        Self {
            id: row.id,
            project_id: row.project_id,
            session_id: row.session_id,
            content: row.content,
            observations_count: row.observations_count,
            key_facts,
            time_range_start: row.time_range_start,
            time_range_end: row.time_range_end,
            created_at: row.created_at,
        }
    }
}
