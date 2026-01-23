//! Impact analysis queries.
//!
//! Traverses the graph to find all entities affected by changes
//! to a given entity (spec, context, task, decision).

use anyhow::Result;
use neo4rs::Query;
use serde::Serialize;

use crate::GraphClient;

/// A node affected by a change.
#[derive(Debug, Clone, Serialize)]
pub struct ImpactNode {
    pub id: String,
    pub label: String,
    pub name: String,
    pub relationship: String,
    pub depth: i64,
}

/// Analyze the impact of changes to a spec.
pub async fn spec_impact(client: &GraphClient, spec_id: &str) -> Result<Vec<ImpactNode>> {
    let query = Query::new(
        "MATCH (s:Spec {id: $id})
         OPTIONAL MATCH (s)<-[r1:IMPLEMENTS]-(t:Task)
         WITH s, collect({id: t.id, label: 'Task', name: t.title, rel: type(r1)}) as tasks
         OPTIONAL MATCH (s)<-[r2:RELATES_TO]-(d:Decision)
         WITH s, tasks, collect({id: d.id, label: 'Decision', name: d.title, rel: type(r2)}) as decisions
         OPTIONAL MATCH (s)<-[r3:DEPENDS_ON]-(dep:Spec)
         WITH s, tasks, decisions, collect({id: dep.id, label: 'Spec', name: dep.title, rel: type(r3)}) as dependents
         UNWIND (tasks + decisions + dependents) as impact
         WITH impact WHERE impact.id IS NOT NULL
         RETURN impact.id as id, impact.label as label, impact.name as name, impact.rel as relationship
         "
            .to_string(),
    )
    .param("id", spec_id);

    parse_flat_impact_rows(client.query(query).await?)
}

/// Analyze the impact of changes to a bounded context.
pub async fn context_impact(client: &GraphClient, context_id: &str) -> Result<Vec<ImpactNode>> {
    let query = Query::new(
        "MATCH (c:BoundedContext {id: $id})
         OPTIONAL MATCH (c)<-[:PART_OF]-(e:DomainEntity)
         WITH c, collect({id: e.id, label: 'DomainEntity', name: e.name, rel: 'PART_OF'}) as entities
         OPTIONAL MATCH (c)<-[:DEFINED_IN]-(t:Term)
         WITH c, entities, collect({id: t.name, label: 'Term', name: t.name, rel: 'DEFINED_IN'}) as terms
         OPTIONAL MATCH (c)<-[:UPSTREAM_OF]-(ds:BoundedContext)
         WITH c, entities, terms, collect({id: ds.id, label: 'BoundedContext', name: ds.name, rel: 'DOWNSTREAM'}) as downstreams
         UNWIND (entities + terms + downstreams) as impact
         WITH impact WHERE impact.id IS NOT NULL
         RETURN impact.id as id, impact.label as label, impact.name as name, impact.rel as relationship
         "
            .to_string(),
    )
    .param("id", context_id);

    parse_flat_impact_rows(client.query(query).await?)
}

/// Analyze the impact of changes to a task.
pub async fn task_impact(client: &GraphClient, task_id: &str) -> Result<Vec<ImpactNode>> {
    let query = Query::new(
        "MATCH (t:Task {id: $id})
         OPTIONAL MATCH (t)-[:IMPLEMENTS]->(s:Spec)
         WITH t, collect({id: s.id, label: 'Spec', name: s.title, rel: 'IMPLEMENTS'}) as specs
         OPTIONAL MATCH (t)<-[:BLOCKED_BY]-(blocked:Task)
         WITH t, specs, collect({id: blocked.id, label: 'Task', name: blocked.title, rel: 'BLOCKS'}) as blockedTasks
         UNWIND (specs + blockedTasks) as impact
         WITH impact WHERE impact.id IS NOT NULL
         RETURN impact.id as id, impact.label as label, impact.name as name, impact.rel as relationship
         "
            .to_string(),
    )
    .param("id", task_id);

    parse_flat_impact_rows(client.query(query).await?)
}

/// Analyze the impact of changes to a decision.
pub async fn decision_impact(client: &GraphClient, decision_id: &str) -> Result<Vec<ImpactNode>> {
    let query = Query::new(
        "MATCH (d:Decision {id: $id})
         OPTIONAL MATCH (d)-[:RELATES_TO]->(s:Spec)
         WITH d, collect({id: s.id, label: 'Spec', name: s.title, rel: 'RELATES_TO'}) as specs
         OPTIONAL MATCH (d)<-[:SUPERSEDED_BY]-(old:Decision)
         WITH d, specs, collect({id: old.id, label: 'Decision', name: old.title, rel: 'SUPERSEDES'}) as superseded
         OPTIONAL MATCH (d)-[:SUPERSEDED_BY]->(newer:Decision)
         WITH d, specs, superseded, collect({id: newer.id, label: 'Decision', name: newer.title, rel: 'SUPERSEDED_BY'}) as superseding
         UNWIND (specs + superseded + superseding) as impact
         WITH impact WHERE impact.id IS NOT NULL
         RETURN impact.id as id, impact.label as label, impact.name as name, impact.rel as relationship
         "
            .to_string(),
    )
    .param("id", decision_id);

    parse_flat_impact_rows(client.query(query).await?)
}

/// Generic impact analysis - dispatches to type-specific queries.
pub async fn impact_analysis(client: &GraphClient, entity_type: &str, entity_id: &str, _max_depth: u32) -> Result<Vec<ImpactNode>> {
    match entity_type {
        "spec" => spec_impact(client, entity_id).await,
        "context" => context_impact(client, entity_id).await,
        "task" => task_impact(client, entity_id).await,
        "decision" => decision_impact(client, entity_id).await,
        _ => {
            // Fallback: generic traversal with variable-length paths
            let query = Query::new(
                "MATCH (start {id: $id})
                 MATCH (start)-[r*1..3]-(connected)
                 WHERE start <> connected
                 WITH DISTINCT connected,
                      [rel IN r | type(rel)][0] as relationship
                 RETURN connected.id as id, labels(connected)[0] as label,
                        COALESCE(connected.title, connected.name, connected.id) as name,
                        relationship
                 LIMIT 50"
                    .to_string(),
            )
            .param("id", entity_id);

            parse_flat_impact_rows(client.query(query).await?)
        }
    }
}

/// Parse flat impact rows from Neo4j results.
fn parse_flat_impact_rows(rows: Vec<neo4rs::Row>) -> Result<Vec<ImpactNode>> {
    let mut nodes = Vec::new();

    for row in rows {
        let id: String = row.get("id").unwrap_or_default();
        let label: String = row.get("label").unwrap_or_default();
        let name: String = row.get("name").unwrap_or_default();
        let relationship: String = row.get("relationship").unwrap_or_default();

        if !id.is_empty() {
            nodes.push(ImpactNode {
                id,
                label,
                name,
                relationship,
                depth: 1,
            });
        }
    }

    Ok(nodes)
}
