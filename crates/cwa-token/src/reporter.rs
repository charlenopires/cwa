//! Token usage reports.
//!
//! Generates formatted reports showing token breakdown by source,
//! estimated costs, and optimization recommendations.

use serde::Serialize;

use crate::analyzer::TokenCount;
use crate::optimizer::Suggestion;

/// Full token usage report.
#[derive(Debug, Clone, Serialize)]
pub struct TokenReport {
    pub total_tokens: usize,
    pub total_characters: usize,
    pub total_lines: usize,
    pub sources: Vec<TokenCount>,
    pub suggestions: Vec<Suggestion>,
}

impl TokenReport {
    /// Create a report from analysis results and suggestions.
    pub fn new(sources: Vec<TokenCount>, suggestions: Vec<Suggestion>) -> Self {
        let total_tokens = sources.iter().map(|s| s.tokens).sum();
        let total_characters = sources.iter().map(|s| s.characters).sum();
        let total_lines = sources.iter().map(|s| s.lines).sum();

        Self {
            total_tokens,
            total_characters,
            total_lines,
            sources,
            suggestions,
        }
    }

    /// Format the report as a human-readable string.
    pub fn to_display_string(&self) -> String {
        let mut output = String::new();

        output.push_str(&format!("Token Usage Report\n"));
        output.push_str(&format!("{}\n", "─".repeat(50)));
        output.push_str(&format!("Total tokens:     {:>8}\n", self.total_tokens));
        output.push_str(&format!("Total characters: {:>8}\n", self.total_characters));
        output.push_str(&format!("Total lines:      {:>8}\n", self.total_lines));
        output.push_str(&format!("{}\n\n", "─".repeat(50)));

        if !self.sources.is_empty() {
            output.push_str("Breakdown by source:\n");

            // Sort by tokens descending for display
            let mut sorted = self.sources.clone();
            sorted.sort_by(|a, b| b.tokens.cmp(&a.tokens));

            for source in &sorted {
                let pct = if self.total_tokens > 0 {
                    source.tokens * 100 / self.total_tokens
                } else {
                    0
                };

                // Create a bar visualization
                let bar_len = (pct / 2).max(1);
                let bar: String = "█".repeat(bar_len);

                output.push_str(&format!(
                    "  {:>6} ({:>2}%) {} {}\n",
                    source.tokens,
                    pct,
                    bar,
                    short_source(&source.source)
                ));
            }
        }

        if !self.suggestions.is_empty() {
            output.push_str(&format!("\nOptimization suggestions:\n"));
            for (i, suggestion) in self.suggestions.iter().enumerate() {
                output.push_str(&format!(
                    "  {}. [~{} tokens] {}: {}\n",
                    i + 1,
                    suggestion.estimated_savings,
                    short_source(&suggestion.source),
                    suggestion.action
                ));
            }

            let total_savings: usize = self.suggestions.iter()
                .map(|s| s.estimated_savings)
                .sum();
            output.push_str(&format!(
                "\n  Potential savings: ~{} tokens ({:.0}%)\n",
                total_savings,
                if self.total_tokens > 0 {
                    total_savings as f64 / self.total_tokens as f64 * 100.0
                } else {
                    0.0
                }
            ));
        }

        output
    }
}

/// Shorten a source path for display.
fn short_source(source: &str) -> &str {
    // Take just the filename or last two path components
    source.rsplit('/').next().unwrap_or(source)
}
