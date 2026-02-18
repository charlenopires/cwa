//! Memory and session queries — Redis implementation.

use crate::client::{RedisError, RedisPool, RedisResult};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRow {
    pub id: String,
    pub project_id: String,
    pub session_id: Option<String>,
    pub entry_type: String,
    pub content: String,
    pub importance: String,
    pub tags: Option<String>,
    pub related_entity_type: Option<String>,
    pub related_entity_id: Option<String>,
    pub created_at: String,
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRow {
    pub id: String,
    pub project_id: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub summary: Option<String>,
    pub goals: Option<String>,
    pub accomplishments: Option<String>,
}

pub async fn create_memory_entry(
    pool: &RedisPool,
    id: &str,
    project_id: &str,
    entry_type: &str,
    content: &str,
    importance: &str,
    tags: Option<&str>,
) -> RedisResult<()> {
    let now = chrono::Utc::now();
    let row = MemoryRow {
        id: id.to_string(),
        project_id: project_id.to_string(),
        session_id: None,
        entry_type: entry_type.to_string(),
        content: content.to_string(),
        importance: importance.to_string(),
        tags: tags.map(str::to_string),
        related_entity_type: None,
        related_entity_id: None,
        created_at: now.to_rfc3339(),
        expires_at: None,
    };

    let mut conn = pool.clone();
    let key = format!("cwa:{}:memory:{}", project_id, id);
    conn.hset::<_, _, _, ()>(&key, "data", serde_json::to_string(&row)?).await?;

    // Add to sorted set (score = timestamp for ordering)
    let zkey = format!("cwa:{}:memories:all", project_id);
    conn.zadd::<_, _, _, ()>(&zkey, id, now.timestamp()).await?;

    Ok(())
}

pub async fn list_memory(
    pool: &RedisPool,
    project_id: &str,
    limit: Option<i64>,
) -> RedisResult<Vec<MemoryRow>> {
    let mut conn = pool.clone();
    let zkey = format!("cwa:{}:memories:all", project_id);
    let limit = limit.unwrap_or(100);
    // Get most recent entries first
    let ids: Vec<String> = conn.zrevrange(&zkey, 0, (limit - 1) as isize).await?;
    let mut memories = Vec::new();
    for id in ids {
        let key = format!("cwa:{}:memory:{}", project_id, id);
        let mut c = pool.clone();
        let json: Option<String> = c.hget(&key, "data").await?;
        if let Some(j) = json {
            if let Ok(row) = serde_json::from_str::<MemoryRow>(&j) {
                memories.push(row);
            }
        }
    }
    Ok(memories)
}

pub async fn search_memory(
    pool: &RedisPool,
    project_id: &str,
    query: &str,
) -> RedisResult<Vec<MemoryRow>> {
    let all = list_memory(pool, project_id, None).await?;
    let query_lower = query.to_lowercase();
    Ok(all
        .into_iter()
        .filter(|m| {
            m.content.to_lowercase().contains(&query_lower)
                || m.tags
                    .as_ref()
                    .map(|t| t.to_lowercase().contains(&query_lower))
                    .unwrap_or(false)
        })
        .collect())
}

pub async fn cleanup_expired_memory(pool: &RedisPool) -> RedisResult<usize> {
    // Scan for all memory keys and delete expired ones
    let mut conn = pool.clone();
    let mut scan: redis::AsyncIter<String> = conn.scan_match("cwa:*:memory:*").await?;
    let mut keys = Vec::new();
    while let Some(key) = scan.next_item().await {
        keys.push(key);
    }
    drop(scan);

    let now = chrono::Utc::now().to_rfc3339();
    let mut deleted = 0;
    for key in keys {
        let mut c = pool.clone();
        let json: Option<String> = c.hget(&key, "data").await?;
        if let Some(j) = json {
            if let Ok(row) = serde_json::from_str::<MemoryRow>(&j) {
                if let Some(ref expires) = row.expires_at {
                    if expires < &now {
                        c.del::<_, ()>(&key).await?;
                        deleted += 1;
                    }
                }
            }
        }
    }
    Ok(deleted)
}

pub async fn create_session(
    pool: &RedisPool,
    id: &str,
    project_id: &str,
    goals: Option<&str>,
) -> RedisResult<()> {
    let now = chrono::Utc::now().to_rfc3339();
    let row = SessionRow {
        id: id.to_string(),
        project_id: project_id.to_string(),
        started_at: now,
        ended_at: None,
        summary: None,
        goals: goals.map(str::to_string),
        accomplishments: None,
    };

    let mut conn = pool.clone();
    // Session with 30-day TTL
    let key = format!("cwa:session:{}", id);
    conn.hset::<_, _, _, ()>(&key, "data", serde_json::to_string(&row)?).await?;
    conn.expire::<_, ()>(&key, 30 * 24 * 3600).await?;

    // Track active session per project
    let active_key = format!("cwa:{}:session:active", project_id);
    conn.set::<_, _, ()>(&active_key, id).await?;

    Ok(())
}

pub async fn end_session(
    pool: &RedisPool,
    session_id: &str,
    summary: Option<&str>,
    accomplishments: Option<&str>,
) -> RedisResult<()> {
    let key = format!("cwa:session:{}", session_id);
    let mut conn = pool.clone();
    let json: Option<String> = conn.hget(&key, "data").await?;
    if let Some(j) = json {
        let mut row: SessionRow = serde_json::from_str(&j)?;
        row.ended_at = Some(chrono::Utc::now().to_rfc3339());
        row.summary = summary.map(str::to_string);
        row.accomplishments = accomplishments.map(str::to_string);
        conn.hset::<_, _, _, ()>(&key, "data", serde_json::to_string(&row)?).await?;
    }
    Ok(())
}

pub async fn get_active_session(
    pool: &RedisPool,
    project_id: &str,
) -> RedisResult<Option<SessionRow>> {
    let mut conn = pool.clone();
    let active_key = format!("cwa:{}:session:active", project_id);
    let session_id: Option<String> = conn.get(&active_key).await?;
    if let Some(id) = session_id {
        let key = format!("cwa:session:{}", id);
        let json: Option<String> = conn.hget(&key, "data").await?;
        if let Some(j) = json {
            return Ok(Some(serde_json::from_str(&j)?));
        }
    }
    Ok(None)
}

pub async fn list_sessions(
    pool: &RedisPool,
    project_id: &str,
    limit: i64,
) -> RedisResult<Vec<SessionRow>> {
    // Sessions are stored individually, scan for project's sessions
    let mut conn = pool.clone();
    let mut scan: redis::AsyncIter<String> = conn.scan_match("cwa:session:*").await?;
    let mut keys = Vec::new();
    while let Some(key) = scan.next_item().await {
        keys.push(key);
    }
    drop(scan);

    let mut sessions = Vec::new();
    for key in keys {
        let mut c = pool.clone();
        let json: Option<String> = c.hget(&key, "data").await?;
        if let Some(j) = json {
            if let Ok(row) = serde_json::from_str::<SessionRow>(&j) {
                if row.project_id == project_id {
                    sessions.push(row);
                }
            }
        }
    }
    sessions.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    sessions.truncate(limit as usize);
    Ok(sessions)
}
