//! Spec synchronization to Neo4j.
//!
//! Syncs specs as (:Spec) nodes with relationships to (:Project).

use anyhow::Result;
use neo4rs::Query;
use tracing::debug;

use cwa_db::DbPool;
use crate::GraphClient;
use super::SyncResult;

/// Sync all specs for a project to Neo4j.
pub async fn sync_specs(client: &GraphClient, db: &DbPool, project_id: &str) -> Result<SyncResult> {
    let specs = cwa_db::queries::specs::list_specs(db, project_id).await
        .map_err(|e| anyhow::anyhow!("Failed to list specs: {}", e))?;

    let mut result = SyncResult::default();

    for spec in &specs {
        // MERGE the Spec node
        let query = Query::new(
            "MERGE (s:Spec {id: $id})
             SET s.title = $title,
                 s.description = $description,
                 s.status = $status,
                 s.priority = $priority,
                 s.acceptance_criteria = $acceptance_criteria,
                 s.created_at = $created_at,
                 s.updated_at = $updated_at"
                .to_string(),
        )
        .param("id", spec.id.as_str())
        .param("title", spec.title.as_str())
        .param("description", spec.description.as_deref().unwrap_or(""))
        .param("status", spec.status.as_str())
        .param("priority", spec.priority.as_str())
        .param("acceptance_criteria", spec.acceptance_criteria.as_deref().unwrap_or(""))
        .param("created_at", spec.created_at.as_str())
        .param("updated_at", spec.updated_at.as_str());

        client.execute(query).await?;
        result.nodes_created += 1;

        // Create BELONGS_TO relationship to Project
        let rel_query = Query::new(
            "MATCH (s:Spec {id: $spec_id}), (p:Project {id: $project_id})
             MERGE (s)-[:BELONGS_TO]->(p)"
                .to_string(),
        )
        .param("spec_id", spec.id.as_str())
        .param("project_id", project_id);

        client.execute(rel_query).await?;
        result.relationships_created += 1;

        // Handle spec dependencies (stored as JSON array of spec IDs)
        if let Some(ref deps_json) = spec.dependencies {
            if let Ok(dep_ids) = serde_json::from_str::<Vec<String>>(deps_json) {
                for dep_id in &dep_ids {
                    let dep_query = Query::new(
                        "MATCH (s:Spec {id: $spec_id}), (dep:Spec {id: $dep_id})
                         MERGE (s)-[:DEPENDS_ON]->(dep)"
                            .to_string(),
                    )
                    .param("spec_id", spec.id.as_str())
                    .param("dep_id", dep_id.as_str());

                    client.execute(dep_query).await?;
                    result.relationships_created += 1;
                }
            }
        }

        debug!(spec_id = %spec.id, title = %spec.title, "Synced spec");
    }

    Ok(result)
}
