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

    /// Record a structured observation
    Observe(ObserveArgs),

    /// View observations timeline
    Timeline(TimelineArgs),

    /// Generate a summary from recent observations
    Summarize(SummarizeArgs),
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

    /// Decay factor to apply to all observation confidences (e.g., 0.98)
    #[arg(long)]
    pub decay: Option<f64>,
}

#[derive(Args)]
pub struct ExportArgs {
    /// Output file
    #[arg(short, long)]
    pub output: Option<String>,
}

#[derive(Args)]
pub struct ObserveArgs {
    /// Observation title
    pub title: String,

    /// Observation type (bugfix, feature, refactor, discovery, decision, change, insight)
    #[arg(long, short = 't', default_value = "discovery")]
    pub obs_type: String,

    /// Narrative description
    #[arg(long, short = 'n')]
    pub narrative: Option<String>,

    /// Facts learned (can be repeated)
    #[arg(long, short = 'f')]
    pub fact: Vec<String>,

    /// Concepts (can be repeated: how-it-works, why-it-exists, what-changed, problem-solution, gotcha, pattern, trade-off)
    #[arg(long, short = 'c')]
    pub concept: Vec<String>,

    /// Files modified (can be repeated)
    #[arg(long)]
    pub files_modified: Vec<String>,

    /// Files read (can be repeated)
    #[arg(long)]
    pub files_read: Vec<String>,

    /// Initial confidence (0.0 - 1.0)
    #[arg(long, default_value = "0.8")]
    pub confidence: f64,
}

#[derive(Args)]
pub struct TimelineArgs {
    /// Number of days back to look
    #[arg(long, default_value = "7")]
    pub days: i64,

    /// Maximum number of entries
    #[arg(long, default_value = "20")]
    pub limit: i64,
}

#[derive(Args)]
pub struct SummarizeArgs {
    /// Number of recent observations to summarize
    #[arg(long, default_value = "10")]
    pub count: i64,
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
        MemoryCommands::Observe(args) => cmd_observe(&pool, &project.id, args).await,
        MemoryCommands::Timeline(args) => cmd_timeline(&pool, &project.id, args),
        MemoryCommands::Summarize(args) => cmd_summarize(&pool, &project.id, args),
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
    // Apply decay if specified
    if let Some(factor) = args.decay {
        let decayed = cwa_core::memory::decay_confidence(pool, project_id, factor)?;
        println!(
            "{} Decayed {} observation confidences by factor {}",
            "✓".green().bold(),
            decayed,
            factor
        );
    }

    // Remove low-confidence observations
    let removed_obs = cwa_core::memory::remove_low_confidence_observations(
        pool, project_id, args.min_confidence,
    )?;
    if !removed_obs.is_empty() {
        println!(
            "{} Removed {} low-confidence observations",
            "✓".green().bold(),
            removed_obs.len()
        );
    }

    // Also compact memories (existing behavior)
    println!(
        "{} Compacting memories (min_confidence: {})...",
        "→".dimmed(),
        args.min_confidence
    );

    match cwa_embedding::MemoryPipeline::default_pipeline() {
        Ok(pipeline) => {
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
        }
        Err(_) => {
            println!("{} Memory compaction skipped (Qdrant/Ollama unavailable)", "!".yellow());
        }
    }

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

/// Record a structured observation.
async fn cmd_observe(pool: &cwa_db::DbPool, project_id: &str, args: ObserveArgs) -> Result<()> {
    // Validate observation type
    cwa_core::memory::observation::ObservationType::from_str(&args.obs_type)
        .ok_or_else(|| anyhow::anyhow!(
            "Invalid observation type: '{}'. Use: {}",
            args.obs_type,
            cwa_core::memory::observation::ObservationType::all_variants().join(", ")
        ))?;

    println!("{} Recording observation...", "→".dimmed());

    // Try embedding pipeline first (optional - fallback to DB-only)
    match cwa_embedding::ObservationPipeline::default_pipeline() {
        Ok(pipeline) => {
            let result = pipeline.add_observation(
                pool, project_id, &args.obs_type, &args.title,
                args.narrative.as_deref(), &args.fact, &args.concept,
                &args.files_modified, &args.files_read,
                None, args.confidence,
            ).await?;

            println!(
                "{} Observation recorded (id: {}, embedding: {} dims)",
                "✓".green().bold(),
                result.id[..8].dimmed(),
                result.embedding_dim
            );
        }
        Err(_) => {
            // Fallback: store in DB without embedding
            let obs = cwa_core::memory::add_observation(
                pool, project_id, &args.obs_type, &args.title,
                args.narrative.as_deref(), &args.fact, &args.concept,
                &args.files_modified, &args.files_read,
                None, args.confidence,
            )?;

            println!(
                "{} Observation recorded (id: {}, no embedding)",
                "✓".green().bold(),
                obs.id[..8].dimmed(),
            );
            println!("{} Embedding skipped (Qdrant/Ollama unavailable)", "!".yellow());
        }
    }

    // Print summary
    println!("  {} [{}] {}", "•".dimmed(), args.obs_type.cyan(), args.title);
    if !args.fact.is_empty() {
        for fact in &args.fact {
            println!("    {} {}", "→".dimmed(), fact);
        }
    }

    Ok(())
}

/// View observations timeline.
fn cmd_timeline(pool: &cwa_db::DbPool, project_id: &str, args: TimelineArgs) -> Result<()> {
    let observations = cwa_core::memory::get_timeline(pool, project_id, args.days, args.limit)?;

    if observations.is_empty() {
        println!("{}", "No observations found.".dimmed());
        return Ok(());
    }

    println!("{} Observations (last {} days):\n", "→".dimmed(), args.days);

    let mut current_date = String::new();
    for obs in &observations {
        // Group by date
        let date = obs.created_at.split('T').next()
            .unwrap_or(&obs.created_at)
            .split(' ').next()
            .unwrap_or(&obs.created_at);

        if date != current_date {
            if !current_date.is_empty() {
                println!();
            }
            println!("  {}", date.bold());
            current_date = date.to_string();
        }

        let type_label = format!("[{}]", obs.obs_type.to_uppercase());
        let confidence_pct = (obs.confidence * 100.0) as u32;
        let conf_str = format!("{}%", confidence_pct);
        let conf_colored = if confidence_pct >= 80 {
            conf_str.green()
        } else if confidence_pct >= 50 {
            conf_str.yellow()
        } else {
            conf_str.red()
        };

        println!(
            "    {} {} {} ({})",
            type_label.cyan(),
            obs.title,
            conf_colored.dimmed(),
            obs.id[..8].dimmed()
        );
    }

    println!();
    Ok(())
}

/// Generate a summary from recent observations.
fn cmd_summarize(pool: &cwa_db::DbPool, project_id: &str, args: SummarizeArgs) -> Result<()> {
    let observations = cwa_core::memory::get_timeline(pool, project_id, 30, args.count)?;

    if observations.is_empty() {
        println!("{}", "No observations to summarize.".dimmed());
        return Ok(());
    }

    // Collect all facts from the observations (fetch full details)
    let ids: Vec<&str> = observations.iter().map(|o| o.id.as_str()).collect();
    let full_observations = cwa_core::memory::get_observations_batch(pool, &ids)?;

    let mut all_facts: Vec<String> = Vec::new();
    let mut summary_parts: Vec<String> = Vec::new();

    for obs in &full_observations {
        summary_parts.push(format!("[{}] {}", obs.obs_type.to_uppercase(), obs.title));
        all_facts.extend(obs.facts.clone());
    }

    let content = summary_parts.join(". ");

    // Create the summary
    let summary = cwa_core::memory::create_summary(
        pool, project_id, None, &content, &all_facts, full_observations.len() as i64,
    )?;

    println!("{} Summary created (id: {})", "✓".green().bold(), summary.id[..8].dimmed());
    println!("  {} {} observations summarized", "•".dimmed(), full_observations.len());
    if !all_facts.is_empty() {
        println!("  {} {} key facts extracted", "•".dimmed(), all_facts.len());
    }
    println!("\n{}", content.dimmed());

    Ok(())
}
