//! Domain (BoundedContext, DomainObject, GlossaryTerm) synchronization to Neo4j.
//!
//! Creates nodes and relationships representing the domain model:
//! - (:BoundedContext)-[:BELONGS_TO]->(:Project)
//! - (:DomainEntity)-[:PART_OF]->(:BoundedContext)
//! - (:Term)-[:DEFINED_IN]->(:BoundedContext)
//! - (:BoundedContext)-[:UPSTREAM_OF]->(:BoundedContext)

use anyhow::Result;
use neo4rs::Query;
use tracing::debug;

use cwa_db::DbPool;
use crate::GraphClient;
use super::SyncResult;

/// Sync all domain entities for a project to Neo4j.
pub async fn sync_domain(client: &GraphClient, db: &DbPool, project_id: &str) -> Result<SyncResult> {
    let mut result = SyncResult::default();

    // Sync bounded contexts
    let contexts = cwa_db::queries::domains::list_contexts(db, project_id)
        .map_err(|e| anyhow::anyhow!("Failed to list contexts: {}", e))?;

    for ctx in &contexts {
        let query = Query::new(
            "MERGE (c:BoundedContext {id: $id})
             SET c.name = $name,
                 c.description = $description,
                 c.responsibilities = $responsibilities,
                 c.created_at = $created_at,
                 c.updated_at = $updated_at"
                .to_string(),
        )
        .param("id", ctx.id.as_str())
        .param("name", ctx.name.as_str())
        .param("description", ctx.description.as_deref().unwrap_or(""))
        .param("responsibilities", ctx.responsibilities.as_deref().unwrap_or(""))
        .param("created_at", ctx.created_at.as_str())
        .param("updated_at", ctx.updated_at.as_str());

        client.execute(query).await?;
        result.nodes_created += 1;

        // BELONGS_TO Project
        let rel_query = Query::new(
            "MATCH (c:BoundedContext {id: $ctx_id}), (p:Project {id: $project_id})
             MERGE (c)-[:BELONGS_TO]->(p)"
                .to_string(),
        )
        .param("ctx_id", ctx.id.as_str())
        .param("project_id", project_id);

        client.execute(rel_query).await?;
        result.relationships_created += 1;

        // Upstream context relationships (stored as JSON array of context IDs)
        if let Some(ref upstream_json) = ctx.upstream_contexts {
            if let Ok(upstream_ids) = serde_json::from_str::<Vec<String>>(upstream_json) {
                for up_id in &upstream_ids {
                    let up_query = Query::new(
                        "MATCH (c:BoundedContext {id: $ctx_id}), (up:BoundedContext {id: $up_id})
                         MERGE (up)-[:UPSTREAM_OF]->(c)"
                            .to_string(),
                    )
                    .param("ctx_id", ctx.id.as_str())
                    .param("up_id", up_id.as_str());

                    client.execute(up_query).await?;
                    result.relationships_created += 1;
                }
            }
        }

        // Sync domain objects for this context
        let objects = cwa_db::queries::domains::list_domain_objects(db, &ctx.id)
            .map_err(|e| anyhow::anyhow!("Failed to list domain objects: {}", e))?;

        for obj in &objects {
            let obj_query = Query::new(
                "MERGE (e:DomainEntity {id: $id})
                 SET e.name = $name,
                     e.entity_type = $entity_type,
                     e.description = $description,
                     e.properties = $properties,
                     e.behaviors = $behaviors,
                     e.invariants = $invariants,
                     e.created_at = $created_at,
                     e.updated_at = $updated_at"
                    .to_string(),
            )
            .param("id", obj.id.as_str())
            .param("name", obj.name.as_str())
            .param("entity_type", obj.object_type.as_str())
            .param("description", obj.description.as_deref().unwrap_or(""))
            .param("properties", obj.properties.as_deref().unwrap_or(""))
            .param("behaviors", obj.behaviors.as_deref().unwrap_or(""))
            .param("invariants", obj.invariants.as_deref().unwrap_or(""))
            .param("created_at", obj.created_at.as_str())
            .param("updated_at", obj.updated_at.as_str());

            client.execute(obj_query).await?;
            result.nodes_created += 1;

            // PART_OF BoundedContext
            let part_of_query = Query::new(
                "MATCH (e:DomainEntity {id: $entity_id}), (c:BoundedContext {id: $ctx_id})
                 MERGE (e)-[:PART_OF]->(c)"
                    .to_string(),
            )
            .param("entity_id", obj.id.as_str())
            .param("ctx_id", ctx.id.as_str());

            client.execute(part_of_query).await?;
            result.relationships_created += 1;

            debug!(entity_id = %obj.id, name = %obj.name, object_type = %obj.object_type, "Synced domain entity");
        }

        debug!(context_id = %ctx.id, name = %ctx.name, "Synced bounded context");
    }

    // Sync glossary terms
    let terms = cwa_db::queries::domains::list_glossary(db, project_id)
        .map_err(|e| anyhow::anyhow!("Failed to list glossary: {}", e))?;

    for term in &terms {
        let term_query = Query::new(
            "MERGE (t:Term {name: $name})
             SET t.definition = $definition,
                 t.aliases = $aliases,
                 t.created_at = $created_at,
                 t.updated_at = $updated_at"
                .to_string(),
        )
        .param("name", term.term.as_str())
        .param("definition", term.definition.as_str())
        .param("aliases", term.aliases.as_deref().unwrap_or(""))
        .param("created_at", term.created_at.as_str())
        .param("updated_at", term.updated_at.as_str());

        client.execute(term_query).await?;
        result.nodes_created += 1;

        // BELONGS_TO Project
        let proj_rel = Query::new(
            "MATCH (t:Term {name: $name}), (p:Project {id: $project_id})
             MERGE (t)-[:BELONGS_TO]->(p)"
                .to_string(),
        )
        .param("name", term.term.as_str())
        .param("project_id", project_id);

        client.execute(proj_rel).await?;
        result.relationships_created += 1;

        // DEFINED_IN BoundedContext (if context_id is set)
        if let Some(ref ctx_id) = term.context_id {
            let ctx_rel = Query::new(
                "MATCH (t:Term {name: $name}), (c:BoundedContext {id: $ctx_id})
                 MERGE (t)-[:DEFINED_IN]->(c)"
                    .to_string(),
            )
            .param("name", term.term.as_str())
            .param("ctx_id", ctx_id.as_str());

            client.execute(ctx_rel).await?;
            result.relationships_created += 1;
        }

        debug!(term = %term.term, "Synced glossary term");
    }

    Ok(result)
}
