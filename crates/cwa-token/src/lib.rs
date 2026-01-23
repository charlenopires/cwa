//! # CWA Token
//!
//! Token counting, cost estimation, and optimization for CWA.
//!
//! Analyzes CLAUDE.md, agent files, and memory entries
//! to provide token usage reports and reduction suggestions.

pub mod analyzer;
pub mod optimizer;
pub mod reporter;

pub use analyzer::{TokenCount, analyze_file, analyze_text, analyze_project, count_tokens};
pub use optimizer::{Suggestion, optimize, suggest_for_content};
pub use reporter::TokenReport;
