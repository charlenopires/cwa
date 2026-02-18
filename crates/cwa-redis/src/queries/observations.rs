//! Observation and summary queries — Redis implementation using Streams.

use crate::client::{RedisError, RedisPool, RedisResult};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationRow {
    pub id: String,
    pub project_id: String,
    pub session_id: Option<String>,
    pub obs_type: String,
    pub title: String,
    pub narrative: Option<String>,
    pub facts: Option<String>,
    pub concepts: Option<String>,
    pub files_modified: Option<String>,
    pub files_read: Option<String>,
    pub related_entity_type: Option<String>,
    pub related_entity_id: Option<String>,
    pub confidence: f64,
    pub embedding_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationIndexRow {
    pub id: String,
    pub obs_type: String,
    pub title: String,
    pub confidence: f64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryRow {
    pub id: String,
    pub project_id: String,
    pub session_id: Option<String>,
    pub content: String,
    pub observations_count: i64,
    pub key_facts: Option<String>,
    pub time_range_start: Option<String>,
    pub time_range_end: Option<String>,
    pub created_at: String,
}

#[allow(clippy::too_many_arguments)]
pub async fn create_observation(
    pool: &RedisPool,
    id: &str,
    project_id: &str,
    session_id: Option<&str>,
    obs_type: &str,
    title: &str,
    narrative: Option<&str>,
    facts: Option<&str>,
    concepts: Option<&str>,
    files_modified: Option<&str>,
    files_read: Option<&str>,
    related_entity_type: Option<&str>,
    related_entity_id: Option<&str>,
    confidence: f64,
) -> RedisResult<()> {
    let now = chrono::Utc::now();
    let row = ObservationRow {
        id: id.to_string(),
        project_id: project_id.to_string(),
        session_id: session_id.map(str::to_string),
        obs_type: obs_type.to_string(),
        title: title.to_string(),
        narrative: narrative.map(str::to_string),
        facts: facts.map(str::to_string),
        concepts: concepts.map(str::to_string),
        files_modified: files_modified.map(str::to_string),
        files_read: files_read.map(str::to_string),
        related_entity_type: related_entity_type.map(str::to_string),
        related_entity_id: related_entity_id.map(str::to_string),
        confidence,
        embedding_id: None,
        created_at: now.to_rfc3339(),
    };

    let json = serde_json::to_string(&row)?;

    let mut conn = pool.clone();

    // Store as HASH
    let key = format!("cwa:{}:observation:{}", project_id, id);
    conn.hset::<_, _, _, ()>(&key, "data", &json).await?;

    // Add to sorted set (score = timestamp)
    let zkey = format!("cwa:{}:observations:all", project_id);
    conn.zadd::<_, _, _, ()>(&zkey, id, now.timestamp()).await?;

    // Also publish to stream for timeline
    let stream_key = format!("cwa:{}:observations", project_id);
    let fields = vec![
        ("id", id),
        ("type", obs_type),
        ("title", title),
        ("data", &json),
    ];
    conn.xadd::<_, _, _, _, ()>(&stream_key, "*", &fields).await?;

    Ok(())
}

pub async fn get_observation(
    pool: &RedisPool,
    observation_id: &str,
) -> RedisResult<Option<ObservationRow>> {
    let mut conn = pool.clone();
    let pattern = format!("cwa:*:observation:{}", observation_id);
    let mut scan: redis::AsyncIter<String> = conn.scan_match(&pattern).await?;
    if let Some(key) = scan.next_item().await {
        drop(scan);
        let mut c = pool.clone();
        let json: Option<String> = c.hget(&key, "data").await?;
        if let Some(j) = json {
            return Ok(Some(serde_json::from_str(&j)?));
        }
    }
    Ok(None)
}

pub async fn get_observations_batch(
    pool: &RedisPool,
    ids: &[&str],
) -> RedisResult<Vec<ObservationRow>> {
    let mut observations = Vec::new();
    for id in ids {
        if let Ok(Some(obs)) = get_observation(pool, id).await {
            observations.push(obs);
        }
    }
    Ok(observations)
}

pub async fn list_observations_compact(
    pool: &RedisPool,
    project_id: &str,
    offset: i64,
    limit: i64,
) -> RedisResult<Vec<ObservationIndexRow>> {
    let mut conn = pool.clone();
    let zkey = format!("cwa:{}:observations:all", project_id);
    let end = offset + limit - 1;
    let ids: Vec<String> = conn.zrevrange(&zkey, offset as isize, end as isize).await?;
    let mut results = Vec::new();
    for id in ids {
        let key = format!("cwa:{}:observation:{}", project_id, id);
        let mut c = pool.clone();
        let json: Option<String> = c.hget(&key, "data").await?;
        if let Some(j) = json {
            if let Ok(row) = serde_json::from_str::<ObservationRow>(&j) {
                results.push(ObservationIndexRow {
                    id: row.id,
                    obs_type: row.obs_type,
                    title: row.title,
                    confidence: row.confidence,
                    created_at: row.created_at,
                });
            }
        }
    }
    Ok(results)
}

pub async fn list_observations_timeline(
    pool: &RedisPool,
    project_id: &str,
    offset: i64,
    limit: i64,
) -> RedisResult<Vec<ObservationIndexRow>> {
    list_observations_compact(pool, project_id, offset, limit).await
}

pub async fn list_high_confidence(
    pool: &RedisPool,
    project_id: &str,
    min_confidence: f64,
    limit: i64,
) -> RedisResult<Vec<ObservationRow>> {
    let mut conn = pool.clone();
    let zkey = format!("cwa:{}:observations:all", project_id);
    let ids: Vec<String> = conn.zrevrange(&zkey, 0, (limit * 2 - 1) as isize).await?;
    let mut results = Vec::new();
    for id in ids {
        if results.len() >= limit as usize {
            break;
        }
        let key = format!("cwa:{}:observation:{}", project_id, id);
        let mut c = pool.clone();
        let json: Option<String> = c.hget(&key, "data").await?;
        if let Some(j) = json {
            if let Ok(row) = serde_json::from_str::<ObservationRow>(&j) {
                if row.confidence >= min_confidence {
                    results.push(row);
                }
            }
        }
    }
    Ok(results)
}

pub async fn update_confidence(
    pool: &RedisPool,
    observation_id: &str,
    confidence: f64,
) -> RedisResult<()> {
    if let Ok(Some(mut row)) = get_observation(pool, observation_id).await {
        row.confidence = confidence;
        let mut conn = pool.clone();
        let key = format!("cwa:{}:observation:{}", row.project_id, observation_id);
        conn.hset::<_, _, _, ()>(&key, "data", serde_json::to_string(&row)?).await?;
    }
    Ok(())
}

pub async fn update_embedding_id(
    pool: &RedisPool,
    observation_id: &str,
    embedding_id: &str,
) -> RedisResult<()> {
    if let Ok(Some(mut row)) = get_observation(pool, observation_id).await {
        row.embedding_id = Some(embedding_id.to_string());
        let mut conn = pool.clone();
        let key = format!("cwa:{}:observation:{}", row.project_id, observation_id);
        conn.hset::<_, _, _, ()>(&key, "data", serde_json::to_string(&row)?).await?;
    }
    Ok(())
}

pub async fn decay_all_confidence(
    pool: &RedisPool,
    project_id: &str,
    decay_factor: f64,
) -> RedisResult<usize> {
    let mut conn = pool.clone();
    let zkey = format!("cwa:{}:observations:all", project_id);
    let ids: Vec<String> = conn.zrange(&zkey, 0, -1).await?;
    let count = ids.len();
    for id in ids {
        let key = format!("cwa:{}:observation:{}", project_id, id);
        let mut c = pool.clone();
        let json: Option<String> = c.hget(&key, "data").await?;
        if let Some(j) = json {
            if let Ok(mut row) = serde_json::from_str::<ObservationRow>(&j) {
                row.confidence *= decay_factor;
                c.hset::<_, _, _, ()>(&key, "data", serde_json::to_string(&row)?).await?;
            }
        }
    }
    Ok(count)
}

pub async fn remove_low_confidence(
    pool: &RedisPool,
    project_id: &str,
    min_confidence: f64,
) -> RedisResult<Vec<String>> {
    let mut conn = pool.clone();
    let zkey = format!("cwa:{}:observations:all", project_id);
    let ids: Vec<String> = conn.zrange(&zkey, 0, -1).await?;
    let mut removed = Vec::new();
    for id in ids {
        let key = format!("cwa:{}:observation:{}", project_id, id);
        let mut c = pool.clone();
        let json: Option<String> = c.hget(&key, "data").await?;
        if let Some(j) = json {
            if let Ok(row) = serde_json::from_str::<ObservationRow>(&j) {
                if row.confidence < min_confidence {
                    c.del::<_, ()>(&key).await?;
                    conn.zrem::<_, _, ()>(&zkey, &id).await?;
                    removed.push(id);
                }
            }
        }
    }
    Ok(removed)
}

pub async fn create_summary(
    pool: &RedisPool,
    id: &str,
    project_id: &str,
    session_id: Option<&str>,
    content: &str,
    observations_count: i64,
    key_facts: Option<&str>,
    time_range_start: Option<&str>,
    time_range_end: Option<&str>,
) -> RedisResult<()> {
    let now = chrono::Utc::now();
    let row = SummaryRow {
        id: id.to_string(),
        project_id: project_id.to_string(),
        session_id: session_id.map(str::to_string),
        content: content.to_string(),
        observations_count,
        key_facts: key_facts.map(str::to_string),
        time_range_start: time_range_start.map(str::to_string),
        time_range_end: time_range_end.map(str::to_string),
        created_at: now.to_rfc3339(),
    };

    let mut conn = pool.clone();
    let key = format!("cwa:{}:summary:{}", project_id, id);
    conn.hset::<_, _, _, ()>(&key, "data", serde_json::to_string(&row)?).await?;

    let zkey = format!("cwa:{}:summaries:all", project_id);
    conn.zadd::<_, _, _, ()>(&zkey, id, now.timestamp()).await?;

    Ok(())
}

pub async fn get_recent_summaries(
    pool: &RedisPool,
    project_id: &str,
    limit: i64,
) -> RedisResult<Vec<SummaryRow>> {
    let mut conn = pool.clone();
    let zkey = format!("cwa:{}:summaries:all", project_id);
    let ids: Vec<String> = conn.zrevrange(&zkey, 0, (limit - 1) as isize).await?;
    let mut summaries = Vec::new();
    for id in ids {
        let key = format!("cwa:{}:summary:{}", project_id, id);
        let mut c = pool.clone();
        let json: Option<String> = c.hget(&key, "data").await?;
        if let Some(j) = json {
            if let Ok(row) = serde_json::from_str::<SummaryRow>(&j) {
                summaries.push(row);
            }
        }
    }
    Ok(summaries)
}
