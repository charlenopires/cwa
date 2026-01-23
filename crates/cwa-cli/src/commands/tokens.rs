//! Token analysis CLI commands.

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use std::path::Path;

#[derive(Subcommand)]
pub enum TokenCommands {
    /// Analyze token usage for a file or all context files
    Analyze {
        /// File path to analyze (omit for all context files)
        path: Option<String>,
        /// Analyze all context files
        #[arg(long)]
        all: bool,
    },

    /// Suggest optimizations to reduce token usage
    Optimize {
        /// Target token budget
        #[arg(long, default_value = "8000")]
        budget: usize,
    },

    /// Generate a full token usage report
    Report,
}

pub async fn execute(cmd: TokenCommands, project_dir: &Path) -> Result<()> {
    match cmd {
        TokenCommands::Analyze { path, all } => {
            if all || path.is_none() {
                cmd_analyze_all(project_dir)
            } else {
                cmd_analyze_file(Path::new(path.as_ref().unwrap()))
            }
        }
        TokenCommands::Optimize { budget } => cmd_optimize(project_dir, budget),
        TokenCommands::Report => cmd_report(project_dir),
    }
}

/// Analyze a single file.
fn cmd_analyze_file(path: &Path) -> Result<()> {
    let count = cwa_token::analyze_file(path)?;

    println!("{}", "Token Analysis".bold());
    println!("{}", "─".repeat(40));
    println!("  File:       {}", count.source);
    println!("  Tokens:     {}", count.tokens.to_string().cyan());
    println!("  Characters: {}", count.characters);
    println!("  Lines:      {}", count.lines);
    println!("{}", "─".repeat(40));

    // Show content-specific suggestions
    let content = std::fs::read_to_string(path)?;
    let suggestions = cwa_token::suggest_for_content(&count.source, &content)?;

    if !suggestions.is_empty() {
        println!("\n{}", "Suggestions:".bold());
        for (i, s) in suggestions.iter().enumerate() {
            println!("  {}. {} (~{} tokens)", i + 1, s.action, s.estimated_savings);
        }
    }

    Ok(())
}

/// Analyze all context files in the project.
fn cmd_analyze_all(project_dir: &Path) -> Result<()> {
    let counts = cwa_token::analyze_project(project_dir)?;

    if counts.is_empty() {
        println!("{}", "No context files found.".dimmed());
        return Ok(());
    }

    let total: usize = counts.iter().map(|c| c.tokens).sum();

    println!("{}", "Token Analysis (All Context Files)".bold());
    println!("{}", "─".repeat(60));

    // Sort by token count descending
    let mut sorted = counts.clone();
    sorted.sort_by(|a, b| b.tokens.cmp(&a.tokens));

    for count in &sorted {
        let pct = if total > 0 { count.tokens * 100 / total } else { 0 };
        let tokens_str = format!("{:>6}", count.tokens);
        let colored_tokens = if count.tokens > 2000 {
            tokens_str.red()
        } else if count.tokens > 500 {
            tokens_str.yellow()
        } else {
            tokens_str.green()
        };

        // Shorten path relative to project dir
        let display_path = count.source
            .strip_prefix(&project_dir.display().to_string())
            .unwrap_or(&count.source)
            .trim_start_matches('/');

        println!("  {} ({:>2}%) {}", colored_tokens, pct, display_path);
    }

    println!("{}", "─".repeat(60));
    println!("  {} total tokens across {} files", total.to_string().bold(), counts.len());

    Ok(())
}

/// Suggest optimizations for the token budget.
fn cmd_optimize(project_dir: &Path, budget: usize) -> Result<()> {
    let counts = cwa_token::analyze_project(project_dir)?;
    let total: usize = counts.iter().map(|c| c.tokens).sum();

    println!("{} Budget: {} tokens, Current: {} tokens",
        "Token Optimization".bold(),
        budget.to_string().green(),
        total.to_string().cyan()
    );
    println!("{}", "─".repeat(50));

    if total <= budget {
        println!("{}", "Already within budget.".green());
        return Ok(());
    }

    let excess = total - budget;
    println!("  Excess: {} tokens need to be reduced\n", excess.to_string().red());

    let suggestions = cwa_token::optimize(&counts, budget)?;

    if suggestions.is_empty() {
        println!("{}", "No automatic suggestions available.".dimmed());
    } else {
        println!("{}", "Suggestions:".bold());
        for (i, s) in suggestions.iter().enumerate() {
            let priority = match s.priority {
                cwa_token::optimizer::SuggestionPriority::High => "[HIGH]".red(),
                cwa_token::optimizer::SuggestionPriority::Medium => "[MED]".yellow(),
                cwa_token::optimizer::SuggestionPriority::Low => "[LOW]".dimmed(),
            };
            println!("  {}. {} ~{} tokens: {}",
                i + 1,
                priority,
                s.estimated_savings,
                s.action
            );
        }

        let total_savings: usize = suggestions.iter().map(|s| s.estimated_savings).sum();
        println!("\n  Potential savings: ~{} tokens", total_savings.to_string().green());
    }

    Ok(())
}

/// Generate a full report.
fn cmd_report(project_dir: &Path) -> Result<()> {
    let counts = cwa_token::analyze_project(project_dir)?;
    let suggestions = cwa_token::optimize(&counts, 8000)?;
    let report = cwa_token::TokenReport::new(counts, suggestions);

    println!("{}", report.to_display_string());

    Ok(())
}
