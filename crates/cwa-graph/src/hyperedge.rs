//! Hyperedge patterns for the Neo4j knowledge hypergraph.
//!
//! Hyperedges are represented as intermediate nodes connecting multiple entities,
//! enabling n-ary relationships with metadata:
//!
//! ```cypher
//! (:Hyperedge {id, type, created_at})-[:INVOLVES]->(:Entity)
//! ```
//!
//! Example — a Spec that implements multiple DomainObjects via a BoundedContext:
//! ```cypher
//! (h:Hyperedge {type: "implementation"})
//!   -[:INVOLVES]->(s:Spec)
//!   -[:INVOLVES]->(c:BoundedContext)
//!   -[:INVOLVES]->(d:DomainEntity)
//! ```

use anyhow::Result;
use neo4rs::Query;
use serde::Serialize;
use uuid::Uuid;

use crate::GraphClient;

/// Entity types that can participate in a hyperedge.
#[derive(Debug, Clone, PartialEq)]
pub enum EntityType {
    Project,
    Spec,
    Task,
    BoundedContext,
    DomainObject,
    Observation,
    File,
    Decision,
}

impl EntityType {
    /// The Neo4j node label for this entity type.
    pub fn label(&self) -> &'static str {
        match self {
            EntityType::Project => "Project",
            EntityType::Spec => "Spec",
            EntityType::Task => "Task",
            EntityType::BoundedContext => "BoundedContext",
            EntityType::DomainObject => "DomainEntity",
            EntityType::Observation => "Observation",
            EntityType::File => "File",
            EntityType::Decision => "Decision",
        }
    }

    /// Parse from string (case-insensitive).
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "project" => Self::Project,
            "spec" => Self::Spec,
            "task" => Self::Task,
            "boundedcontext" | "bounded_context" => Self::BoundedContext,
            "domainobject" | "domain_object" => Self::DomainObject,
            "observation" => Self::Observation,
            "file" => Self::File,
            "decision" => Self::Decision,
            _ => Self::Spec,
        }
    }
}

/// Summary of a hyperedge returned from graph queries.
#[derive(Debug, Clone, Serialize)]
pub struct HyperedgeInfo {
    pub id: String,
    pub edge_type: String,
    pub created_at: String,
}

/// Create a hyperedge connecting multiple entities in the graph.
///
/// # Arguments
/// * `edge_type` — semantic label for the relationship (e.g. "implementation", "dependency")
/// * `entities`  — slice of `(EntityType, entity_id)` pairs to connect
///
/// Returns the ID of the newly created hyperedge node.
pub async fn create_hyperedge(
    client: &GraphClient,
    edge_type: &str,
    entities: &[(EntityType, &str)],
) -> Result<String> {
    let id = Uuid::new_v4().to_string();
    let created_at = chrono::Utc::now().to_rfc3339();

    let create_query = Query::new(
        "CREATE (h:Hyperedge {id: $id, type: $edge_type, created_at: $created_at})"
            .to_string(),
    )
    .param("id", id.as_str())
    .param("edge_type", edge_type)
    .param("created_at", created_at.as_str());

    client.execute(create_query).await?;

    for (entity_type, entity_id) in entities {
        let label = entity_type.label();
        let involves_query = Query::new(format!(
            "MATCH (h:Hyperedge {{id: $h_id}}), (e:{label} {{id: $entity_id}})
             MERGE (h)-[:INVOLVES]->(e)"
        ))
        .param("h_id", id.as_str())
        .param("entity_id", *entity_id);

        client.execute(involves_query).await?;
    }

    Ok(id)
}

/// Find all hyperedges that involve a given entity.
///
/// Returns up to 50 most-recent hyperedges ordered by `created_at` descending.
pub async fn find_hyperedges_for_entity(
    client: &GraphClient,
    entity_type: EntityType,
    entity_id: &str,
) -> Result<Vec<HyperedgeInfo>> {
    let label = entity_type.label();

    let query = Query::new(format!(
        "MATCH (h:Hyperedge)-[:INVOLVES]->(e:{label} {{id: $entity_id}})
         RETURN h.id AS id, h.type AS edge_type, h.created_at AS created_at
         ORDER BY h.created_at DESC
         LIMIT 50"
    ))
    .param("entity_id", entity_id);

    let rows = client.query(query).await?;

    let mut hyperedges = Vec::with_capacity(rows.len());
    for row in rows {
        let id: String = row.get("id").unwrap_or_default();
        let edge_type: String = row.get("edge_type").unwrap_or_default();
        let created_at: String = row.get("created_at").unwrap_or_default();
        hyperedges.push(HyperedgeInfo { id, edge_type, created_at });
    }

    Ok(hyperedges)
}
