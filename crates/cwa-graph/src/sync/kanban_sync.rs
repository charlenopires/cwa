//! Task synchronization to Neo4j.
//!
//! Creates nodes and relationships:
//! - (:Task)-[:BELONGS_TO]->(:Project)
//! - (:Task)-[:IMPLEMENTS]->(:Spec)
//! - (:Task)-[:BLOCKED_BY]->(:Task)

use anyhow::Result;
use neo4rs::Query;
use tracing::debug;

use cwa_db::DbPool;
use crate::GraphClient;
use super::SyncResult;

/// Sync all tasks for a project to Neo4j.
pub async fn sync_tasks(client: &GraphClient, db: &DbPool, project_id: &str) -> Result<SyncResult> {
    let tasks = cwa_db::queries::tasks::list_tasks(db, project_id)
        .map_err(|e| anyhow::anyhow!("Failed to list tasks: {}", e))?;

    let mut result = SyncResult::default();

    for task in &tasks {
        // MERGE the Task node
        let query = Query::new(
            "MERGE (t:Task {id: $id})
             SET t.title = $title,
                 t.description = $description,
                 t.status = $status,
                 t.priority = $priority,
                 t.assignee = $assignee,
                 t.labels = $labels,
                 t.estimated_effort = $estimated_effort,
                 t.actual_effort = $actual_effort,
                 t.created_at = $created_at,
                 t.updated_at = $updated_at,
                 t.started_at = $started_at,
                 t.completed_at = $completed_at"
                .to_string(),
        )
        .param("id", task.id.as_str())
        .param("title", task.title.as_str())
        .param("description", task.description.as_deref().unwrap_or(""))
        .param("status", task.status.as_str())
        .param("priority", task.priority.as_str())
        .param("assignee", task.assignee.as_deref().unwrap_or(""))
        .param("labels", task.labels.as_deref().unwrap_or(""))
        .param("estimated_effort", task.estimated_effort.as_deref().unwrap_or(""))
        .param("actual_effort", task.actual_effort.as_deref().unwrap_or(""))
        .param("created_at", task.created_at.as_str())
        .param("updated_at", task.updated_at.as_str())
        .param("started_at", task.started_at.as_deref().unwrap_or(""))
        .param("completed_at", task.completed_at.as_deref().unwrap_or(""));

        client.execute(query).await?;
        result.nodes_created += 1;

        // BELONGS_TO Project
        let proj_rel = Query::new(
            "MATCH (t:Task {id: $task_id}), (p:Project {id: $project_id})
             MERGE (t)-[:BELONGS_TO]->(p)"
                .to_string(),
        )
        .param("task_id", task.id.as_str())
        .param("project_id", project_id);

        client.execute(proj_rel).await?;
        result.relationships_created += 1;

        // IMPLEMENTS Spec (if spec_id is set)
        if let Some(ref spec_id) = task.spec_id {
            let spec_rel = Query::new(
                "MATCH (t:Task {id: $task_id}), (s:Spec {id: $spec_id})
                 MERGE (t)-[:IMPLEMENTS]->(s)"
                    .to_string(),
            )
            .param("task_id", task.id.as_str())
            .param("spec_id", spec_id.as_str());

            client.execute(spec_rel).await?;
            result.relationships_created += 1;
        }

        // BLOCKED_BY relationships (stored as JSON array of task IDs)
        if let Some(ref blocked_json) = task.blocked_by {
            if let Ok(blocked_ids) = serde_json::from_str::<Vec<String>>(blocked_json) {
                for blocked_id in &blocked_ids {
                    let blocked_query = Query::new(
                        "MATCH (t:Task {id: $task_id}), (blocker:Task {id: $blocker_id})
                         MERGE (t)-[:BLOCKED_BY]->(blocker)"
                            .to_string(),
                    )
                    .param("task_id", task.id.as_str())
                    .param("blocker_id", blocked_id.as_str());

                    client.execute(blocked_query).await?;
                    result.relationships_created += 1;
                }
            }
        }

        debug!(task_id = %task.id, title = %task.title, status = %task.status, "Synced task");
    }

    Ok(result)
}
