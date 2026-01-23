//! Graph neighborhood exploration queries.
//!
//! Explores the local neighborhood around a node, returning
//! connected nodes and their relationships.

use anyhow::Result;
use neo4rs::Query;
use serde::Serialize;

use crate::GraphClient;

/// A node in the exploration result.
#[derive(Debug, Clone, Serialize)]
pub struct ExploreNode {
    pub id: String,
    pub label: String,
    pub name: String,
}

/// A relationship in the exploration result.
#[derive(Debug, Clone, Serialize)]
pub struct ExploreRelationship {
    pub from_id: String,
    pub to_id: String,
    pub rel_type: String,
}

/// Result of a neighborhood exploration.
#[derive(Debug, Clone, Serialize)]
pub struct ExploreResult {
    pub center: Option<ExploreNode>,
    pub nodes: Vec<ExploreNode>,
    pub relationships: Vec<ExploreRelationship>,
}

/// Explore the neighborhood of an entity by type and ID.
pub async fn explore_neighborhood(
    client: &GraphClient,
    _entity_type: &str,
    entity_id: &str,
    depth: u32,
) -> Result<ExploreResult> {
    let depth = depth.min(5); // Cap at 5 to prevent runaway queries

    // Get the center node
    let center_query = Query::new(
        "MATCH (n {id: $id})
         RETURN n.id as id, labels(n)[0] as label,
                COALESCE(n.title, n.name, n.id) as name
         LIMIT 1"
            .to_string(),
    )
    .param("id", entity_id);

    let center_rows = client.query(center_query).await?;
    let center = center_rows.into_iter().next().map(|row| {
        ExploreNode {
            id: row.get("id").unwrap_or_default(),
            label: row.get("label").unwrap_or_default(),
            name: row.get("name").unwrap_or_default(),
        }
    });

    // Get connected nodes
    let nodes_query = Query::new(format!(
        "MATCH (start {{id: $id}})
         MATCH (start)-[*1..{}]-(connected)
         WHERE start <> connected
         WITH DISTINCT connected
         RETURN connected.id as id, labels(connected)[0] as label,
                COALESCE(connected.title, connected.name, connected.id) as name
         LIMIT 100",
        depth
    ))
    .param("id", entity_id);

    let node_rows = client.query(nodes_query).await?;
    let mut nodes = Vec::new();
    for row in node_rows {
        let id: String = row.get("id").unwrap_or_default();
        if !id.is_empty() {
            nodes.push(ExploreNode {
                id,
                label: row.get("label").unwrap_or_default(),
                name: row.get("name").unwrap_or_default(),
            });
        }
    }

    // Get relationships in the neighborhood
    let rels_query = Query::new(format!(
        "MATCH (start {{id: $id}})
         MATCH (start)-[*1..{}]-(connected)
         WHERE start <> connected
         WITH start, connected
         MATCH (a)-[r]-(b)
         WHERE (a = start OR a = connected) AND (b = start OR b = connected)
           AND a.id < b.id
         WITH DISTINCT startNode(r).id as from_id, endNode(r).id as to_id, type(r) as rel_type
         RETURN from_id, to_id, rel_type
         LIMIT 200",
        depth
    ))
    .param("id", entity_id);

    let rel_rows = client.query(rels_query).await?;
    let mut relationships = Vec::new();
    for row in rel_rows {
        let from_id: String = row.get("from_id").unwrap_or_default();
        let to_id: String = row.get("to_id").unwrap_or_default();
        let rel_type: String = row.get("rel_type").unwrap_or_default();

        if !from_id.is_empty() && !to_id.is_empty() {
            relationships.push(ExploreRelationship {
                from_id,
                to_id,
                rel_type,
            });
        }
    }

    Ok(ExploreResult {
        center,
        nodes,
        relationships,
    })
}

/// List all nodes of a given label type.
pub async fn list_nodes_by_label(client: &GraphClient, label: &str) -> Result<Vec<ExploreNode>> {
    // Sanitize label to prevent injection (only allow alphanumeric + underscore)
    let safe_label: String = label.chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect();

    let query = Query::new(format!(
        "MATCH (n:{})
         RETURN n.id as id, labels(n)[0] as label,
                COALESCE(n.title, n.name, n.id) as name
         ORDER BY name
         LIMIT 200",
        safe_label
    ));

    let rows = client.query(query).await?;
    let mut nodes = Vec::new();

    for row in rows {
        let id: String = row.get("id").unwrap_or_default();
        if !id.is_empty() {
            nodes.push(ExploreNode {
                id,
                label: row.get("label").unwrap_or_default(),
                name: row.get("name").unwrap_or_default(),
            });
        }
    }

    Ok(nodes)
}
