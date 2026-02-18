//! Specification queries — Redis implementation.

use crate::client::{RedisError, RedisPool, RedisResult};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecRow {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: String,
    pub acceptance_criteria: Option<String>,
    pub dependencies: Option<String>,
    pub context_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub archived_at: Option<String>,
}

pub async fn create_spec(
    pool: &RedisPool,
    id: &str,
    project_id: &str,
    title: &str,
    description: Option<&str>,
    priority: &str,
) -> RedisResult<()> {
    let now = chrono::Utc::now();
    let row = SpecRow {
        id: id.to_string(),
        project_id: project_id.to_string(),
        title: title.to_string(),
        description: description.map(str::to_string),
        status: "draft".to_string(),
        priority: priority.to_string(),
        acceptance_criteria: None,
        dependencies: None,
        context_id: None,
        created_at: now.to_rfc3339(),
        updated_at: now.to_rfc3339(),
        archived_at: None,
    };
    save_spec(pool, &row, now.timestamp()).await
}

pub async fn create_spec_with_criteria(
    pool: &RedisPool,
    id: &str,
    project_id: &str,
    title: &str,
    description: Option<&str>,
    priority: &str,
    criteria_json: &str,
) -> RedisResult<()> {
    let now = chrono::Utc::now();
    let row = SpecRow {
        id: id.to_string(),
        project_id: project_id.to_string(),
        title: title.to_string(),
        description: description.map(str::to_string),
        status: "draft".to_string(),
        priority: priority.to_string(),
        acceptance_criteria: Some(criteria_json.to_string()),
        dependencies: None,
        context_id: None,
        created_at: now.to_rfc3339(),
        updated_at: now.to_rfc3339(),
        archived_at: None,
    };
    save_spec(pool, &row, now.timestamp()).await
}

async fn save_spec(pool: &RedisPool, row: &SpecRow, score: i64) -> RedisResult<()> {
    let mut conn = pool.clone();
    let key = format!("cwa:{}:spec:{}", row.project_id, row.id);
    let json = serde_json::to_string(row)?;
    conn.hset::<_, _, _, ()>(&key, "data", &json).await?;
    conn.hset::<_, _, _, ()>(&key, "status", &row.status).await?;

    // Add to all-specs sorted set
    let zkey = format!("cwa:{}:specs:all", row.project_id);
    conn.zadd::<_, _, _, ()>(&zkey, &row.id, score).await?;

    // Add to status index
    let skey = format!("cwa:{}:specs:status:{}", row.project_id, row.status);
    conn.sadd::<_, _, ()>(&skey, &row.id).await?;

    Ok(())
}

pub async fn get_spec(pool: &RedisPool, spec_id: &str) -> RedisResult<SpecRow> {
    // spec_id might be "project_id/spec_id" or just spec_id — try to find it
    // We need to scan for the key since we don't know the project_id
    let mut conn = pool.clone();
    let pattern = format!("cwa:*:spec:{}", spec_id);
    let mut scan: redis::AsyncIter<String> = conn.scan_match(&pattern).await?;
    if let Some(key) = scan.next_item().await {
        drop(scan);
        let mut c = pool.clone();
        let json: Option<String> = c.hget(&key, "data").await?;
        if let Some(j) = json {
            return Ok(serde_json::from_str(&j)?);
        }
    }
    Err(RedisError::NotFound(format!("Spec not found: {}", spec_id)))
}

pub async fn get_spec_in_project(
    pool: &RedisPool,
    project_id: &str,
    spec_id: &str,
) -> RedisResult<SpecRow> {
    let mut conn = pool.clone();
    let key = format!("cwa:{}:spec:{}", project_id, spec_id);
    let json: Option<String> = conn.hget(&key, "data").await?;
    match json {
        Some(j) => Ok(serde_json::from_str(&j)?),
        None => Err(RedisError::NotFound(format!("Spec not found: {}", spec_id))),
    }
}

pub async fn get_spec_by_id_prefix(pool: &RedisPool, prefix: &str) -> RedisResult<SpecRow> {
    let mut conn = pool.clone();
    let pattern = format!("cwa:*:spec:{}*", prefix);
    let mut scan: redis::AsyncIter<String> = conn.scan_match(&pattern).await?;
    if let Some(key) = scan.next_item().await {
        drop(scan);
        let mut c = pool.clone();
        let json: Option<String> = c.hget(&key, "data").await?;
        if let Some(j) = json {
            return Ok(serde_json::from_str(&j)?);
        }
    }
    Err(RedisError::NotFound(format!("Spec not found with prefix: {}", prefix)))
}

pub async fn get_spec_by_title(
    pool: &RedisPool,
    project_id: &str,
    title: &str,
) -> RedisResult<SpecRow> {
    let specs = list_specs(pool, project_id).await?;
    specs
        .into_iter()
        .find(|s| s.title.to_lowercase().contains(&title.to_lowercase()))
        .ok_or_else(|| RedisError::NotFound(format!("Spec not found: {}", title)))
}

pub async fn list_specs(pool: &RedisPool, project_id: &str) -> RedisResult<Vec<SpecRow>> {
    let mut conn = pool.clone();
    let zkey = format!("cwa:{}:specs:all", project_id);
    let ids: Vec<String> = conn.zrange(&zkey, 0, -1).await?;
    let mut specs = Vec::new();
    for id in ids {
        let key = format!("cwa:{}:spec:{}", project_id, id);
        let mut c = pool.clone();
        let json: Option<String> = c.hget(&key, "data").await?;
        if let Some(j) = json {
            if let Ok(row) = serde_json::from_str::<SpecRow>(&j) {
                specs.push(row);
            }
        }
    }
    Ok(specs)
}

pub async fn get_active_spec(pool: &RedisPool, project_id: &str) -> RedisResult<Option<SpecRow>> {
    let mut conn = pool.clone();
    let skey = format!("cwa:{}:specs:status:active", project_id);
    let ids: Vec<String> = conn.smembers(&skey).await?;
    if let Some(id) = ids.into_iter().next() {
        let key = format!("cwa:{}:spec:{}", project_id, id);
        let mut c = pool.clone();
        let json: Option<String> = c.hget(&key, "data").await?;
        if let Some(j) = json {
            return Ok(Some(serde_json::from_str(&j)?));
        }
    }
    Ok(None)
}

pub async fn update_spec_status(
    pool: &RedisPool,
    spec_id: &str,
    new_status: &str,
) -> RedisResult<()> {
    let row = get_spec(pool, spec_id).await?;
    let old_status = row.status.clone();

    let mut updated = row.clone();
    updated.status = new_status.to_string();
    updated.updated_at = chrono::Utc::now().to_rfc3339();
    if new_status == "archived" {
        updated.archived_at = Some(chrono::Utc::now().to_rfc3339());
    }

    let mut conn = pool.clone();
    let key = format!("cwa:{}:spec:{}", row.project_id, spec_id);
    conn.hset::<_, _, _, ()>(&key, "data", serde_json::to_string(&updated)?).await?;
    conn.hset::<_, _, _, ()>(&key, "status", new_status).await?;

    // Update status indexes
    let old_skey = format!("cwa:{}:specs:status:{}", row.project_id, old_status);
    conn.srem::<_, _, ()>(&old_skey, spec_id).await?;
    let new_skey = format!("cwa:{}:specs:status:{}", row.project_id, new_status);
    conn.sadd::<_, _, ()>(&new_skey, spec_id).await?;

    Ok(())
}

pub async fn update_acceptance_criteria(
    pool: &RedisPool,
    spec_id: &str,
    criteria_json: &str,
) -> RedisResult<()> {
    let mut row = get_spec(pool, spec_id).await?;
    row.acceptance_criteria = Some(criteria_json.to_string());
    row.updated_at = chrono::Utc::now().to_rfc3339();
    let mut conn = pool.clone();
    let key = format!("cwa:{}:spec:{}", row.project_id, spec_id);
    conn.hset::<_, _, _, ()>(&key, "data", serde_json::to_string(&row)?).await?;
    Ok(())
}

pub async fn delete_all_specs(pool: &RedisPool, project_id: &str) -> RedisResult<usize> {
    let specs = list_specs(pool, project_id).await?;
    let count = specs.len();
    let mut conn = pool.clone();
    for spec in &specs {
        let key = format!("cwa:{}:spec:{}", project_id, spec.id);
        conn.del::<_, ()>(&key).await?;
    }
    let zkey = format!("cwa:{}:specs:all", project_id);
    conn.del::<_, ()>(&zkey).await?;
    for status in &["draft", "active", "in_review", "accepted", "completed", "archived"] {
        let skey = format!("cwa:{}:specs:status:{}", project_id, status);
        conn.del::<_, ()>(&skey).await?;
    }
    Ok(count)
}
