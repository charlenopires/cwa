//! Project queries — Redis implementation.

use crate::client::{RedisError, RedisPool, RedisResult};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectRow {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub constitution_path: Option<String>,
    pub status: String,
    pub tech_stack: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub async fn create_project(
    pool: &RedisPool,
    id: &str,
    name: &str,
    description: Option<&str>,
) -> RedisResult<()> {
    let mut conn = pool.clone();
    let now = chrono::Utc::now().to_rfc3339();
    let row = ProjectRow {
        id: id.to_string(),
        name: name.to_string(),
        description: description.map(str::to_string),
        constitution_path: None,
        status: "active".to_string(),
        tech_stack: None,
        created_at: now.clone(),
        updated_at: now,
    };
    let json = serde_json::to_string(&row)?;
    let key = format!("cwa:{}:info", id);
    conn.hset::<_, _, _, ()>(&key, "data", &json).await?;
    conn.hset::<_, _, _, ()>(&key, "name", name).await?;
    conn.hset::<_, _, _, ()>(&key, "status", "active").await?;
    Ok(())
}

pub async fn get_project(pool: &RedisPool, project_id: &str) -> RedisResult<ProjectRow> {
    let mut conn = pool.clone();
    let key = format!("cwa:{}:info", project_id);
    let json: Option<String> = conn.hget(&key, "data").await?;
    match json {
        Some(j) => Ok(serde_json::from_str(&j)?),
        None => Err(RedisError::NotFound(format!("Project not found: {}", project_id))),
    }
}

pub async fn get_default_project(pool: &RedisPool) -> RedisResult<Option<ProjectRow>> {
    // Scan for any project key
    let mut conn = pool.clone();
    let mut scan: redis::AsyncIter<String> = conn.scan_match("cwa:*:info").await?;
    if let Some(key) = scan.next_item().await {
        drop(scan);
        let mut conn2 = pool.clone();
        let json: Option<String> = conn2.hget(&key, "data").await?;
        if let Some(j) = json {
            return Ok(Some(serde_json::from_str(&j)?));
        }
    }
    Ok(None)
}

pub async fn list_projects(pool: &RedisPool) -> RedisResult<Vec<ProjectRow>> {
    let mut conn = pool.clone();
    let mut scan: redis::AsyncIter<String> = conn.scan_match("cwa:*:info").await?;
    let mut keys = Vec::new();
    while let Some(key) = scan.next_item().await {
        keys.push(key);
    }
    drop(scan);
    let mut projects = Vec::new();
    for key in keys {
        let mut c = pool.clone();
        let json: Option<String> = c.hget(&key, "data").await?;
        if let Some(j) = json {
            if let Ok(row) = serde_json::from_str::<ProjectRow>(&j) {
                projects.push(row);
            }
        }
    }
    Ok(projects)
}

pub async fn update_constitution_path(
    pool: &RedisPool,
    project_id: &str,
    path: &str,
) -> RedisResult<()> {
    let mut row = get_project(pool, project_id).await?;
    row.constitution_path = Some(path.to_string());
    row.updated_at = chrono::Utc::now().to_rfc3339();
    let mut conn = pool.clone();
    let key = format!("cwa:{}:info", project_id);
    conn.hset::<_, _, _, ()>(&key, "data", serde_json::to_string(&row)?).await?;
    Ok(())
}

pub async fn update_project(
    pool: &RedisPool,
    project_id: &str,
    name: &str,
    description: Option<&str>,
) -> RedisResult<()> {
    let mut row = get_project(pool, project_id).await?;
    row.name = name.to_string();
    row.description = description.map(str::to_string);
    row.updated_at = chrono::Utc::now().to_rfc3339();
    let mut conn = pool.clone();
    let key = format!("cwa:{}:info", project_id);
    conn.hset::<_, _, _, ()>(&key, "data", serde_json::to_string(&row)?).await?;
    conn.hset::<_, _, _, ()>(&key, "name", name).await?;
    Ok(())
}

/// Get arbitrary project info JSON (for tech_stack, etc.)
pub async fn get_project_info(pool: &RedisPool, project_id: &str) -> RedisResult<Option<String>> {
    let mut conn = pool.clone();
    let key = format!("cwa:{}:info", project_id);
    let json: Option<String> = conn.hget(&key, "data").await?;
    Ok(json)
}

/// Set arbitrary project info JSON.
pub async fn set_project_info(pool: &RedisPool, project_id: &str, info_json: &str) -> RedisResult<()> {
    let mut conn = pool.clone();
    let key = format!("cwa:{}:info", project_id);
    // Merge into existing row
    if let Ok(mut row) = get_project(pool, project_id).await {
        // Parse the incoming JSON to update tech_stack if present
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(info_json) {
            if let Some(ts) = v.get("tech_stack") {
                row.tech_stack = Some(ts.to_string());
            }
            if let Some(n) = v.get("name").and_then(|v| v.as_str()) {
                row.name = n.to_string();
            }
            if let Some(d) = v.get("description").and_then(|v| v.as_str()) {
                row.description = Some(d.to_string());
            }
        }
        row.updated_at = chrono::Utc::now().to_rfc3339();
        conn.hset::<_, _, _, ()>(&key, "data", serde_json::to_string(&row)?).await?;
    } else {
        // Store raw
        conn.hset::<_, _, _, ()>(&key, "extra", info_json).await?;
    }
    Ok(())
}

pub async fn get_tech_stack(pool: &RedisPool, project_id: &str) -> RedisResult<Vec<String>> {
    let row = get_project(pool, project_id).await?;
    if let Some(ts_str) = row.tech_stack {
        if let Ok(v) = serde_json::from_str::<Vec<String>>(&ts_str) {
            return Ok(v);
        }
        // Try parsing as a JSON array value
        if let Ok(serde_json::Value::Array(arr)) = serde_json::from_str::<serde_json::Value>(&ts_str) {
            let result = arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect();
            return Ok(result);
        }
    }
    Ok(vec![])
}

pub async fn set_tech_stack(pool: &RedisPool, project_id: &str, stack: &[String]) -> RedisResult<()> {
    let mut row = get_project(pool, project_id).await?;
    row.tech_stack = Some(serde_json::to_string(stack)?);
    row.updated_at = chrono::Utc::now().to_rfc3339();
    let mut conn = pool.clone();
    let key = format!("cwa:{}:info", project_id);
    conn.hset::<_, _, _, ()>(&key, "data", serde_json::to_string(&row)?).await?;
    Ok(())
}
