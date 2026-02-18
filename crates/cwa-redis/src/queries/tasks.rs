//! Task (Kanban) queries — Redis implementation.

use crate::client::{RedisError, RedisPool, RedisResult};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRow {
    pub id: String,
    pub project_id: String,
    pub spec_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: String,
    pub assignee: Option<String>,
    pub labels: Option<String>,
    pub estimated_effort: Option<String>,
    pub actual_effort: Option<String>,
    pub blocked_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

pub async fn create_task(
    pool: &RedisPool,
    id: &str,
    project_id: &str,
    title: &str,
    description: Option<&str>,
    spec_id: Option<&str>,
    priority: &str,
) -> RedisResult<()> {
    let now = chrono::Utc::now();
    let row = TaskRow {
        id: id.to_string(),
        project_id: project_id.to_string(),
        spec_id: spec_id.map(str::to_string),
        title: title.to_string(),
        description: description.map(str::to_string),
        status: "backlog".to_string(),
        priority: priority.to_string(),
        assignee: None,
        labels: None,
        estimated_effort: None,
        actual_effort: None,
        blocked_by: None,
        created_at: now.to_rfc3339(),
        updated_at: now.to_rfc3339(),
        started_at: None,
        completed_at: None,
    };
    save_task(pool, &row, now.timestamp()).await
}

async fn save_task(pool: &RedisPool, row: &TaskRow, score: i64) -> RedisResult<()> {
    let mut conn = pool.clone();
    let key = format!("cwa:{}:task:{}", row.project_id, row.id);
    let json = serde_json::to_string(row)?;
    conn.hset::<_, _, _, ()>(&key, "data", &json).await?;
    conn.hset::<_, _, _, ()>(&key, "status", &row.status).await?;

    // Sorted set
    let zkey = format!("cwa:{}:tasks:all", row.project_id);
    conn.zadd::<_, _, _, ()>(&zkey, &row.id, score).await?;

    // Status index
    let skey = format!("cwa:{}:tasks:status:{}", row.project_id, row.status);
    conn.sadd::<_, _, ()>(&skey, &row.id).await?;

    // Spec index
    if let Some(ref sid) = row.spec_id {
        let spec_key = format!("cwa:{}:tasks:spec:{}", row.project_id, sid);
        conn.sadd::<_, _, ()>(&spec_key, &row.id).await?;
    }

    Ok(())
}

pub async fn get_task(pool: &RedisPool, task_id: &str) -> RedisResult<TaskRow> {
    let mut conn = pool.clone();
    let pattern = format!("cwa:*:task:{}", task_id);
    let mut scan: redis::AsyncIter<String> = conn.scan_match(&pattern).await?;
    if let Some(key) = scan.next_item().await {
        drop(scan);
        let mut c = pool.clone();
        let json: Option<String> = c.hget(&key, "data").await?;
        if let Some(j) = json {
            return Ok(serde_json::from_str(&j)?);
        }
    }
    Err(RedisError::NotFound(format!("Task not found: {}", task_id)))
}

pub async fn get_task_in_project(
    pool: &RedisPool,
    project_id: &str,
    task_id: &str,
) -> RedisResult<TaskRow> {
    let mut conn = pool.clone();
    let key = format!("cwa:{}:task:{}", project_id, task_id);
    let json: Option<String> = conn.hget(&key, "data").await?;
    match json {
        Some(j) => Ok(serde_json::from_str(&j)?),
        None => Err(RedisError::NotFound(format!("Task not found: {}", task_id))),
    }
}

pub async fn get_current_task(pool: &RedisPool, project_id: &str) -> RedisResult<Option<TaskRow>> {
    let tasks = list_tasks_by_status(pool, project_id, "in_progress").await?;
    Ok(tasks.into_iter().next())
}

pub async fn list_tasks(pool: &RedisPool, project_id: &str) -> RedisResult<Vec<TaskRow>> {
    let mut conn = pool.clone();
    let zkey = format!("cwa:{}:tasks:all", project_id);
    let ids: Vec<String> = conn.zrange(&zkey, 0, -1).await?;
    let mut tasks = Vec::new();
    for id in ids {
        let key = format!("cwa:{}:task:{}", project_id, id);
        let mut c = pool.clone();
        let json: Option<String> = c.hget(&key, "data").await?;
        if let Some(j) = json {
            if let Ok(row) = serde_json::from_str::<TaskRow>(&j) {
                tasks.push(row);
            }
        }
    }
    Ok(tasks)
}

pub async fn list_tasks_by_spec(pool: &RedisPool, spec_id: &str) -> RedisResult<Vec<TaskRow>> {
    // Need to scan since we don't know project_id
    let mut conn = pool.clone();
    let pattern = format!("cwa:*:tasks:spec:{}", spec_id);
    let mut scan: redis::AsyncIter<String> = conn.scan_match(&pattern).await?;
    let mut task_ids_with_proj: Vec<(String, String)> = Vec::new();
    while let Some(key) = scan.next_item().await {
        // Extract project_id from key: cwa:{project_id}:tasks:spec:{spec_id}
        let parts: Vec<&str> = key.splitn(4, ':').collect();
        if parts.len() >= 2 {
            let project_id = parts[1].to_string();
            let mut c = pool.clone();
            let ids: Vec<String> = c.smembers(&key).await?;
            for id in ids {
                task_ids_with_proj.push((project_id.clone(), id));
            }
        }
    }
    drop(scan);
    let mut tasks = Vec::new();
    for (project_id, task_id) in task_ids_with_proj {
        if let Ok(row) = get_task_in_project(pool, &project_id, &task_id).await {
            tasks.push(row);
        }
    }
    Ok(tasks)
}

pub async fn list_tasks_by_spec_in_project(
    pool: &RedisPool,
    project_id: &str,
    spec_id: &str,
) -> RedisResult<Vec<TaskRow>> {
    let mut conn = pool.clone();
    let spec_key = format!("cwa:{}:tasks:spec:{}", project_id, spec_id);
    let ids: Vec<String> = conn.smembers(&spec_key).await?;
    let mut tasks = Vec::new();
    for id in ids {
        if let Ok(row) = get_task_in_project(pool, project_id, &id).await {
            tasks.push(row);
        }
    }
    Ok(tasks)
}

pub async fn list_tasks_by_status(
    pool: &RedisPool,
    project_id: &str,
    status: &str,
) -> RedisResult<Vec<TaskRow>> {
    let mut conn = pool.clone();
    let skey = format!("cwa:{}:tasks:status:{}", project_id, status);
    let ids: Vec<String> = conn.smembers(&skey).await?;
    let mut tasks = Vec::new();
    for id in ids {
        if let Ok(row) = get_task_in_project(pool, project_id, &id).await {
            tasks.push(row);
        }
    }
    Ok(tasks)
}

pub async fn update_task_status(
    pool: &RedisPool,
    task_id: &str,
    new_status: &str,
) -> RedisResult<()> {
    let row = get_task(pool, task_id).await?;
    let old_status = row.status.clone();
    let project_id = row.project_id.clone();

    let mut updated = row;
    let now = chrono::Utc::now().to_rfc3339();
    updated.status = new_status.to_string();
    updated.updated_at = now.clone();
    if new_status == "in_progress" && updated.started_at.is_none() {
        updated.started_at = Some(now.clone());
    }
    if new_status == "done" {
        updated.completed_at = Some(now);
    }

    let mut conn = pool.clone();
    let key = format!("cwa:{}:task:{}", project_id, task_id);
    conn.hset::<_, _, _, ()>(&key, "data", serde_json::to_string(&updated)?).await?;
    conn.hset::<_, _, _, ()>(&key, "status", new_status).await?;

    // Update status indexes
    let old_skey = format!("cwa:{}:tasks:status:{}", project_id, old_status);
    conn.srem::<_, _, ()>(&old_skey, task_id).await?;
    let new_skey = format!("cwa:{}:tasks:status:{}", project_id, new_status);
    conn.sadd::<_, _, ()>(&new_skey, task_id).await?;

    Ok(())
}

pub async fn count_tasks_by_status(
    pool: &RedisPool,
    project_id: &str,
    status: &str,
) -> RedisResult<i64> {
    let mut conn = pool.clone();
    let skey = format!("cwa:{}:tasks:status:{}", project_id, status);
    let count: i64 = conn.scard(&skey).await?;
    Ok(count)
}

pub async fn get_wip_limit(
    pool: &RedisPool,
    project_id: &str,
    column: &str,
) -> RedisResult<Option<i64>> {
    let mut conn = pool.clone();
    let key = format!("cwa:{}:wip:limits", project_id);
    let val: Option<String> = conn.hget(&key, column).await?;
    Ok(val.and_then(|v| v.parse::<i64>().ok()))
}

pub async fn set_wip_limit(
    pool: &RedisPool,
    project_id: &str,
    column: &str,
    limit: Option<i64>,
    _version: i32,
) -> RedisResult<()> {
    let mut conn = pool.clone();
    let key = format!("cwa:{}:wip:limits", project_id);
    match limit {
        Some(l) => conn.hset::<_, _, _, ()>(&key, column, l.to_string()).await?,
        None => conn.hdel::<_, _, ()>(&key, column).await?,
    }
    Ok(())
}

pub async fn get_all_wip_limits(pool: &RedisPool, project_id: &str) -> RedisResult<Vec<(String, i64)>> {
    let mut conn = pool.clone();
    let key = format!("cwa:{}:wip:limits", project_id);
    let map: std::collections::HashMap<String, String> = conn.hgetall(&key).await?;
    let result = map
        .into_iter()
        .filter_map(|(k, v)| v.parse::<i64>().ok().map(|n| (k, n)))
        .collect();
    Ok(result)
}

pub async fn delete_tasks_by_spec(pool: &RedisPool, spec_id: &str) -> RedisResult<usize> {
    let tasks = list_tasks_by_spec(pool, spec_id).await?;
    let count = tasks.len();
    for task in &tasks {
        let mut conn = pool.clone();
        let key = format!("cwa:{}:task:{}", task.project_id, task.id);
        conn.del::<_, ()>(&key).await?;
        // Remove from all indexes
        let zkey = format!("cwa:{}:tasks:all", task.project_id);
        conn.zrem::<_, _, ()>(&zkey, &task.id).await?;
        let skey = format!("cwa:{}:tasks:status:{}", task.project_id, task.status);
        conn.srem::<_, _, ()>(&skey, &task.id).await?;
    }
    Ok(count)
}

pub async fn delete_all_tasks(pool: &RedisPool, project_id: &str) -> RedisResult<usize> {
    let tasks = list_tasks(pool, project_id).await?;
    let count = tasks.len();
    let mut conn = pool.clone();
    for task in &tasks {
        let key = format!("cwa:{}:task:{}", project_id, task.id);
        conn.del::<_, ()>(&key).await?;
    }
    let zkey = format!("cwa:{}:tasks:all", project_id);
    conn.del::<_, ()>(&zkey).await?;
    for status in &["backlog", "todo", "in_progress", "review", "done"] {
        let skey = format!("cwa:{}:tasks:status:{}", project_id, status);
        conn.del::<_, ()>(&skey).await?;
    }
    Ok(count)
}
