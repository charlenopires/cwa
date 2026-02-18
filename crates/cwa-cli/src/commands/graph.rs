//! Knowledge Graph CLI commands.

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use std::path::Path;

#[derive(Subcommand)]
pub enum GraphCommands {
    /// Sync SQLite entities to Neo4j
    Sync,

    /// Execute a Cypher query
    Query {
        /// Cypher query string
        query: String,
    },

    /// Analyze impact of an entity
    Impact {
        /// Entity type (spec, task, context, decision)
        entity_type: String,
        /// Entity ID
        entity_id: String,
    },

    /// Explore graph neighborhood
    Explore {
        /// Entity type
        entity_type: String,
        /// Entity ID
        entity_id: String,
        /// Traversal depth
        #[arg(long, default_value = "2")]
        depth: u32,
    },

    /// Show graph status
    Status,
}

pub async fn execute(cmd: GraphCommands, project_dir: &Path) -> Result<()> {
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let pool = cwa_db::init_pool(&redis_url).await?;

    let project = cwa_core::project::get_default_project(&pool).await?
        .ok_or_else(|| anyhow::anyhow!("No project found. Run 'cwa init' first."))?;

    // Connect to Neo4j
    let graph_client = cwa_graph::GraphClient::connect_default().await?;

    match cmd {
        GraphCommands::Sync => cmd_sync(&graph_client, &pool, &project.id).await,
        GraphCommands::Query { query } => cmd_query(&graph_client, &query).await,
        GraphCommands::Impact { entity_type, entity_id } => {
            cmd_impact(&graph_client, &entity_type, &entity_id).await
        }
        GraphCommands::Explore { entity_type, entity_id, depth } => {
            cmd_explore(&graph_client, &entity_type, &entity_id, depth).await
        }
        GraphCommands::Status => cmd_status(&graph_client, &pool, &project.id).await,
    }
}

/// Run full sync from SQLite to Neo4j.
async fn cmd_sync(client: &cwa_graph::GraphClient, db: &cwa_db::DbPool, project_id: &str) -> Result<()> {
    println!("{}", "Syncing to Knowledge Graph...".bold());

    // Initialize schema first
    cwa_graph::schema::initialize_schema(client).await?;

    // Run the sync
    let result = cwa_graph::run_full_sync(client, db, project_id).await?;

    println!("\n{}", "Sync complete:".green().bold());
    println!("  Nodes created/updated: {}", result.nodes_created + result.nodes_updated);
    println!("  Relationships created: {}", result.relationships_created);

    Ok(())
}

/// Execute a raw Cypher query.
async fn cmd_query(client: &cwa_graph::GraphClient, cypher: &str) -> Result<()> {
    let results = cwa_graph::queries::search::raw_query(client, cypher).await?;

    if results.is_empty() {
        println!("{}", "No results.".dimmed());
    } else {
        for (i, result) in results.iter().enumerate() {
            println!("{}: {}", (i + 1).to_string().dimmed(), result);
        }
    }

    Ok(())
}

/// Show impact analysis for an entity.
async fn cmd_impact(client: &cwa_graph::GraphClient, entity_type: &str, entity_id: &str) -> Result<()> {
    println!("{} {} {}", "Impact analysis for".bold(), entity_type.cyan(), entity_id.yellow());
    println!("{}", "─".repeat(50));

    let nodes = cwa_graph::queries::impact::impact_analysis(client, entity_type, entity_id, 3).await?;

    if nodes.is_empty() {
        println!("{}", "No related entities found.".dimmed());
        return Ok(());
    }

    for node in &nodes {
        let label_colored = match node.label.as_str() {
            "Spec" => node.label.cyan(),
            "Task" => node.label.green(),
            "BoundedContext" => node.label.magenta(),
            "DomainEntity" => node.label.blue(),
            "Decision" => node.label.yellow(),
            "Term" => node.label.white(),
            _ => node.label.normal(),
        };
        println!(
            "  {} [{}] {} ({})",
            "→".dimmed(),
            label_colored,
            node.name,
            node.relationship.dimmed()
        );
    }

    println!("\n{} related entities found.", nodes.len().to_string().bold());

    Ok(())
}

/// Explore the neighborhood of an entity.
async fn cmd_explore(client: &cwa_graph::GraphClient, entity_type: &str, entity_id: &str, depth: u32) -> Result<()> {
    println!("{} {} {} (depth={})", "Exploring".bold(), entity_type.cyan(), entity_id.yellow(), depth);
    println!("{}", "─".repeat(50));

    let result = cwa_graph::queries::explore::explore_neighborhood(client, entity_type, entity_id, depth).await?;

    if let Some(center) = &result.center {
        println!("{} [{}] {}", "Center:".bold(), center.label.cyan(), center.name);
    } else {
        println!("{}", "Entity not found in graph.".red());
        return Ok(());
    }

    if !result.nodes.is_empty() {
        println!("\n{} ({}):", "Connected nodes".bold(), result.nodes.len());
        for node in &result.nodes {
            println!("  {} [{}] {}", "•".dimmed(), node.label.dimmed(), node.name);
        }
    }

    if !result.relationships.is_empty() {
        println!("\n{} ({}):", "Relationships".bold(), result.relationships.len());
        for rel in &result.relationships {
            println!(
                "  {} {} {} {}",
                rel.from_id.dimmed(),
                "-[".dimmed(),
                rel.rel_type.yellow(),
                format!("]-> {}", rel.to_id).dimmed()
            );
        }
    }

    Ok(())
}

/// Show graph status (node/relationship counts, last sync time).
async fn cmd_status(client: &cwa_graph::GraphClient, db: &cwa_db::DbPool, project_id: &str) -> Result<()> {
    println!("{}", "Knowledge Graph Status".bold());
    println!("{}", "─".repeat(40));

    let counts = client.get_counts().await?;
    println!("  Nodes:         {}", counts.nodes.to_string().cyan());
    println!("  Relationships: {}", counts.relationships.to_string().cyan());

    let last_sync = cwa_graph::get_last_sync_time(db, project_id)?;
    match last_sync {
        Some(time) => println!("  Last sync:     {}", time.green()),
        None => println!("  Last sync:     {}", "never".yellow()),
    }

    println!("{}", "─".repeat(40));

    Ok(())
}
