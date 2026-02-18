//! Observation sync to Neo4j as reasoning memory nodes.
//!
//! Creates Observation nodes that represent development insights,
//! decisions in-the-making, and discovered facts:
//!
//! - (:Observation)-[:RECORDED_IN]->(:Project)
//! - (:Observation)-[:MODIFIES]->(:File)        (for files_modified)
//! - (:Observation)-[:READS]->(:File)            (for files_read)

use anyhow::Result;
use neo4rs::Query;
use tracing::debug;

use cwa_db::DbPool;
use crate::GraphClient;
use super::SyncResult;

/// Sync observations from Redis to Neo4j as reasoning memory nodes.
pub async fn sync_observations(
    client: &GraphClient,
    db: &DbPool,
    project_id: &str,
) -> Result<SyncResult> {
    let mut result = SyncResult::default();

    let observations = cwa_db::queries::observations::list_observations_compact(
        db,
        project_id,
        0,
        200,
    )
    .await
    .map_err(|e| anyhow::anyhow!("Failed to list observations: {}", e))?;

    for obs in &observations {
        // Upsert Observation node
        let node_query = Query::new(
            "MERGE (o:Observation {id: $id})
             SET o.type = $obs_type,
                 o.title = $title,
                 o.confidence = $confidence,
                 o.created_at = $created_at"
                .to_string(),
        )
        .param("id", obs.id.as_str())
        .param("obs_type", obs.obs_type.as_str())
        .param("title", obs.title.as_str())
        .param("confidence", obs.confidence)
        .param("created_at", obs.created_at.as_str());

        client.execute(node_query).await?;
        result.nodes_created += 1;

        // RECORDED_IN Project
        let project_rel = Query::new(
            "MATCH (o:Observation {id: $obs_id}), (p:Project {id: $project_id})
             MERGE (o)-[:RECORDED_IN]->(p)"
                .to_string(),
        )
        .param("obs_id", obs.id.as_str())
        .param("project_id", project_id);

        client.execute(project_rel).await?;
        result.relationships_created += 1;

        debug!(
            obs_id = %obs.id,
            obs_type = %obs.obs_type,
            confidence = obs.confidence,
            "Synced observation node"
        );
    }

    // For full observations (with files), fetch details individually
    // We limit to recent high-confidence ones to avoid overloading the graph
    let full_obs = cwa_db::queries::observations::list_high_confidence(
        db,
        project_id,
        0.7,
        50,
    )
    .await
    .map_err(|e| anyhow::anyhow!("Failed to list high-confidence observations: {}", e))?;

    for obs in &full_obs {
        // Link files modified
        if let Some(ref files_json) = obs.files_modified {
            if let Ok(files) = serde_json::from_str::<Vec<String>>(files_json) {
                for file_path in &files {
                    let file_query = Query::new(
                        "MERGE (f:File {path: $path})
                         WITH f
                         MATCH (o:Observation {id: $obs_id})
                         MERGE (o)-[:MODIFIES]->(f)"
                            .to_string(),
                    )
                    .param("path", file_path.as_str())
                    .param("obs_id", obs.id.as_str());

                    client.execute(file_query).await?;
                    result.relationships_created += 1;
                }
            }
        }

        // Link files read
        if let Some(ref files_json) = obs.files_read {
            if let Ok(files) = serde_json::from_str::<Vec<String>>(files_json) {
                for file_path in &files {
                    let file_query = Query::new(
                        "MERGE (f:File {path: $path})
                         WITH f
                         MATCH (o:Observation {id: $obs_id})
                         MERGE (o)-[:READS]->(f)"
                            .to_string(),
                    )
                    .param("path", file_path.as_str())
                    .param("obs_id", obs.id.as_str());

                    client.execute(file_query).await?;
                    result.relationships_created += 1;
                }
            }
        }
    }

    Ok(result)
}
