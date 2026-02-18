//! Decision (ADR) synchronization to Neo4j.
//!
//! Creates nodes and relationships:
//! - (:Decision)-[:BELONGS_TO]->(:Project)
//! - (:Decision)-[:RELATES_TO]->(:Spec)
//! - (:Decision)-[:SUPERSEDED_BY]->(:Decision)

use anyhow::Result;
use neo4rs::Query;
use tracing::debug;

use cwa_db::DbPool;
use crate::GraphClient;
use super::SyncResult;

/// Sync all decisions for a project to Neo4j.
pub async fn sync_decisions(client: &GraphClient, db: &DbPool, project_id: &str) -> Result<SyncResult> {
    let decisions = cwa_db::queries::decisions::list_decisions(db, project_id).await
        .map_err(|e| anyhow::anyhow!("Failed to list decisions: {}", e))?;

    let mut result = SyncResult::default();

    for decision in &decisions {
        // MERGE the Decision node
        let query = Query::new(
            "MERGE (d:Decision {id: $id})
             SET d.title = $title,
                 d.status = $status,
                 d.context = $context,
                 d.decision = $decision,
                 d.consequences = $consequences,
                 d.alternatives = $alternatives,
                 d.created_at = $created_at,
                 d.updated_at = $updated_at"
                .to_string(),
        )
        .param("id", decision.id.as_str())
        .param("title", decision.title.as_str())
        .param("status", decision.status.as_str())
        .param("context", decision.context.as_str())
        .param("decision", decision.decision.as_str())
        .param("consequences", decision.consequences.as_deref().unwrap_or(""))
        .param("alternatives", decision.alternatives.as_deref().unwrap_or(""))
        .param("created_at", decision.created_at.as_str())
        .param("updated_at", decision.updated_at.as_str());

        client.execute(query).await?;
        result.nodes_created += 1;

        // BELONGS_TO Project
        let proj_rel = Query::new(
            "MATCH (d:Decision {id: $decision_id}), (p:Project {id: $project_id})
             MERGE (d)-[:BELONGS_TO]->(p)"
                .to_string(),
        )
        .param("decision_id", decision.id.as_str())
        .param("project_id", project_id);

        client.execute(proj_rel).await?;
        result.relationships_created += 1;

        // RELATES_TO Specs (stored as JSON array of spec IDs)
        if let Some(ref specs_json) = decision.related_specs {
            if let Ok(spec_ids) = serde_json::from_str::<Vec<String>>(specs_json) {
                for spec_id in &spec_ids {
                    let spec_rel = Query::new(
                        "MATCH (d:Decision {id: $decision_id}), (s:Spec {id: $spec_id})
                         MERGE (d)-[:RELATES_TO]->(s)"
                            .to_string(),
                    )
                    .param("decision_id", decision.id.as_str())
                    .param("spec_id", spec_id.as_str());

                    client.execute(spec_rel).await?;
                    result.relationships_created += 1;
                }
            }
        }

        // SUPERSEDED_BY relationship
        if let Some(ref superseded_by) = decision.superseded_by {
            let supersede_rel = Query::new(
                "MATCH (d:Decision {id: $decision_id}), (newer:Decision {id: $newer_id})
                 MERGE (d)-[:SUPERSEDED_BY]->(newer)"
                    .to_string(),
            )
            .param("decision_id", decision.id.as_str())
            .param("newer_id", superseded_by.as_str());

            client.execute(supersede_rel).await?;
            result.relationships_created += 1;
        }

        debug!(decision_id = %decision.id, title = %decision.title, "Synced decision");
    }

    Ok(result)
}
