//! Token optimization suggestions.
//!
//! Analyzes token usage and suggests ways to reduce token consumption
//! while maintaining context quality.

use anyhow::Result;
use serde::Serialize;

use crate::analyzer::{TokenCount, count_tokens};

/// An optimization suggestion.
#[derive(Debug, Clone, Serialize)]
pub struct Suggestion {
    pub source: String,
    pub action: String,
    pub estimated_savings: usize,
    pub priority: SuggestionPriority,
}

/// Priority level for suggestions.
#[derive(Debug, Clone, Serialize)]
pub enum SuggestionPriority {
    High,
    Medium,
    Low,
}

/// Generate optimization suggestions given a token budget.
pub fn optimize(counts: &[TokenCount], budget: usize) -> Result<Vec<Suggestion>> {
    let total: usize = counts.iter().map(|c| c.tokens).sum();
    let mut suggestions = Vec::new();

    if total <= budget {
        return Ok(suggestions);
    }

    let excess = total - budget;

    // Find the largest sources first
    let mut sorted: Vec<&TokenCount> = counts.iter().collect();
    sorted.sort_by(|a, b| b.tokens.cmp(&a.tokens));

    for count in &sorted {
        // Large files (>2000 tokens) can likely be split
        if count.tokens > 2000 {
            let savings = count.tokens / 3; // Estimate 33% savings from splitting
            suggestions.push(Suggestion {
                source: count.source.clone(),
                action: format!(
                    "Split into focused sections ({} tokens, could save ~{})",
                    count.tokens, savings
                ),
                estimated_savings: savings,
                priority: SuggestionPriority::High,
            });
        }

        // Files with many lines relative to tokens (verbose prose)
        if count.lines > 0 && count.tokens / count.lines > 20 {
            let savings = count.tokens / 4;
            suggestions.push(Suggestion {
                source: count.source.clone(),
                action: "Condense verbose descriptions".to_string(),
                estimated_savings: savings,
                priority: SuggestionPriority::Medium,
            });
        }
    }

    // Check for duplicate-looking sources (same prefix)
    let sources: Vec<&str> = counts.iter().map(|c| c.source.as_str()).collect();
    for i in 0..sources.len() {
        for j in (i + 1)..sources.len() {
            if similar_sources(sources[i], sources[j]) {
                let savings = counts[j].tokens.min(counts[i].tokens) / 2;
                suggestions.push(Suggestion {
                    source: format!("{} + {}", sources[i], sources[j]),
                    action: "Consolidate similar files".to_string(),
                    estimated_savings: savings,
                    priority: SuggestionPriority::Medium,
                });
            }
        }
    }

    // Sort by estimated savings
    suggestions.sort_by(|a, b| b.estimated_savings.cmp(&a.estimated_savings));

    // Only keep suggestions that help reach the budget
    let mut cumulative_savings = 0;
    suggestions.retain(|s| {
        if cumulative_savings >= excess {
            false
        } else {
            cumulative_savings += s.estimated_savings;
            true
        }
    });

    Ok(suggestions)
}

/// Suggest optimizations for a single piece of content.
pub fn suggest_for_content(source: &str, content: &str) -> Result<Vec<Suggestion>> {
    let tokens = count_tokens(content)?;
    let lines = content.lines().count();
    let mut suggestions = Vec::new();

    // Check for repeated patterns
    let line_list: Vec<&str> = content.lines().collect();
    let mut seen = std::collections::HashSet::new();
    let mut duplicates = 0;
    for line in &line_list {
        let trimmed = line.trim();
        if !trimmed.is_empty() && trimmed.len() > 20 && !seen.insert(trimmed) {
            duplicates += 1;
        }
    }

    if duplicates > 2 {
        suggestions.push(Suggestion {
            source: source.to_string(),
            action: format!("Remove {} duplicate lines", duplicates),
            estimated_savings: duplicates * 15, // ~15 tokens per duplicate line
            priority: SuggestionPriority::High,
        });
    }

    // Check for excessive comments (lines starting with # or //)
    let comment_lines = line_list.iter()
        .filter(|l| {
            let t = l.trim();
            t.starts_with('#') || t.starts_with("//") || t.starts_with("<!--")
        })
        .count();

    if lines > 0 && comment_lines * 100 / lines > 30 {
        suggestions.push(Suggestion {
            source: source.to_string(),
            action: format!("Reduce comments ({} of {} lines are comments)", comment_lines, lines),
            estimated_savings: comment_lines * 10,
            priority: SuggestionPriority::Medium,
        });
    }

    // Check for very long lines
    let long_lines = line_list.iter().filter(|l| l.len() > 200).count();
    if long_lines > 3 {
        suggestions.push(Suggestion {
            source: source.to_string(),
            action: format!("Truncate {} very long lines (>200 chars)", long_lines),
            estimated_savings: long_lines * 30,
            priority: SuggestionPriority::Low,
        });
    }

    // If total is very high, suggest truncation
    if tokens > 4000 {
        suggestions.push(Suggestion {
            source: source.to_string(),
            action: format!("File is very large ({} tokens). Consider splitting into sections.", tokens),
            estimated_savings: tokens / 3,
            priority: SuggestionPriority::High,
        });
    }

    Ok(suggestions)
}

/// Check if two source paths are similar (same directory, similar names).
fn similar_sources(a: &str, b: &str) -> bool {
    let a_parts: Vec<&str> = a.rsplit('/').collect();
    let b_parts: Vec<&str> = b.rsplit('/').collect();

    // Same directory
    if a_parts.len() >= 2 && b_parts.len() >= 2 && a_parts[1] == b_parts[1] {
        // Check if filenames have similar prefixes
        let a_name = a_parts[0];
        let b_name = b_parts[0];
        let common_prefix = a_name.chars()
            .zip(b_name.chars())
            .take_while(|(a, b)| a == b)
            .count();
        common_prefix > 5
    } else {
        false
    }
}
