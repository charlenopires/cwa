//! Architectural Decision Record (ADR) queries — Redis implementation.

use crate::client::{RedisError, RedisPool, RedisResult};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRow {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub status: String,
    pub context: String,
    pub decision: String,
    pub consequences: Option<String>,
    pub alternatives: Option<String>,
    pub related_specs: Option<String>,
    pub superseded_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub async fn create_decision(
    pool: &RedisPool,
    id: &str,
    project_id: &str,
    title: &str,
    context: &str,
    decision_text: &str,
) -> RedisResult<()> {
    let now = chrono::Utc::now();
    let row = DecisionRow {
        id: id.to_string(),
        project_id: project_id.to_string(),
        title: title.to_string(),
        status: "proposed".to_string(),
        context: context.to_string(),
        decision: decision_text.to_string(),
        consequences: None,
        alternatives: None,
        related_specs: None,
        superseded_by: None,
        created_at: now.to_rfc3339(),
        updated_at: now.to_rfc3339(),
    };

    let mut conn = pool.clone();
    let key = format!("cwa:{}:decision:{}", project_id, id);
    conn.hset::<_, _, _, ()>(&key, "data", serde_json::to_string(&row)?).await?;

    let zkey = format!("cwa:{}:decisions:all", project_id);
    conn.zadd::<_, _, _, ()>(&zkey, id, now.timestamp()).await?;

    Ok(())
}

pub async fn get_decision(pool: &RedisPool, decision_id: &str) -> RedisResult<DecisionRow> {
    let mut conn = pool.clone();
    let pattern = format!("cwa:*:decision:{}", decision_id);
    let mut scan: redis::AsyncIter<String> = conn.scan_match(&pattern).await?;
    if let Some(key) = scan.next_item().await {
        drop(scan);
        let mut c = pool.clone();
        let json: Option<String> = c.hget(&key, "data").await?;
        if let Some(j) = json {
            return Ok(serde_json::from_str(&j)?);
        }
    }
    Err(RedisError::NotFound(format!("Decision not found: {}", decision_id)))
}

pub async fn list_decisions(pool: &RedisPool, project_id: &str) -> RedisResult<Vec<DecisionRow>> {
    let mut conn = pool.clone();
    let zkey = format!("cwa:{}:decisions:all", project_id);
    let ids: Vec<String> = conn.zrange(&zkey, 0, -1).await?;
    let mut decisions = Vec::new();
    for id in ids {
        let key = format!("cwa:{}:decision:{}", project_id, id);
        let mut c = pool.clone();
        let json: Option<String> = c.hget(&key, "data").await?;
        if let Some(j) = json {
            if let Ok(row) = serde_json::from_str::<DecisionRow>(&j) {
                decisions.push(row);
            }
        }
    }
    Ok(decisions)
}

pub async fn list_accepted_decisions(
    pool: &RedisPool,
    project_id: &str,
) -> RedisResult<Vec<DecisionRow>> {
    let all = list_decisions(pool, project_id).await?;
    Ok(all.into_iter().filter(|d| d.status == "accepted").collect())
}

pub async fn update_decision_status(
    pool: &RedisPool,
    decision_id: &str,
    new_status: &str,
) -> RedisResult<()> {
    let mut row = get_decision(pool, decision_id).await?;
    row.status = new_status.to_string();
    row.updated_at = chrono::Utc::now().to_rfc3339();
    let mut conn = pool.clone();
    let key = format!("cwa:{}:decision:{}", row.project_id, decision_id);
    conn.hset::<_, _, _, ()>(&key, "data", serde_json::to_string(&row)?).await?;
    Ok(())
}

pub async fn supersede_decision(
    pool: &RedisPool,
    old_id: &str,
    new_id: &str,
) -> RedisResult<()> {
    let mut row = get_decision(pool, old_id).await?;
    row.status = "superseded".to_string();
    row.superseded_by = Some(new_id.to_string());
    row.updated_at = chrono::Utc::now().to_rfc3339();
    let mut conn = pool.clone();
    let key = format!("cwa:{}:decision:{}", row.project_id, old_id);
    conn.hset::<_, _, _, ()>(&key, "data", serde_json::to_string(&row)?).await?;
    Ok(())
}
