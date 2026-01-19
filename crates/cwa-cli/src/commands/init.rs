//! Project initialization command.

use anyhow::Result;
use clap::Args;
use colored::Colorize;
use std::path::PathBuf;

#[derive(Args)]
pub struct InitArgs {
    /// Project name
    pub name: String,

    /// Initialize from a natural language prompt
    #[arg(long)]
    pub from_prompt: Option<String>,

    /// Target directory (defaults to ./<name>)
    #[arg(short, long)]
    pub directory: Option<PathBuf>,
}

pub async fn execute(args: InitArgs) -> Result<()> {
    let target_dir = args
        .directory
        .unwrap_or_else(|| PathBuf::from(&args.name));

    println!(
        "{} Creating project: {}",
        "→".blue().bold(),
        args.name.cyan()
    );

    // Create project structure
    cwa_core::project::scaffold::create_project(&target_dir, &args.name).await?;

    // If from-prompt provided, generate initial spec
    if let Some(prompt) = args.from_prompt {
        println!("{} Generating from prompt...", "→".blue().bold());
        cwa_core::project::scaffold::generate_from_prompt(&target_dir, &prompt).await?;
    }

    println!();
    println!("{} Project created: {}", "✓".green().bold(), args.name.cyan());
    println!("  Directory: {}", target_dir.display());
    println!();
    println!("{}", "Next steps:".bold());
    println!("  cd {}", target_dir.display());
    println!("  cwa spec new <feature>    # Create your first specification");
    println!("  cwa domain discover       # Discover domain concepts");
    println!("  cwa serve                 # Start web dashboard");

    Ok(())
}
