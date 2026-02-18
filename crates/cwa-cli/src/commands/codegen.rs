//! Code generation CLI commands.

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use std::path::Path;

#[derive(Subcommand)]
pub enum CodegenCommands {
    /// Generate a subagent for a bounded context
    Agent {
        /// Bounded context ID (or --all)
        context_id: Option<String>,
        /// Generate agents for all contexts
        #[arg(long)]
        all: bool,
        /// Preview without writing files
        #[arg(long)]
        dry_run: bool,
    },

    /// Generate a skill from a spec
    Skill {
        /// Spec ID
        spec_id: String,
        /// Preview without writing files
        #[arg(long)]
        dry_run: bool,
    },

    /// Generate validation hooks from domain rules
    Hooks {
        /// Preview without writing files
        #[arg(long)]
        dry_run: bool,
    },

    /// Generate Claude Code commands
    Commands {
        /// Preview without writing files
        #[arg(long)]
        dry_run: bool,
    },

    /// Regenerate CLAUDE.md from current state
    ClaudeMd {
        /// Preview without writing files
        #[arg(long)]
        dry_run: bool,
    },

    /// Generate all artifacts
    All {
        /// Preview without writing files
        #[arg(long)]
        dry_run: bool,
    },
}

pub async fn execute(cmd: CodegenCommands, project_dir: &Path) -> Result<()> {
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let pool = cwa_db::init_pool(&redis_url).await?;

    let project = cwa_core::project::get_default_project(&pool).await?
        .ok_or_else(|| anyhow::anyhow!("No project found. Run 'cwa init' first."))?;

    match cmd {
        CodegenCommands::Agent { context_id, all, dry_run } => {
            if all || context_id.is_none() {
                cmd_agents_all(&pool, &project.id, project_dir, dry_run).await
            } else {
                cmd_agent(&pool, context_id.as_deref().unwrap(), project_dir, dry_run).await
            }
        }
        CodegenCommands::Skill { spec_id, dry_run } => {
            cmd_skill(&pool, &project.id, &spec_id, project_dir, dry_run).await
        }
        CodegenCommands::Hooks { dry_run } => {
            cmd_hooks(&pool, &project.id, project_dir, dry_run).await
        }
        CodegenCommands::Commands { dry_run } => {
            cmd_commands(project_dir, dry_run)
        }
        CodegenCommands::ClaudeMd { dry_run } => {
            cmd_claude_md(&pool, &project.id, project_dir, dry_run).await
        }
        CodegenCommands::All { dry_run } => {
            cmd_all(&pool, &project.id, project_dir, dry_run).await
        }
    }
}

async fn cmd_agent(pool: &cwa_db::DbPool, context_id: &str, project_dir: &Path, dry_run: bool) -> Result<()> {
    let agent = cwa_codegen::generate_agent(pool, context_id).await?;

    if dry_run {
        println!("{} Would generate: .claude/agents/{}", "→".dimmed(), agent.filename);
        println!("{}", "─".repeat(40));
        println!("{}", agent.content);
    } else {
        let output_dir = project_dir.join(".claude/agents");
        let written = cwa_codegen::write_agents(&[agent.clone()], &output_dir)?;
        println!("{} Generated agent: {}", "✓".green().bold(), written[0]);
    }

    Ok(())
}

async fn cmd_agents_all(pool: &cwa_db::DbPool, project_id: &str, project_dir: &Path, dry_run: bool) -> Result<()> {
    let agents = cwa_codegen::generate_all_agents(pool, project_id).await?;

    if agents.is_empty() {
        println!("{}", "No bounded contexts found. Create one with 'cwa domain context new'.".dimmed());
        return Ok(());
    }

    if dry_run {
        println!("{} Would generate {} agents:", "→".dimmed(), agents.len());
        for agent in &agents {
            println!("  .claude/agents/{} ({})", agent.filename, agent.context_name);
        }
    } else {
        let output_dir = project_dir.join(".claude/agents");
        let written = cwa_codegen::write_agents(&agents, &output_dir)?;
        println!("{} Generated {} agents:", "✓".green().bold(), written.len());
        for path in &written {
            println!("  {}", path);
        }
    }

    Ok(())
}

async fn cmd_skill(pool: &cwa_db::DbPool, project_id: &str, spec_id: &str, project_dir: &Path, dry_run: bool) -> Result<()> {
    let skill = cwa_codegen::generate_skill(pool, project_id, spec_id).await?;

    if dry_run {
        println!("{} Would generate: .claude/skills/{}/{}", "→".dimmed(), skill.dirname, skill.filename);
        println!("{}", "─".repeat(40));
        println!("{}", skill.content);
    } else {
        let output_dir = project_dir.join(".claude/skills");
        let written = cwa_codegen::write_skills(&[skill.clone()], &output_dir)?;
        println!("{} Generated skill: {}", "✓".green().bold(), written[0]);
    }

    Ok(())
}

async fn cmd_hooks(pool: &cwa_db::DbPool, project_id: &str, project_dir: &Path, dry_run: bool) -> Result<()> {
    let hooks = cwa_codegen::generate_hooks(pool, project_id).await?;

    if dry_run {
        println!("{} Would generate hooks.json ({} hooks)", "→".dimmed(), hooks.hook_count);
        if hooks.hook_count > 0 {
            println!("{}", "─".repeat(40));
            println!("{}", hooks.content);
        }
    } else {
        if hooks.hook_count == 0 {
            println!("{}", "No domain invariants found for hook generation.".dimmed());
            return Ok(());
        }
        let path = cwa_codegen::write_hooks(&hooks, project_dir)?;
        println!("{} Generated {} hooks: {}", "✓".green().bold(), hooks.hook_count, path);
    }

    Ok(())
}

fn cmd_commands(project_dir: &Path, dry_run: bool) -> Result<()> {
    let commands = cwa_codegen::generate_all_commands();

    if dry_run {
        println!("{} Would generate {} commands:", "→".dimmed(), commands.len());
        for cmd in &commands {
            println!("  .claude/commands/{}", cmd.filename);
        }
    } else {
        let output_dir = project_dir.join(".claude/commands");
        let written = cwa_codegen::write_commands(&commands, &output_dir)?;
        println!("{} Generated {} commands:", "✓".green().bold(), written.len());
        for path in &written {
            println!("  {}", path);
        }
    }

    Ok(())
}

async fn cmd_claude_md(pool: &cwa_db::DbPool, project_id: &str, project_dir: &Path, dry_run: bool) -> Result<()> {
    let generated = cwa_codegen::generate_claude_md(pool, project_id).await?;

    if dry_run {
        println!("{} Would regenerate CLAUDE.md", "→".dimmed());
        println!("{}", "─".repeat(40));
        println!("{}", generated.content);
    } else {
        let path = cwa_codegen::write_claude_md(&generated, project_dir)?;
        println!("{} Regenerated: {}", "✓".green().bold(), path);
    }

    Ok(())
}

async fn cmd_all(pool: &cwa_db::DbPool, project_id: &str, project_dir: &Path, dry_run: bool) -> Result<()> {
    println!("{}", "Generating all artifacts...".bold());

    // Agents
    let agents = cwa_codegen::generate_all_agents(pool, project_id).await?;
    if !agents.is_empty() {
        if dry_run {
            println!("  {} agents: {}", agents.len(), agents.iter().map(|a| a.filename.as_str()).collect::<Vec<_>>().join(", "));
        } else {
            let output_dir = project_dir.join(".claude/agents");
            let written = cwa_codegen::write_agents(&agents, &output_dir)?;
            println!("  {} {} agents", "✓".green(), written.len());
        }
    }

    // Skills
    let skills = cwa_codegen::generate_all_skills(pool, project_id).await?;
    if !skills.is_empty() {
        if dry_run {
            println!("  {} skills: {}", skills.len(), skills.iter().map(|s| s.dirname.as_str()).collect::<Vec<_>>().join(", "));
        } else {
            let output_dir = project_dir.join(".claude/skills");
            let written = cwa_codegen::write_skills(&skills, &output_dir)?;
            println!("  {} {} skills", "✓".green(), written.len());
        }
    }

    // Commands
    let commands = cwa_codegen::generate_all_commands();
    if dry_run {
        println!("  {} commands: {}", commands.len(), commands.iter().map(|c| c.name.as_str()).collect::<Vec<_>>().join(", "));
    } else {
        let output_dir = project_dir.join(".claude/commands");
        let written = cwa_codegen::write_commands(&commands, &output_dir)?;
        println!("  {} {} commands", "✓".green(), written.len());
    }

    // Hooks
    let hooks = cwa_codegen::generate_hooks(pool, project_id).await?;
    if hooks.hook_count > 0 {
        if dry_run {
            println!("  {} hooks", hooks.hook_count);
        } else {
            cwa_codegen::write_hooks(&hooks, project_dir)?;
            println!("  {} {} hooks", "✓".green(), hooks.hook_count);
        }
    }

    // CLAUDE.md
    let claude_md = cwa_codegen::generate_claude_md(pool, project_id).await?;
    if dry_run {
        println!("  CLAUDE.md");
    } else {
        cwa_codegen::write_claude_md(&claude_md, project_dir)?;
        println!("  {} CLAUDE.md", "✓".green());
    }

    if dry_run {
        println!("\n{}", "(dry run - no files written)".dimmed());
    } else {
        println!("\n{}", "All artifacts generated.".green().bold());
    }

    Ok(())
}
