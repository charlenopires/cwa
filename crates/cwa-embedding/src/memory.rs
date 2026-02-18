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
    Observation,
}

impl MemoryType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Preference => "preference",
            Self::Decision => "decision",
            Self::Fact => "fact",
            Self::Pattern => "pattern",
            Self::DesignSystem => "design_system",
            Self::Observation => "observation",
        }
    }

    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "preference" => Ok(Self::Preference),
            "decision" => Ok(Self::Decision),
            "fact" => Ok(Self::Fact),
            "pattern" => Ok(Self::Pattern),
            "design_system" => Ok(Self::DesignSystem),
            "observation" => Ok(Self::Observation),
            _ => anyhow::bail!("Invalid memory type: '{}'. Use: preference, decision, fact, pattern, design_system, observation", s),
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
        store_memory_in_db(db, &id, project_id, content, entry_type, context, &embedding_id).await?;

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
            if memory_exists(db, project_id, &entry.id).await? {
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
            ).await?;

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

/// Store a memory in the new `memories` table (Redis backend).
async fn store_memory_in_db(
    db: &DbPool,
    id: &str,
    project_id: &str,
    content: &str,
    entry_type: MemoryType,
    _context: Option<&str>,
    _embedding_id: &str,
) -> Result<()> {
    cwa_db::queries::memory::create_memory_entry(
        db, id, project_id, entry_type.as_str(), content, "normal", None,
    ).await
    .map_err(|e| anyhow::anyhow!("Failed to store memory: {}", e))
}

/// Check if a memory with this ID already exists (Redis backend).
async fn memory_exists(
    db: &DbPool,
    project_id: &str,
    _id: &str,
) -> Result<bool> {
    // With Redis we don't need to check existence before import — always return false
    // to allow import, relying on idempotent upsert logic.
    let _ = db;
    let _ = project_id;
    Ok(false)
}

/// List entries from the legacy `memory` table (stub — no legacy table with Redis backend).
fn list_legacy_memories(
    _db: &DbPool,
    _project_id: &str,
) -> Result<Vec<LegacyMemoryEntry>> {
    Ok(vec![])
}

/// Remove memories below a confidence threshold (stub — returns empty with Redis backend).
fn remove_low_confidence_memories(
    _db: &DbPool,
    _project_id: &str,
    _min_confidence: f64,
    _keep_top: Option<usize>,
) -> Result<Vec<String>> {
    Ok(vec![])
}
