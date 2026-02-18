//! Glossary term queries — Redis implementation.

use crate::client::{RedisError, RedisPool, RedisResult};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlossaryTermRow {
    pub id: String,
    pub project_id: String,
    pub context_id: Option<String>,
    pub term: String,
    pub definition: String,
    pub aliases: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

fn slugify(term: &str) -> String {
    term.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

pub async fn create_glossary_term(
    pool: &RedisPool,
    id: &str,
    project_id: &str,
    term: &str,
    definition: &str,
    context_id: Option<&str>,
) -> RedisResult<()> {
    let now = chrono::Utc::now().to_rfc3339();
    let slug = slugify(term);
    let row = GlossaryTermRow {
        id: id.to_string(),
        project_id: project_id.to_string(),
        context_id: context_id.map(str::to_string),
        term: term.to_string(),
        definition: definition.to_string(),
        aliases: None,
        created_at: now.clone(),
        updated_at: now,
    };
    let mut conn = pool.clone();
    let key = format!("cwa:{}:term:{}", project_id, slug);
    conn.hset::<_, _, _, ()>(&key, "data", serde_json::to_string(&row)?).await?;
    conn.hset::<_, _, _, ()>(&key, "id", id).await?;
    let set_key = format!("cwa:{}:glossary:all", project_id);
    conn.sadd::<_, _, ()>(&set_key, &slug).await?;
    Ok(())
}

pub async fn list_glossary(pool: &RedisPool, project_id: &str) -> RedisResult<Vec<GlossaryTermRow>> {
    let mut conn = pool.clone();
    let set_key = format!("cwa:{}:glossary:all", project_id);
    let slugs: Vec<String> = conn.smembers(&set_key).await?;
    let mut terms = Vec::new();
    for slug in slugs {
        let key = format!("cwa:{}:term:{}", project_id, slug);
        let mut c = pool.clone();
        let json: Option<String> = c.hget(&key, "data").await?;
        if let Some(j) = json {
            if let Ok(row) = serde_json::from_str::<GlossaryTermRow>(&j) {
                terms.push(row);
            }
        }
    }
    terms.sort_by(|a, b| a.term.cmp(&b.term));
    Ok(terms)
}

pub async fn get_term(pool: &RedisPool, project_id: &str, term: &str) -> RedisResult<GlossaryTermRow> {
    let slug = slugify(term);
    let mut conn = pool.clone();
    let key = format!("cwa:{}:term:{}", project_id, slug);
    let json: Option<String> = conn.hget(&key, "data").await?;
    match json {
        Some(j) => Ok(serde_json::from_str(&j)?),
        None => Err(RedisError::NotFound(format!("Term not found: {}", term))),
    }
}
