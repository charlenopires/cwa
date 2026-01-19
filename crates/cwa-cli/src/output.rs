//! Terminal output formatting.

use colored::Colorize;
use cwa_core::spec::model::Spec;
use cwa_core::task::model::{Board, WipStatus};
use cwa_core::domain::model::{BoundedContext, GlossaryTerm, ContextMap};

/// Print a single spec.
pub fn print_spec(spec: &Spec) {
    println!("{} {}", spec.title.cyan().bold(), format!("({})", spec.id).dimmed());
    println!();

    if let Some(desc) = &spec.description {
        println!("{}", desc);
        println!();
    }

    println!("{}: {}", "Status".bold(), format!("{:?}", spec.status).yellow());
    println!("{}: {}", "Priority".bold(), format!("{:?}", spec.priority));

    if !spec.acceptance_criteria.is_empty() {
        println!();
        println!("{}", "Acceptance Criteria".bold());
        for (i, criterion) in spec.acceptance_criteria.iter().enumerate() {
            println!("  {}. {}", i + 1, criterion);
        }
    }
}

/// Print specs as a table.
pub fn print_specs_table(specs: &[Spec]) {
    if specs.is_empty() {
        println!("{}", "No specifications found.".dimmed());
        return;
    }

    println!("{:<36} {:<30} {:<12} {:<10}", "ID", "Title", "Status", "Priority");
    println!("{}", "-".repeat(90));

    for spec in specs {
        let status_colored = match spec.status {
            cwa_core::spec::model::SpecStatus::Draft => "draft".dimmed(),
            cwa_core::spec::model::SpecStatus::Active => "active".yellow(),
            cwa_core::spec::model::SpecStatus::Validated => "validated".green(),
            cwa_core::spec::model::SpecStatus::Archived => "archived".dimmed(),
        };

        println!(
            "{:<36} {:<30} {:<12} {:<10}",
            &spec.id[..8],
            truncate(&spec.title, 28),
            status_colored,
            format!("{:?}", spec.priority).to_lowercase()
        );
    }
}

/// Print the Kanban board.
pub fn print_board(board: &Board) {
    // Calculate column widths
    let col_width = 25;

    // Print header
    print!("{}", "│".dimmed());
    for col in &board.columns {
        let header = format!(
            " {} {} ",
            col.name.to_uppercase(),
            col.wip_limit.map_or(String::new(), |l| format!("({})", l))
        );
        print!("{:^width$}{}", header.bold(), "│".dimmed(), width = col_width);
    }
    println!();

    // Print separator
    print!("{}", "├".dimmed());
    for _ in &board.columns {
        print!("{}{}", "─".repeat(col_width).dimmed(), "┼".dimmed());
    }
    println!();

    // Find max tasks in any column
    let max_tasks = board.columns.iter().map(|c| c.tasks.len()).max().unwrap_or(0);

    // Print tasks row by row
    for i in 0..max_tasks {
        print!("{}", "│".dimmed());
        for col in &board.columns {
            if let Some(task) = col.tasks.get(i) {
                let task_str = truncate(&task.title, col_width - 2);
                let colored = match col.name.as_str() {
                    "in_progress" => task_str.yellow(),
                    "done" => task_str.green(),
                    _ => task_str.normal(),
                };
                print!(" {:<width$}{}", colored, "│".dimmed(), width = col_width - 1);
            } else {
                print!("{:width$}{}", "", "│".dimmed(), width = col_width);
            }
        }
        println!();
    }

    // Print footer
    print!("{}", "└".dimmed());
    for _ in &board.columns {
        print!("{}{}", "─".repeat(col_width).dimmed(), "┴".dimmed());
    }
    println!();
}

/// Print WIP status.
pub fn print_wip(wip: &WipStatus) {
    println!("{}", "WIP Status".bold());
    println!();

    for col in &wip.columns {
        let status = if col.is_exceeded {
            format!("{}/{}", col.current, col.limit.unwrap_or(0)).red()
        } else if col.limit.is_some() {
            format!("{}/{}", col.current, col.limit.unwrap()).green()
        } else {
            format!("{}/∞", col.current).normal()
        };

        let indicator = if col.is_exceeded { "⚠" } else { "✓" };

        println!(
            "  {} {:15} {}",
            if col.is_exceeded { indicator.red() } else { indicator.green() },
            col.name,
            status
        );
    }
}

/// Print bounded contexts.
pub fn print_contexts(contexts: &[BoundedContext]) {
    if contexts.is_empty() {
        println!("{}", "No bounded contexts defined.".dimmed());
        return;
    }

    println!("{}", "Bounded Contexts".bold());
    println!();

    for ctx in contexts {
        println!("  {} {}", "●".cyan(), ctx.name.bold());
        if let Some(desc) = &ctx.description {
            println!("    {}", desc.dimmed());
        }
    }
}

/// Print context map.
pub fn print_context_map(map: &ContextMap) {
    if map.contexts.is_empty() {
        println!("{}", "No contexts defined.".dimmed());
        return;
    }

    println!("{}", "Context Map".bold());
    println!();

    for ctx in &map.contexts {
        println!("  [{}]", ctx.cyan());
    }

    if !map.relationships.is_empty() {
        println!();
        println!("{}", "Relationships".bold());
        for rel in &map.relationships {
            println!(
                "  {} {} {}",
                rel.upstream_id,
                "→".dimmed(),
                rel.downstream_id
            );
        }
    }
}

/// Print glossary terms.
pub fn print_glossary(terms: &[GlossaryTerm]) {
    if terms.is_empty() {
        println!("{}", "No glossary terms defined.".dimmed());
        return;
    }

    println!("{}", "Domain Glossary".bold());
    println!();

    for term in terms {
        println!("  {}", term.term.cyan().bold());
        println!("    {}", term.definition);
        if !term.aliases.is_empty() {
            println!("    Aliases: {}", term.aliases.join(", ").dimmed());
        }
        println!();
    }
}

/// Truncate a string to a maximum length.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}
