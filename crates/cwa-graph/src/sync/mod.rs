//! SQLite to Neo4j synchronization pipeline.
//!
//! Reads entities from SQLite, syncs them to Neo4j as nodes/relationships,
//! and tracks sync state to avoid redundant writes.

pub mod spec_sync;
pub mod domain_sync;
pub mod kanban_sync;
pub mod decision_sync;
pub mod design_sync;

use anyhow::{Context, Result};
use neo4rs::Query;
use tracing::info;

use cwa_db::DbPool;
use crate::GraphClient;

/// Result of a sync operation.
#[derive(Debug, Clone, Default)]
pub struct SyncResult {
    pub nodes_created: usize,
    pub nodes_updated: usize,
    pub relationships_created: usize,
}

impl SyncResult {
    fn merge(&mut self, other: &SyncResult) {
        self.nodes_created += other.nodes_created;
        self.nodes_updated += other.nodes_updated;
        self.relationships_created += other.relationships_created;
    }
}

/// Run full sync from SQLite to Neo4j for a given project.
pub async fn run_full_sync(client: &GraphClient, db: &DbPool, project_id: &str) -> Result<SyncResult> {
    info!(project_id, "Starting full graph sync");

    let mut total = SyncResult::default();

    // Sync project node first
    sync_project_node(client, db, project_id).await?;
    total.nodes_created += 1;

    // Sync specs
    let spec_result = spec_sync::sync_specs(client, db, project_id).await
        .context("Failed to sync specs")?;
    info!(nodes = spec_result.nodes_created + spec_result.nodes_updated, rels = spec_result.relationships_created, "Specs synced");
    total.merge(&spec_result);

    // Sync domain model (contexts + objects + terms)
    let domain_result = domain_sync::sync_domain(client, db, project_id).await
        .context("Failed to sync domain model")?;
    info!(nodes = domain_result.nodes_created + domain_result.nodes_updated, rels = domain_result.relationships_created, "Domain synced");
    total.merge(&domain_result);

    // Sync tasks
    let task_result = kanban_sync::sync_tasks(client, db, project_id).await
        .context("Failed to sync tasks")?;
    info!(nodes = task_result.nodes_created + task_result.nodes_updated, rels = task_result.relationships_created, "Tasks synced");
    total.merge(&task_result);

    // Sync decisions
    let decision_result = decision_sync::sync_decisions(client, db, project_id).await
        .context("Failed to sync decisions")?;
    info!(nodes = decision_result.nodes_created + decision_result.nodes_updated, rels = decision_result.relationships_created, "Decisions synced");
    total.merge(&decision_result);

    // Sync design systems
    let design_result = design_sync::sync_design_systems(client, db, project_id).await
        .context("Failed to sync design systems")?;
    info!(nodes = design_result.nodes_created + design_result.nodes_updated, rels = design_result.relationships_created, "Design systems synced");
    total.merge(&design_result);

    // Sync state tracking not needed with Redis backend

    info!(
        nodes_created = total.nodes_created,
        nodes_updated = total.nodes_updated,
        relationships = total.relationships_created,
        "Full sync complete"
    );

    Ok(total)
}

/// Create/update the Project node in Neo4j.
async fn sync_project_node(client: &GraphClient, db: &DbPool, project_id: &str) -> Result<()> {
    let project = cwa_db::queries::projects::get_project(db, project_id).await
        .map_err(|e| anyhow::anyhow!("Failed to get project: {}", e))?;

    let query = Query::new(
        "MERGE (p:Project {id: $id})
         SET p.name = $name,
             p.description = $description,
             p.status = $status,
             p.updated_at = $updated_at"
            .to_string(),
    )
    .param("id", project.id.as_str())
    .param("name", project.name.as_str())
    .param("description", project.description.as_deref().unwrap_or(""))
    .param("status", project.status.as_str())
    .param("updated_at", project.updated_at.as_str());

    client.execute(query).await?;
    Ok(())
}

/// Update sync_state after a successful sync (no-op with Redis backend).
fn update_sync_state(_db: &DbPool, _project_id: &str) -> Result<()> {
    Ok(())
}

/// Get the last sync timestamp for a project (no-op with Redis backend).
pub fn get_last_sync_time(_db: &DbPool, _project_id: &str) -> Result<Option<String>> {
    Ok(None)
}
