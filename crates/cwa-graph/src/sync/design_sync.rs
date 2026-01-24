//! Design system sync to Neo4j.
//!
//! Syncs design system entries from SQLite to Neo4j as DesignSystem nodes.

use anyhow::Result;
use neo4rs::Query;
use tracing::debug;

use cwa_db::DbPool;
use crate::GraphClient;
use super::SyncResult;

/// Sync all design systems for a project to Neo4j.
pub async fn sync_design_systems(client: &GraphClient, db: &DbPool, project_id: &str) -> Result<SyncResult> {
    let design_systems = cwa_db::queries::design_systems::list_design_systems(db, project_id)
        .map_err(|e| anyhow::anyhow!("Failed to list design systems: {}", e))?;

    let mut result = SyncResult::default();

    for ds in &design_systems {
        // Count colors from JSON
        let colors_count = ds.colors_json.as_deref()
            .and_then(|j| serde_json::from_str::<serde_json::Value>(j).ok())
            .map(|v| {
                let primary = v["primary"].as_array().map(|a| a.len()).unwrap_or(0);
                let secondary = v["secondary"].as_array().map(|a| a.len()).unwrap_or(0);
                let neutral = v["neutral"].as_array().map(|a| a.len()).unwrap_or(0);
                (primary + secondary + neutral) as i64
            })
            .unwrap_or(0);

        // Get typography families
        let typography_families = ds.typography_json.as_deref()
            .and_then(|j| serde_json::from_str::<serde_json::Value>(j).ok())
            .and_then(|v| v["font_families"].as_array().map(|arr| {
                arr.iter()
                    .filter_map(|f| f["name"].as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            }))
            .unwrap_or_default();

        // Count components
        let components_count = ds.components_json.as_deref()
            .and_then(|j| serde_json::from_str::<Vec<serde_json::Value>>(j).ok())
            .map(|v| v.len() as i64)
            .unwrap_or(0);

        // MERGE the DesignSystem node
        let query = Query::new(
            "MERGE (ds:DesignSystem {id: $id})
             SET ds.source_url = $source_url,
                 ds.colors_count = $colors_count,
                 ds.typography_families = $typography_families,
                 ds.components_count = $components_count,
                 ds.created_at = $created_at,
                 ds.updated_at = $updated_at"
                .to_string(),
        )
        .param("id", ds.id.as_str())
        .param("source_url", ds.source_url.as_str())
        .param("colors_count", colors_count)
        .param("typography_families", typography_families.as_str())
        .param("components_count", components_count)
        .param("created_at", ds.created_at.as_str())
        .param("updated_at", ds.updated_at.as_str());

        client.execute(query).await?;
        result.nodes_created += 1;

        debug!(id = %ds.id, "Synced DesignSystem node");

        // Create BELONGS_TO relationship
        let rel_query = Query::new(
            "MATCH (ds:DesignSystem {id: $ds_id}), (p:Project {id: $project_id})
             MERGE (ds)-[:BELONGS_TO]->(p)"
                .to_string(),
        )
        .param("ds_id", ds.id.as_str())
        .param("project_id", project_id);

        client.execute(rel_query).await?;
        result.relationships_created += 1;
    }

    Ok(result)
}
