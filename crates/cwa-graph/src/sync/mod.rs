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
use rusqlite::params;
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

    // Update sync_state for all synced entities
    update_sync_state(db, project_id)?;

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
    let project = cwa_db::queries::projects::get_project(db, project_id)
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

/// Update sync_state table after a successful sync.
fn update_sync_state(db: &DbPool, project_id: &str) -> Result<()> {
    db.with_conn(|conn| {
        // Mark all entities for this project as synced
        // We update specs
        conn.execute(
            "INSERT OR REPLACE INTO sync_state (entity_type, entity_id, last_synced_at, sync_version)
             SELECT 'spec', id, datetime('now'), COALESCE(
                 (SELECT sync_version FROM sync_state WHERE entity_type = 'spec' AND entity_id = specs.id), 0
             ) + 1
             FROM specs WHERE project_id = ?1",
            params![project_id],
        )?;

        // Mark contexts
        conn.execute(
            "INSERT OR REPLACE INTO sync_state (entity_type, entity_id, last_synced_at, sync_version)
             SELECT 'context', id, datetime('now'), COALESCE(
                 (SELECT sync_version FROM sync_state WHERE entity_type = 'context' AND entity_id = bounded_contexts.id), 0
             ) + 1
             FROM bounded_contexts WHERE project_id = ?1",
            params![project_id],
        )?;

        // Mark tasks
        conn.execute(
            "INSERT OR REPLACE INTO sync_state (entity_type, entity_id, last_synced_at, sync_version)
             SELECT 'task', id, datetime('now'), COALESCE(
                 (SELECT sync_version FROM sync_state WHERE entity_type = 'task' AND entity_id = tasks.id), 0
             ) + 1
             FROM tasks WHERE project_id = ?1",
            params![project_id],
        )?;

        // Mark decisions
        conn.execute(
            "INSERT OR REPLACE INTO sync_state (entity_type, entity_id, last_synced_at, sync_version)
             SELECT 'decision', id, datetime('now'), COALESCE(
                 (SELECT sync_version FROM sync_state WHERE entity_type = 'decision' AND entity_id = decisions.id), 0
             ) + 1
             FROM decisions WHERE project_id = ?1",
            params![project_id],
        )?;

        Ok(())
    })
    .map_err(|e| anyhow::anyhow!("Failed to update sync state: {}", e))
}

/// Get the last sync timestamp for a project (any entity type).
pub fn get_last_sync_time(db: &DbPool, project_id: &str) -> Result<Option<String>> {
    db.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT MAX(ss.last_synced_at)
             FROM sync_state ss
             JOIN specs s ON ss.entity_type = 'spec' AND ss.entity_id = s.id AND s.project_id = ?1
             UNION ALL
             SELECT MAX(ss.last_synced_at)
             FROM sync_state ss
             JOIN tasks t ON ss.entity_type = 'task' AND ss.entity_id = t.id AND t.project_id = ?1"
        )?;

        let mut rows = stmt.query(params![project_id])?;
        let mut latest: Option<String> = None;
        while let Some(row) = rows.next()? {
            let val: Option<String> = row.get(0)?;
            if let Some(v) = val {
                latest = Some(match latest {
                    Some(ref l) if v > *l => v,
                    None => v,
                    Some(l) => l,
                });
            }
        }
        Ok(latest)
    })
    .map_err(|e| anyhow::anyhow!("Failed to get last sync time: {}", e))
}
