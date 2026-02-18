//! Domain model queries (bounded contexts, domain objects) — Redis implementation.

use crate::client::{RedisError, RedisPool, RedisResult};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundedContextRow {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub description: Option<String>,
    pub responsibilities: Option<String>,
    pub upstream_contexts: Option<String>,
    pub downstream_contexts: Option<String>,
    pub relationship_type: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainObjectRow {
    pub id: String,
    pub context_id: String,
    pub name: String,
    pub object_type: String,
    pub description: Option<String>,
    pub properties: Option<String>,
    pub behaviors: Option<String>,
    pub invariants: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub async fn create_context(
    pool: &RedisPool,
    id: &str,
    project_id: &str,
    name: &str,
    description: Option<&str>,
) -> RedisResult<()> {
    let now = chrono::Utc::now().to_rfc3339();
    let row = BoundedContextRow {
        id: id.to_string(),
        project_id: project_id.to_string(),
        name: name.to_string(),
        description: description.map(str::to_string),
        responsibilities: None,
        upstream_contexts: None,
        downstream_contexts: None,
        relationship_type: None,
        created_at: now.clone(),
        updated_at: now,
    };
    let mut conn = pool.clone();
    let key = format!("cwa:{}:context:{}", project_id, id);
    conn.hset::<_, _, _, ()>(&key, "data", serde_json::to_string(&row)?).await?;
    let set_key = format!("cwa:{}:contexts:all", project_id);
    conn.sadd::<_, _, ()>(&set_key, id).await?;
    Ok(())
}

pub async fn get_context(pool: &RedisPool, context_id: &str) -> RedisResult<BoundedContextRow> {
    let mut conn = pool.clone();
    let pattern = format!("cwa:*:context:{}", context_id);
    let mut scan: redis::AsyncIter<String> = conn.scan_match(&pattern).await?;
    if let Some(key) = scan.next_item().await {
        drop(scan);
        let mut c = pool.clone();
        let json: Option<String> = c.hget(&key, "data").await?;
        if let Some(j) = json {
            return Ok(serde_json::from_str(&j)?);
        }
    }
    Err(RedisError::NotFound(format!("Context not found: {}", context_id)))
}

pub async fn get_context_in_project(
    pool: &RedisPool,
    project_id: &str,
    context_id: &str,
) -> RedisResult<BoundedContextRow> {
    let mut conn = pool.clone();
    let key = format!("cwa:{}:context:{}", project_id, context_id);
    let json: Option<String> = conn.hget(&key, "data").await?;
    match json {
        Some(j) => Ok(serde_json::from_str(&j)?),
        None => Err(RedisError::NotFound(format!("Context not found: {}", context_id))),
    }
}

pub async fn list_contexts(
    pool: &RedisPool,
    project_id: &str,
) -> RedisResult<Vec<BoundedContextRow>> {
    let mut conn = pool.clone();
    let set_key = format!("cwa:{}:contexts:all", project_id);
    let ids: Vec<String> = conn.smembers(&set_key).await?;
    let mut contexts = Vec::new();
    for id in ids {
        let key = format!("cwa:{}:context:{}", project_id, id);
        let mut c = pool.clone();
        let json: Option<String> = c.hget(&key, "data").await?;
        if let Some(j) = json {
            if let Ok(row) = serde_json::from_str::<BoundedContextRow>(&j) {
                contexts.push(row);
            }
        }
    }
    Ok(contexts)
}

pub async fn create_domain_object(
    pool: &RedisPool,
    id: &str,
    context_id: &str,
    name: &str,
    object_type: &str,
    description: Option<&str>,
) -> RedisResult<()> {
    // Find the project_id from context
    let ctx = get_context(pool, context_id).await?;
    let project_id = ctx.project_id;

    let now = chrono::Utc::now().to_rfc3339();
    let row = DomainObjectRow {
        id: id.to_string(),
        context_id: context_id.to_string(),
        name: name.to_string(),
        object_type: object_type.to_string(),
        description: description.map(str::to_string),
        properties: None,
        behaviors: None,
        invariants: None,
        created_at: now.clone(),
        updated_at: now,
    };

    let mut conn = pool.clone();
    let key = format!("cwa:{}:domain:{}", project_id, id);
    conn.hset::<_, _, _, ()>(&key, "data", serde_json::to_string(&row)?).await?;
    conn.hset::<_, _, _, ()>(&key, "context_id", context_id).await?;

    let ctx_key = format!("cwa:{}:domains:ctx:{}", project_id, context_id);
    conn.sadd::<_, _, ()>(&ctx_key, id).await?;

    Ok(())
}

pub async fn list_domain_objects(
    pool: &RedisPool,
    project_id: &str,
) -> RedisResult<Vec<DomainObjectRow>> {
    let mut conn = pool.clone();
    let pattern = format!("cwa:{}:domain:*", project_id);
    let mut scan: redis::AsyncIter<String> = conn.scan_match(&pattern).await?;
    let mut keys = Vec::new();
    while let Some(key) = scan.next_item().await {
        keys.push(key);
    }
    drop(scan);
    let mut objects = Vec::new();
    for key in keys {
        // Skip context index keys
        if key.contains(":domains:") {
            continue;
        }
        let mut c = pool.clone();
        let json: Option<String> = c.hget(&key, "data").await?;
        if let Some(j) = json {
            if let Ok(row) = serde_json::from_str::<DomainObjectRow>(&j) {
                objects.push(row);
            }
        }
    }
    Ok(objects)
}

pub async fn list_domain_objects_by_context(
    pool: &RedisPool,
    project_id: &str,
    context_id: &str,
) -> RedisResult<Vec<DomainObjectRow>> {
    let mut conn = pool.clone();
    let ctx_key = format!("cwa:{}:domains:ctx:{}", project_id, context_id);
    let ids: Vec<String> = conn.smembers(&ctx_key).await?;
    let mut objects = Vec::new();
    for id in ids {
        let key = format!("cwa:{}:domain:{}", project_id, id);
        let mut c = pool.clone();
        let json: Option<String> = c.hget(&key, "data").await?;
        if let Some(j) = json {
            if let Ok(row) = serde_json::from_str::<DomainObjectRow>(&j) {
                objects.push(row);
            }
        }
    }
    Ok(objects)
}
