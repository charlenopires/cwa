//! Design system CLI commands.

use anyhow::Result;
use clap::{Args, Subcommand};
use colored::Colorize;
use std::path::Path;

#[derive(Subcommand)]
pub enum DesignCommands {
    /// Extract a design system from a software screenshot via Claude Vision API
    FromImage(FromImageArgs),
}

#[derive(Args)]
pub struct FromImageArgs {
    /// URL of the image to analyze
    pub url: String,

    /// Claude model to use for vision analysis
    #[arg(long, default_value = "claude-sonnet-4-20250514")]
    pub model: String,

    /// Preview analysis without storing
    #[arg(long)]
    pub dry_run: bool,
}

pub async fn execute(cmd: DesignCommands, project_dir: &Path) -> Result<()> {
    match cmd {
        DesignCommands::FromImage(args) => cmd_from_image(args, project_dir).await,
    }
}

async fn cmd_from_image(args: FromImageArgs, project_dir: &Path) -> Result<()> {
    let db_path = project_dir.join(".cwa/cwa.db");
    let pool = cwa_db::init_pool(&db_path)?;

    let project = cwa_core::project::get_default_project(&pool)?
        .ok_or_else(|| anyhow::anyhow!("No project found. Run 'cwa init' first."))?;

    // 1. Get API key from environment
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| anyhow::anyhow!(
            "ANTHROPIC_API_KEY environment variable not set.\n\
             Set it with: export ANTHROPIC_API_KEY=your-key"
        ))?;

    // 2. Call Claude Vision API
    println!("{} Analyzing image: {}", "→".dimmed(), args.url);
    let client = cwa_core::design::vision::ClaudeVisionClient::new(&api_key, &args.model);
    let mut design_system = client.analyze_image(&args.url).await?;

    // Fill in metadata
    design_system.id = uuid::Uuid::new_v4().to_string();
    design_system.project_id = project.id.clone();
    design_system.source_url = args.url.clone();

    if args.dry_run {
        println!("\n{} Design system extracted (dry run):\n", "→".dimmed());
        print_design_summary(&design_system);
        println!("\n{}", "(dry run - no files written)".dimmed());
        return Ok(());
    }

    // 3. Store in SQLite
    cwa_core::design::store_design_system(&pool, &design_system)?;
    println!("{} Stored design system (id: {})", "✓".green().bold(), &design_system.id[..8]);

    // 4. Generate .claude/design-system.md
    let generated = cwa_codegen::generate_design_system_md(&pool, &project.id)?;
    if let Some(gen) = generated {
        let path = cwa_codegen::write_design_system_md(&gen, project_dir)?;
        println!("{} Generated: {}", "✓".green().bold(), path);
    }

    // 5. Store as semantic memory (optional, non-fatal)
    match cwa_embedding::MemoryPipeline::default_pipeline() {
        Ok(pipeline) => {
            let summary = format_design_for_embedding(&design_system);
            let result = pipeline.add_memory(
                &pool,
                &project.id,
                &summary,
                cwa_embedding::MemoryType::DesignSystem,
                Some("design-system"),
            ).await;
            match result {
                Ok(r) => println!("{} Stored embedding ({} dims)", "✓".green().bold(), r.embedding_dim),
                Err(e) => println!("{} Embedding skipped (Qdrant/Ollama unavailable): {}", "!".yellow(), e),
            }
        }
        Err(_) => {
            println!("{} Embedding skipped (Qdrant unavailable)", "!".yellow());
        }
    }

    // 6. Sync to Neo4j (optional, non-fatal)
    match cwa_graph::GraphClient::connect_default().await {
        Ok(graph_client) => {
            let sync_result = cwa_graph::sync::design_sync::sync_design_systems(
                &graph_client, &pool, &project.id
            ).await;
            match sync_result {
                Ok(r) => println!("{} Graph synced ({} nodes, {} relationships)",
                    "✓".green().bold(), r.nodes_created, r.relationships_created),
                Err(e) => println!("{} Graph sync skipped: {}", "!".yellow(), e),
            }
        }
        Err(_) => {
            println!("{} Graph sync skipped (Neo4j unavailable)", "!".yellow());
        }
    }

    println!("\n{}", "Design system ready.".green().bold());
    println!("  Reference: {}", ".claude/design-system.md".bold());
    Ok(())
}

/// Print a summary of the extracted design system.
fn print_design_summary(ds: &cwa_core::design::model::DesignSystem) {
    // Colors
    let total_colors = ds.colors_count();
    println!("  {} Colors: {} total", "•".dimmed(), total_colors);
    println!("    Primary: {}", ds.colors.primary.len());
    println!("    Secondary: {}", ds.colors.secondary.len());
    println!("    Neutral: {}", ds.colors.neutral.len());

    if let Some(ref hex) = ds.colors.semantic.success {
        println!("    Semantic: success={}", hex);
    }
    if let Some(ref hex) = ds.colors.semantic.error {
        println!("    Semantic: error={}", hex);
    }

    // Typography
    println!("  {} Typography:", "•".dimmed());
    for family in &ds.typography.font_families {
        println!("    {} ({})", family.name, family.category);
    }
    if !ds.typography.scale.is_empty() {
        println!("    Scale: {} steps", ds.typography.scale.len());
    }

    // Spacing
    if !ds.spacing.is_empty() {
        println!("  {} Spacing: {} tokens", "•".dimmed(), ds.spacing.len());
    }

    // Border Radius
    if !ds.border_radius.is_empty() {
        println!("  {} Border Radius: {} tokens", "•".dimmed(), ds.border_radius.len());
    }

    // Shadows
    if !ds.shadows.is_empty() {
        println!("  {} Shadows: {} tokens", "•".dimmed(), ds.shadows.len());
    }

    // Breakpoints
    if !ds.breakpoints.is_empty() {
        println!("  {} Breakpoints: {} defined", "•".dimmed(), ds.breakpoints.len());
    }

    // Components
    if !ds.components.is_empty() {
        println!("  {} Components: {}", "•".dimmed(), ds.components.len());
        for comp in &ds.components {
            println!("    - {}", comp.name);
        }
    }
}

/// Format the design system as a text summary for semantic embedding.
fn format_design_for_embedding(ds: &cwa_core::design::model::DesignSystem) -> String {
    let mut parts = Vec::new();

    parts.push(format!("Design System extracted from: {}", ds.source_url));

    // Colors
    if !ds.colors.primary.is_empty() {
        let colors: Vec<String> = ds.colors.primary.iter()
            .map(|c| format!("{}: {}", c.name, c.hex))
            .collect();
        parts.push(format!("Primary colors: {}", colors.join(", ")));
    }
    if !ds.colors.secondary.is_empty() {
        let colors: Vec<String> = ds.colors.secondary.iter()
            .map(|c| format!("{}: {}", c.name, c.hex))
            .collect();
        parts.push(format!("Secondary colors: {}", colors.join(", ")));
    }
    if !ds.colors.neutral.is_empty() {
        let colors: Vec<String> = ds.colors.neutral.iter()
            .map(|c| format!("{}: {}", c.name, c.hex))
            .collect();
        parts.push(format!("Neutral colors: {}", colors.join(", ")));
    }

    // Typography
    if !ds.typography.font_families.is_empty() {
        let families: Vec<String> = ds.typography.font_families.iter()
            .map(|f| format!("{} ({})", f.name, f.category))
            .collect();
        parts.push(format!("Font families: {}", families.join(", ")));
    }

    // Spacing
    if !ds.spacing.is_empty() {
        let tokens: Vec<String> = ds.spacing.iter()
            .map(|s| format!("{}: {}px", s.name, s.value_px))
            .collect();
        parts.push(format!("Spacing: {}", tokens.join(", ")));
    }

    // Components
    if !ds.components.is_empty() {
        let names: Vec<&str> = ds.components.iter()
            .map(|c| c.name.as_str())
            .collect();
        parts.push(format!("Components: {}", names.join(", ")));
    }

    parts.join(". ")
}
