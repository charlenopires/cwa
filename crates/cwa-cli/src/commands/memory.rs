//! Memory management commands.

use anyhow::Result;
use clap::{Args, Subcommand};
use colored::Colorize;
use std::path::Path;

#[derive(Subcommand)]
pub enum MemoryCommands {
    /// Add a memory entry with embedding
    Add(AddArgs),

    /// Semantic search across memories
    Search(SearchArgs),

    /// Import legacy memory entries with embeddings
    Import,

    /// Compact memories (remove low-confidence entries)
    Compact(CompactArgs),

    /// Sync memory with CLAUDE.md
    Sync,

    /// Export memory for new session
    Export(ExportArgs),
}

#[derive(Args)]
pub struct AddArgs {
    /// Memory content
    pub content: String,

    /// Entry type (preference, decision, fact, pattern)
    #[arg(long, short = 't', default_value = "fact")]
    pub entry_type: String,

    /// Context for the memory
    #[arg(long, short)]
    pub context: Option<String>,
}

#[derive(Args)]
pub struct SearchArgs {
    /// Search query
    pub query: String,

    /// Number of results to return
    #[arg(long, default_value = "5")]
    pub top_k: u64,

    /// Use legacy text search instead of semantic search
    #[arg(long)]
    pub legacy: bool,
}

#[derive(Args)]
pub struct CompactArgs {
    /// Minimum confidence threshold (entries below this are removed)
    #[arg(long, default_value = "0.3")]
    pub min_confidence: f64,

    /// Maximum number of entries to remove
    #[arg(long)]
    pub keep_top: Option<usize>,
}

#[derive(Args)]
pub struct ExportArgs {
    /// Output file
    #[arg(short, long)]
    pub output: Option<String>,
}

pub async fn execute(cmd: MemoryCommands, project_dir: &Path) -> Result<()> {
    let db_path = project_dir.join(".cwa/cwa.db");
    let pool = cwa_db::init_pool(&db_path)?;

    let project = cwa_core::project::get_default_project(&pool)?
        .ok_or_else(|| anyhow::anyhow!("No project found. Run 'cwa init' first."))?;

    match cmd {
        MemoryCommands::Add(args) => cmd_add(&pool, &project.id, args).await,
        MemoryCommands::Search(args) => cmd_search(&pool, &project.id, args).await,
        MemoryCommands::Import => cmd_import(&pool, &project.id).await,
        MemoryCommands::Compact(args) => cmd_compact(&pool, &project.id, args).await,
        MemoryCommands::Sync => cmd_sync(&pool, &project.id, project_dir),
        MemoryCommands::Export(args) => cmd_export(&pool, &project.id, args),
    }
}

/// Add a memory with embedding.
async fn cmd_add(pool: &cwa_db::DbPool, project_id: &str, args: AddArgs) -> Result<()> {
    let entry_type = cwa_embedding::MemoryType::from_str(&args.entry_type)?;

    println!("{}", "Adding memory...".dimmed());

    let pipeline = cwa_embedding::MemoryPipeline::default_pipeline()?;
    let result = pipeline.add_memory(
        pool,
        project_id,
        &args.content,
        entry_type,
        args.context.as_deref(),
    ).await?;

    println!(
        "{} Memory added (id: {}, embedding: {} dims)",
        "✓".green().bold(),
        result.id[..8].dimmed(),
        result.embedding_dim
    );

    Ok(())
}

/// Semantic search across memories.
async fn cmd_search(pool: &cwa_db::DbPool, project_id: &str, args: SearchArgs) -> Result<()> {
    if args.legacy {
        // Use the existing text-based search
        let results = cwa_core::memory::search_memory(pool, project_id, &args.query)?;

        if results.is_empty() {
            println!("{}", "No results found.".dimmed());
        } else {
            println!("{} Found {} entries:", "✓".green().bold(), results.len());
            for entry in results {
                println!("  [{}] {}", entry.entry_type.dimmed(), entry.content);
            }
        }
        return Ok(());
    }

    // Semantic search via embeddings
    println!("{}", "Searching...".dimmed());

    let search = cwa_embedding::SemanticSearch::default_search()?;
    let results = search.search_project(&args.query, project_id, args.top_k).await?;

    if results.is_empty() {
        println!("{}", "No results found.".dimmed());
        return Ok(());
    }

    println!("{} Found {} results:\n", "✓".green().bold(), results.len());

    for (i, result) in results.iter().enumerate() {
        let score_pct = (result.score * 100.0) as u32;
        let score_color = if score_pct > 80 {
            format!("{}%", score_pct).green()
        } else if score_pct > 50 {
            format!("{}%", score_pct).yellow()
        } else {
            format!("{}%", score_pct).red()
        };

        println!(
            "  {}. [{}] {} ({})",
            (i + 1).to_string().bold(),
            result.entry_type.cyan(),
            result.content,
            score_color,
        );

        if !result.context.is_empty() {
            println!("     {}", format!("context: {}", result.context).dimmed());
        }
    }

    Ok(())
}

/// Import legacy memory entries.
async fn cmd_import(pool: &cwa_db::DbPool, project_id: &str) -> Result<()> {
    println!("{}", "Importing legacy memories...".bold());

    let pipeline = cwa_embedding::MemoryPipeline::default_pipeline()?;
    let count = pipeline.import_legacy_memories(pool, project_id).await?;

    println!(
        "{} Imported {} memories with embeddings",
        "✓".green().bold(),
        count
    );

    Ok(())
}

/// Compact memories by removing low-confidence entries.
async fn cmd_compact(pool: &cwa_db::DbPool, project_id: &str, args: CompactArgs) -> Result<()> {
    println!(
        "{} Compacting memories (min_confidence: {})...",
        "→".dimmed(),
        args.min_confidence
    );

    let pipeline = cwa_embedding::MemoryPipeline::default_pipeline()?;
    let removed = pipeline.compact_memories(
        pool,
        project_id,
        args.min_confidence,
        args.keep_top,
    ).await?;

    println!(
        "{} Removed {} low-confidence memories",
        "✓".green().bold(),
        removed
    );

    Ok(())
}

/// Sync memory with CLAUDE.md.
fn cmd_sync(pool: &cwa_db::DbPool, project_id: &str, project_dir: &Path) -> Result<()> {
    let summary = cwa_core::memory::get_context_summary(pool, project_id)?;
    let content = summary.to_compact_string();

    let claude_md_path = project_dir.join("CLAUDE.md");
    std::fs::write(&claude_md_path, content)?;

    println!("{} Synced CLAUDE.md", "✓".green().bold());
    Ok(())
}

/// Export memory entries.
fn cmd_export(pool: &cwa_db::DbPool, project_id: &str, args: ExportArgs) -> Result<()> {
    let entries = cwa_core::memory::list_memory(pool, project_id, Some(100))?;
    let json = serde_json::to_string_pretty(&entries)?;

    if let Some(output) = args.output {
        std::fs::write(&output, &json)?;
        println!("{} Exported to {}", "✓".green().bold(), output);
    } else {
        println!("{}", json);
    }

    Ok(())
}
