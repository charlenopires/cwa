//! Terminal output formatting.

use colored::{ColoredString, Colorize};
use cwa_core::spec::model::Spec;
use cwa_core::task::model::{Board, BoardColumn, Task, WipStatus};
use cwa_core::domain::model::{BoundedContext, GlossaryTerm, ContextMap};
use unicode_width::UnicodeWidthStr;

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
            cwa_core::spec::model::SpecStatus::InReview => "in_review".cyan(),
            cwa_core::spec::model::SpecStatus::Accepted => "accepted".blue(),
            cwa_core::spec::model::SpecStatus::Completed => "completed".green(),
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

/// Print tasks as a table.
pub fn print_tasks_table(tasks: &[Task]) {
    if tasks.is_empty() {
        println!("{}", "No tasks found.".dimmed());
        return;
    }

    println!(
        "{:<10} {:<30} {:<12} {:<10}",
        "ID", "Title", "Status", "Priority"
    );
    println!("{}", "─".repeat(65));

    for task in tasks {
        let status_colored = match task.status.as_str() {
            "in_progress" => "in_progress".yellow(),
            "done" => "done".green(),
            "review" => "review".cyan(),
            "todo" => "todo".normal(),
            s => s.dimmed(),
        };

        println!(
            "{:<10} {:<30} {:<12} {:<10}",
            &task.id[..8],
            truncate(&task.title, 28),
            status_colored,
            task.priority
        );
    }

    println!();
    println!("{} task(s) total", tasks.len());
}

/// Get terminal width, defaulting to 80.
fn term_width() -> usize {
    terminal_size::terminal_size()
        .map(|(w, _)| w.0 as usize)
        .unwrap_or(80)
}

/// Pad a plain string to a given visual width (right-padded).
fn pad_right(s: &str, width: usize) -> String {
    let visual = UnicodeWidthStr::width(s);
    if visual >= width {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(width - visual))
    }
}

/// Truncate a string respecting visual width.
fn truncate_visual(s: &str, max_width: usize) -> String {
    if UnicodeWidthStr::width(s) <= max_width {
        return s.to_string();
    }
    if max_width <= 3 {
        return ".".repeat(max_width);
    }
    let mut result = String::new();
    let mut current_width = 0;
    for ch in s.chars() {
        let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if current_width + ch_width > max_width - 2 {
            break;
        }
        result.push(ch);
        current_width += ch_width;
    }
    result.push_str("..");
    result
}

/// Get display name for a column, abbreviated for narrow widths.
fn column_display_name(name: &str, max_width: usize) -> String {
    let full = name.to_uppercase();
    if UnicodeWidthStr::width(full.as_str()) <= max_width {
        return full;
    }
    match name {
        "in_progress" => "IN_PROG".to_string(),
        "backlog" => "BKLOG".to_string(),
        _ => full,
    }
}

/// Format column header text (plain, for width calculation).
fn column_header_plain(name: &str, count: usize, wip_limit: Option<i64>, max_width: usize) -> String {
    let suffix = if let Some(l) = wip_limit {
        format!(" {}/{}", count, l)
    } else if count > 0 {
        format!(" {}", count)
    } else {
        String::new()
    };
    let suffix_width = UnicodeWidthStr::width(suffix.as_str());
    let name_budget = if max_width > suffix_width { max_width - suffix_width } else { max_width };
    let display_name = column_display_name(name, name_budget);
    let full = format!("{}{}", display_name, suffix);
    if UnicodeWidthStr::width(full.as_str()) > max_width {
        truncate_visual(&full, max_width)
    } else {
        full
    }
}

/// Get a colored header for a column name.
fn column_header_colored(name: &str, count: usize, wip_limit: Option<i64>, max_width: usize) -> ColoredString {
    let label = column_header_plain(name, count, wip_limit, max_width);

    let exceeded = wip_limit.map_or(false, |l| count as i64 > l);

    if exceeded {
        return label.red().bold();
    }

    match name {
        "backlog" => label.white().dimmed(),
        "todo" => label.blue().bold(),
        "in_progress" => label.yellow().bold(),
        "review" => label.magenta().bold(),
        "done" => label.green().bold(),
        _ => label.normal(),
    }
}

/// Get priority indicator.
fn priority_indicator(priority: &str) -> ColoredString {
    match priority {
        "critical" => "!!".red().bold(),
        "high" => "! ".yellow(),
        "medium" => "· ".dimmed(),
        "low" => "  ".dimmed(),
        _ => "  ".normal(),
    }
}

/// Print a task card for the wide board layout.
fn format_task_card(task: &Task, width: usize) -> String {
    let indicator = match task.priority.as_str() {
        "critical" => "!!",
        "high" => "! ",
        "medium" => "· ",
        _ => "  ",
    };
    // 2 chars for indicator + 1 space + title
    let title_width = if width > 4 { width - 3 } else { 1 };
    let title = truncate_visual(&task.title, title_width);
    format!("{} {}", indicator, pad_right(&title, title_width))
}

/// Print the Kanban board.
pub fn print_board(board: &Board) {
    let total_tasks: usize = board.columns.iter().map(|c| c.tasks.len()).sum();

    if total_tasks == 0 {
        println!("{}", "No tasks found. Create tasks with 'cwa task new <title>'.".dimmed());
        return;
    }

    let width = term_width();

    if width < 60 {
        print_board_compact(board);
    } else {
        print_board_wide(board, width);
    }
}

/// Wide/standard Kanban board layout with columns side-by-side.
fn print_board_wide(board: &Board, term_w: usize) {
    // Filter columns that have tasks or are active workflow columns
    let visible_columns: Vec<&BoardColumn> = if term_w < 100 {
        // For medium terminals, skip empty backlog/done
        board.columns.iter()
            .filter(|c| !c.tasks.is_empty() || c.name == "todo" || c.name == "in_progress" || c.name == "review")
            .collect()
    } else {
        board.columns.iter().collect()
    };

    if visible_columns.is_empty() {
        println!("{}", "No tasks found.".dimmed());
        return;
    }

    let num_cols = visible_columns.len();
    // Distribute width: subtract borders (num_cols + 1 border chars)
    let available = if term_w > num_cols + 1 { term_w - num_cols - 1 } else { num_cols * 10 };
    let col_width = available / num_cols;
    let col_width = col_width.max(12).min(35);

    // ── Header ──
    print!("{}", "┌".dimmed());
    for (i, _) in visible_columns.iter().enumerate() {
        print!("{}", "─".repeat(col_width).dimmed());
        if i < num_cols - 1 {
            print!("{}", "┬".dimmed());
        }
    }
    println!("{}", "┐".dimmed());

    print!("{}", "│".dimmed());
    for (i, col) in visible_columns.iter().enumerate() {
        let header = column_header_colored(&col.name, col.tasks.len(), col.wip_limit, col_width);
        let header_plain = column_header_plain(&col.name, col.tasks.len(), col.wip_limit, col_width);
        let header_width = UnicodeWidthStr::width(header_plain.as_str());
        let padding = if col_width > header_width { col_width - header_width } else { 0 };
        let left_pad = padding / 2;
        let right_pad = padding - left_pad;
        print!("{}{}{}", " ".repeat(left_pad), header, " ".repeat(right_pad));
        if i < num_cols - 1 {
            print!("{}", "│".dimmed());
        }
    }
    println!("{}", "│".dimmed());

    // ── Header separator ──
    print!("{}", "├".dimmed());
    for (i, _) in visible_columns.iter().enumerate() {
        print!("{}", "─".repeat(col_width).dimmed());
        if i < num_cols - 1 {
            print!("{}", "┼".dimmed());
        }
    }
    println!("{}", "┤".dimmed());

    // ── Task rows ──
    let max_tasks = visible_columns.iter().map(|c| c.tasks.len()).max().unwrap_or(0);

    for i in 0..max_tasks {
        print!("{}", "│".dimmed());
        for (ci, col) in visible_columns.iter().enumerate() {
            if let Some(task) = col.tasks.get(i) {
                let card = format_task_card(task, col_width);
                // Apply color based on column
                let colored_card = match col.name.as_str() {
                    "in_progress" => {
                        let ind = priority_indicator(&task.priority);
                        let title_w = if col_width > 4 { col_width - 3 } else { 1 };
                        let title = truncate_visual(&task.title, title_w);
                        let padded_title = pad_right(&title, title_w);
                        format!("{} {}", ind, padded_title.yellow())
                    }
                    "done" => {
                        let title_w = if col_width > 4 { col_width - 3 } else { 1 };
                        let title = truncate_visual(&task.title, title_w);
                        let padded_title = pad_right(&title, title_w);
                        format!("{} {}", "✓ ".green(), padded_title.green().dimmed())
                    }
                    "review" => {
                        let ind = priority_indicator(&task.priority);
                        let title_w = if col_width > 4 { col_width - 3 } else { 1 };
                        let title = truncate_visual(&task.title, title_w);
                        let padded_title = pad_right(&title, title_w);
                        format!("{} {}", ind, padded_title.magenta())
                    }
                    _ => card,
                };
                // We already padded to col_width in format_task_card / colored_card
                print!("{}", colored_card);
            } else {
                print!("{}", " ".repeat(col_width));
            }
            if ci < num_cols - 1 {
                print!("{}", "│".dimmed());
            }
        }
        println!("{}", "│".dimmed());
    }

    // ── Footer ──
    print!("{}", "└".dimmed());
    for (i, _) in visible_columns.iter().enumerate() {
        print!("{}", "─".repeat(col_width).dimmed());
        if i < num_cols - 1 {
            print!("{}", "┴".dimmed());
        }
    }
    println!("{}", "┘".dimmed());

    // ── Summary line ──
    let total: usize = board.columns.iter().map(|c| c.tasks.len()).sum();
    let done: usize = board.columns.iter()
        .find(|c| c.name == "done")
        .map(|c| c.tasks.len())
        .unwrap_or(0);
    if total > 0 {
        let progress = if total > 0 { (done * 100) / total } else { 0 };
        println!(
            " {} {} tasks {} {} done ({}%)",
            "■".cyan(),
            total.to_string().bold(),
            "·".dimmed(),
            done.to_string().green(),
            progress
        );
    }
}

/// Compact vertical board layout for narrow terminals.
fn print_board_compact(board: &Board) {
    println!("{}", " KANBAN BOARD ".on_blue().white().bold());
    println!();

    for col in &board.columns {
        if col.tasks.is_empty() {
            continue;
        }

        let header = column_header_colored(&col.name, col.tasks.len(), col.wip_limit, 30);
        println!(" {} {}", "▸".dimmed(), header);

        for task in &col.tasks {
            let indicator = priority_indicator(&task.priority);
            let id_short = if task.id.len() >= 6 { &task.id[..6] } else { &task.id };
            let title_colored: ColoredString = match col.name.as_str() {
                "in_progress" => task.title.as_str().yellow(),
                "done" => task.title.as_str().green().dimmed(),
                "review" => task.title.as_str().magenta(),
                _ => task.title.as_str().normal(),
            };
            println!(
                "   {} {} {}",
                indicator,
                title_colored,
                id_short.dimmed()
            );
        }
        println!();
    }
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
