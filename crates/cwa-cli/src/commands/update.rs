//! Project update command - interactively updates project metadata and regenerates context files.

use anyhow::{Context, Result};
use clap::Args;
use colored::Colorize;
use dialoguer::Input;
use std::path::Path;

use cwa_core::project::model::ProjectInfo;

#[derive(Args)]
pub struct UpdateArgs {
    /// Skip interactive prompts and use existing values (only regenerate files)
    #[arg(long)]
    pub regenerate_only: bool,

    /// Skip regenerating CLAUDE.md and other context files
    #[arg(long)]
    pub no_regen: bool,
}

pub async fn execute(args: UpdateArgs, project_dir: &Path) -> Result<()> {
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let pool = cwa_db::init_pool(&redis_url).await?;

    let project = cwa_core::project::get_default_project(&pool).await?
        .ok_or_else(|| anyhow::anyhow!("No project found. Run 'cwa init' first."))?;

    println!(
        "{} Updating project: {}\n",
        "→".blue().bold(),
        project.name.cyan()
    );

    // Get existing info for defaults
    let existing = cwa_core::project::get_project_info(&pool, &project.id).await?;

    let info = if args.regenerate_only {
        // Use existing info or create minimal
        existing.unwrap_or_else(|| ProjectInfo::new(
            project.name.clone(),
            project.description.clone().unwrap_or_default(),
            vec![],
            vec![],
            vec![],
        ))
    } else {
        // Interactive prompts
        collect_project_info(&project, existing.as_ref())?
    };

    // Save project info
    cwa_core::project::set_project_info(&pool, &project.id, &info).await?;

    // Update project basic fields if name/description changed
    let current_desc = project.description.clone().unwrap_or_default();
    if info.name != project.name || info.description != current_desc {
        cwa_core::project::update_project(
            &pool,
            &project.id,
            &info.name,
            Some(&info.description),
        ).await?;
    }

    println!("{} Project info saved", "✓".green().bold());

    if !args.no_regen {
        regenerate_all(&pool, &project.id, project_dir).await?;
    }

    println!(
        "\n{} Project updated successfully!",
        "✓".green().bold()
    );

    Ok(())
}

fn collect_project_info(
    project: &cwa_core::project::model::Project,
    existing: Option<&ProjectInfo>,
) -> Result<ProjectInfo> {
    println!("{}", "Enter project information (press Enter to keep default):\n".dimmed());

    // 1. Project name
    let name_default = existing
        .map(|e| e.name.clone())
        .unwrap_or_else(|| project.name.clone());
    let name: String = Input::new()
        .with_prompt("Project name")
        .default(name_default)
        .interact_text()
        .context("Failed to read project name")?;

    // 2. Description
    let desc_default = existing
        .map(|e| e.description.clone())
        .unwrap_or_else(|| project.description.clone().unwrap_or_default());
    let description: String = Input::new()
        .with_prompt("Description")
        .default(desc_default)
        .allow_empty(true)
        .interact_text()
        .context("Failed to read description")?;

    // 3. Tech stack (comma-separated)
    let tech_default = existing
        .map(|e| e.tech_stack.join(", "))
        .unwrap_or_default();
    let tech_input: String = Input::new()
        .with_prompt("Tech stack (comma-separated, e.g., Rust, SQLite, React)")
        .default(tech_default)
        .allow_empty(true)
        .interact_text()
        .context("Failed to read tech stack")?;
    let tech_stack: Vec<String> = tech_input
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // 4. Main features
    println!(
        "\n{} (enter each on a line, empty line to finish):",
        "Main features".bold()
    );
    let existing_features = existing.map(|e| &e.main_features[..]).unwrap_or(&[]);
    let main_features = collect_list("Feature", existing_features)?;

    // 5. Constraints
    println!(
        "\n{} (enter each on a line, empty line to finish):",
        "Constraints/Guidelines".bold()
    );
    let existing_constraints = existing.map(|e| &e.constraints[..]).unwrap_or(&[]);
    let constraints = collect_list("Constraint", existing_constraints)?;

    Ok(ProjectInfo::new(
        name,
        description,
        tech_stack,
        main_features,
        constraints,
    ))
}

fn collect_list(prompt: &str, existing: &[String]) -> Result<Vec<String>> {
    let mut items = Vec::new();
    let mut count = 1;

    // Show existing items first
    if !existing.is_empty() {
        println!("{}", "  Current items (press Enter to keep, or type new ones):".dimmed());
        for item in existing {
            println!("    {} {}", "•".dimmed(), item);
        }
        println!();

        // Ask if user wants to keep existing
        let keep: String = Input::new()
            .with_prompt("Keep existing? (y/n)")
            .default("y".to_string())
            .interact_text()?;

        if keep.to_lowercase().starts_with('y') {
            items.extend(existing.iter().cloned());
            println!("{}", "  Add more items (empty line to finish):".dimmed());
        }
    }

    loop {
        let input: String = Input::new()
            .with_prompt(format!("{} #{}", prompt, count + items.len()))
            .allow_empty(true)
            .interact_text()
            .context("Failed to read input")?;

        if input.is_empty() {
            break;
        }

        items.push(input);
        count += 1;
    }

    Ok(items)
}

async fn regenerate_all(pool: &cwa_db::DbPool, project_id: &str, project_dir: &Path) -> Result<()> {
    println!("\n{} Regenerating context files...", "→".blue().bold());

    // 1. CLAUDE.md
    let claude_md = cwa_codegen::generate_claude_md(pool, project_id).await?;
    cwa_codegen::write_claude_md(&claude_md, project_dir)?;
    println!("  {} CLAUDE.md", "✓".green());

    // 2. Agents
    let agents = cwa_codegen::generate_all_agents(pool, project_id).await?;
    if !agents.is_empty() {
        let output_dir = project_dir.join(".claude/agents");
        cwa_codegen::write_agents(&agents, &output_dir)?;
        println!("  {} {} agents", "✓".green(), agents.len());
    }

    // 3. Skills
    let skills = cwa_codegen::generate_all_skills(pool, project_id).await?;
    if !skills.is_empty() {
        let output_dir = project_dir.join(".claude/skills");
        cwa_codegen::write_skills(&skills, &output_dir)?;
        println!("  {} {} skills", "✓".green(), skills.len());
    }

    // 4. Commands
    let commands = cwa_codegen::generate_all_commands();
    let output_dir = project_dir.join(".claude/commands");
    cwa_codegen::write_commands(&commands, &output_dir)?;
    println!("  {} {} commands", "✓".green(), commands.len());

    // 5. Hooks
    let hooks = cwa_codegen::generate_hooks(pool, project_id).await?;
    cwa_codegen::write_hooks(&hooks, project_dir)?;
    println!("  {} hooks.json", "✓".green());

    Ok(())
}
