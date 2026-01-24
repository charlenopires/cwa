//! Memory indexing pipeline.
//!
//! Handles storing memories with embeddings - creates the SQLite record,
//! generates an embedding via Ollama, and upserts into Qdrant.

use anyhow::{Context, Result};
use tracing::{debug, info};
use uuid::Uuid;

use cwa_db::DbPool;
use crate::ollama::OllamaClient;
use crate::qdrant::{QdrantStore, MEMORIES_COLLECTION};

/// Memory entry types.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MemoryType {
    Preference,
    Decision,
    Fact,
    Pattern,
    DesignSystem,
}

impl MemoryType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Preference => "preference",
            Self::Decision => "decision",
            Self::Fact => "fact",
            Self::Pattern => "pattern",
            Self::DesignSystem => "design_system",
        }
    }

    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "preference" => Ok(Self::Preference),
            "decision" => Ok(Self::Decision),
            "fact" => Ok(Self::Fact),
            "pattern" => Ok(Self::Pattern),
            "design_system" => Ok(Self::DesignSystem),
            _ => anyhow::bail!("Invalid memory type: '{}'. Use: preference, decision, fact, pattern, design_system", s),
        }
    }
}

/// Pipeline for adding memories with embeddings.
pub struct MemoryPipeline {
    ollama: OllamaClient,
    qdrant: QdrantStore,
}

/// Result of adding a memory.
#[derive(Debug)]
pub struct AddMemoryResult {
    pub id: String,
    pub embedding_dim: usize,
}

impl MemoryPipeline {
    /// Create a new memory pipeline with the given clients.
    pub fn new(ollama: OllamaClient, qdrant: QdrantStore) -> Self {
        Self { ollama, qdrant }
    }

    /// Create a pipeline with default client configurations.
    pub fn default_pipeline() -> Result<Self> {
        Ok(Self {
            ollama: OllamaClient::default_client(),
            qdrant: QdrantStore::default_store()?,
        })
    }

    /// Add a memory: store in SQLite, embed, and upsert to Qdrant.
    pub async fn add_memory(
        &self,
        db: &DbPool,
        project_id: &str,
        content: &str,
        entry_type: MemoryType,
        context: Option<&str>,
    ) -> Result<AddMemoryResult> {
        let id = Uuid::new_v4().to_string();

        // Generate embedding
        let embedding = self.ollama.embed(content).await
            .context("Failed to generate embedding")?;
        let dim = embedding.len();

        // Store in SQLite
        let embedding_id = format!("qdrant:{}", id);
        store_memory_in_db(db, &id, project_id, content, entry_type, context, &embedding_id)?;

        // Upsert to Qdrant
        let payload = serde_json::json!({
            "id": id,
            "project_id": project_id,
            "content": content,
            "entry_type": entry_type.as_str(),
            "context": context.unwrap_or(""),
            "created_at": chrono::Utc::now().to_rfc3339(),
        });

        self.qdrant.upsert(MEMORIES_COLLECTION, &id, embedding, payload).await
            .context("Failed to upsert to Qdrant")?;

        info!(id = %id, entry_type = entry_type.as_str(), dim, "Memory added");

        Ok(AddMemoryResult { id, embedding_dim: dim })
    }

    /// Import existing memory entries from the old `memory` table to the new `memories` table with embeddings.
    pub async fn import_legacy_memories(
        &self,
        db: &DbPool,
        project_id: &str,
    ) -> Result<usize> {
        let entries = list_legacy_memories(db, project_id)?;
        let mut count = 0;

        for entry in &entries {
            // Skip if already has an embedding_id in memories table
            if memory_exists(db, &entry.id)? {
                debug!(id = %entry.id, "Skipping already-imported memory");
                continue;
            }

            // Generate embedding for the content
            let embedding = match self.ollama.embed(&entry.content).await {
                Ok(e) => e,
                Err(e) => {
                    debug!(id = %entry.id, error = %e, "Skipping memory (embedding failed)");
                    continue;
                }
            };

            // Store in memories table
            let memory_id = Uuid::new_v4().to_string();
            let embedding_id = format!("qdrant:{}", memory_id);
            store_memory_in_db(
                db, &memory_id, project_id, &entry.content,
                MemoryType::Fact, entry.context.as_deref(), &embedding_id,
            )?;

            // Upsert to Qdrant
            let payload = serde_json::json!({
                "id": memory_id,
                "project_id": project_id,
                "content": entry.content,
                "entry_type": "fact",
                "context": entry.context.as_deref().unwrap_or(""),
                "created_at": chrono::Utc::now().to_rfc3339(),
            });

            self.qdrant.upsert(MEMORIES_COLLECTION, &memory_id, embedding, payload).await?;
            count += 1;
        }

        info!(count, "Imported legacy memories");
        Ok(count)
    }

    /// Remove memories with confidence below a threshold.
    pub async fn compact_memories(
        &self,
        db: &DbPool,
        project_id: &str,
        min_confidence: f64,
        keep_top: Option<usize>,
    ) -> Result<usize> {
        let removed = remove_low_confidence_memories(db, project_id, min_confidence, keep_top)?;

        // Also remove from Qdrant
        for id in &removed {
            let _ = self.qdrant.delete(MEMORIES_COLLECTION, id).await;
        }

        info!(count = removed.len(), min_confidence, "Compacted memories");
        Ok(removed.len())
    }

    /// Get a reference to the Ollama client.
    pub fn ollama(&self) -> &OllamaClient {
        &self.ollama
    }

    /// Get a reference to the Qdrant store.
    pub fn qdrant(&self) -> &QdrantStore {
        &self.qdrant
    }
}

/// Legacy memory entry from the old `memory` table.
struct LegacyMemoryEntry {
    id: String,
    content: String,
    context: Option<String>,
}

/// Store a memory in the new `memories` table.
fn store_memory_in_db(
    db: &DbPool,
    id: &str,
    project_id: &str,
    content: &str,
    entry_type: MemoryType,
    context: Option<&str>,
    embedding_id: &str,
) -> Result<()> {
    db.with_conn(|conn| {
        conn.execute(
            "INSERT INTO memories (id, project_id, content, entry_type, context, embedding_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![id, project_id, content, entry_type.as_str(), context, embedding_id],
        )?;
        Ok(())
    })
    .map_err(|e| anyhow::anyhow!("Failed to store memory: {}", e))
}

/// Check if a memory with this ID already exists.
fn memory_exists(db: &DbPool, id: &str) -> Result<bool> {
    db.with_conn(|conn| {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM memories WHERE id = ?1",
            rusqlite::params![id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    })
    .map_err(|e| anyhow::anyhow!("Failed to check memory: {}", e))
}

/// List entries from the legacy `memory` table.
fn list_legacy_memories(db: &DbPool, project_id: &str) -> Result<Vec<LegacyMemoryEntry>> {
    db.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, content, context FROM memory WHERE project_id = ?1"
        )?;

        let rows = stmt.query_map(rusqlite::params![project_id], |row| {
            Ok(LegacyMemoryEntry {
                id: row.get(0)?,
                content: row.get(1)?,
                context: row.get(2)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| cwa_db::pool::DbError::Connection(e))
    })
    .map_err(|e| anyhow::anyhow!("Failed to list legacy memories: {}", e))
}

/// Remove memories below a confidence threshold, return their IDs.
fn remove_low_confidence_memories(
    db: &DbPool,
    project_id: &str,
    min_confidence: f64,
    keep_top: Option<usize>,
) -> Result<Vec<String>> {
    db.with_conn(|conn| {
        // First, find IDs to remove
        let query = match keep_top {
            Some(top) => format!(
                "SELECT id FROM memories
                 WHERE project_id = ?1 AND confidence < ?2
                 ORDER BY confidence ASC
                 LIMIT {}",
                top
            ),
            None => "SELECT id FROM memories WHERE project_id = ?1 AND confidence < ?2".to_string(),
        };

        let mut stmt = conn.prepare(&query)?;
        let ids: Vec<String> = stmt
            .query_map(rusqlite::params![project_id, min_confidence], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        // Delete them
        for id in &ids {
            conn.execute("DELETE FROM memories WHERE id = ?1", rusqlite::params![id])?;
        }

        Ok(ids)
    })
    .map_err(|e| anyhow::anyhow!("Failed to compact memories: {}", e))
}
