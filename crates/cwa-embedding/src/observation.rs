//! Observation embedding pipeline.
//!
//! Handles storing observations with embeddings - creates the SQLite record,
//! generates an embedding via Ollama, and upserts into Qdrant.

use anyhow::{Context, Result};
use tracing::{debug, info};
use uuid::Uuid;

use cwa_db::DbPool;
use crate::ollama::OllamaClient;
use crate::qdrant::{QdrantStore, OBSERVATIONS_COLLECTION};

/// Pipeline for adding observations with embeddings.
pub struct ObservationPipeline {
    ollama: OllamaClient,
    qdrant: QdrantStore,
}

/// Result of adding an observation.
#[derive(Debug)]
pub struct AddObservationResult {
    pub id: String,
    pub embedding_dim: usize,
}

/// A search result from observation vector search.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ObservationSearchResult {
    pub id: String,
    pub title: String,
    pub obs_type: String,
    pub score: f32,
    pub created_at: String,
}

impl ObservationPipeline {
    /// Create a new observation pipeline with the given clients.
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

    /// Add an observation: store in SQLite, embed, and upsert to Qdrant.
    pub async fn add_observation(
        &self,
        db: &DbPool,
        project_id: &str,
        obs_type: &str,
        title: &str,
        narrative: Option<&str>,
        facts: &[String],
        concepts: &[String],
        files_modified: &[String],
        files_read: &[String],
        session_id: Option<&str>,
        confidence: f64,
    ) -> Result<AddObservationResult> {
        let id = Uuid::new_v4().to_string();

        // Build embedding text from title + narrative + facts
        let mut embed_text = title.to_string();
        if let Some(n) = narrative {
            embed_text.push_str(". ");
            embed_text.push_str(n);
        }
        if !facts.is_empty() {
            embed_text.push_str(". ");
            embed_text.push_str(&facts.join(", "));
        }

        // Generate embedding
        let embedding = self.ollama.embed(&embed_text).await
            .context("Failed to generate observation embedding")?;
        let dim = embedding.len();

        // Serialize JSON arrays
        let facts_json = if facts.is_empty() { None } else { Some(serde_json::to_string(facts)?) };
        let concepts_json = if concepts.is_empty() { None } else { Some(serde_json::to_string(concepts)?) };
        let files_mod_json = if files_modified.is_empty() { None } else { Some(serde_json::to_string(files_modified)?) };
        let files_read_json = if files_read.is_empty() { None } else { Some(serde_json::to_string(files_read)?) };

        // Store in SQLite
        let embedding_id = format!("qdrant:{}", id);
        cwa_db::queries::observations::create_observation(
            db, &id, project_id, session_id, obs_type, title, narrative,
            facts_json.as_deref(), concepts_json.as_deref(),
            files_mod_json.as_deref(), files_read_json.as_deref(),
            None, None, confidence,
        ).await.map_err(|e| anyhow::anyhow!("Failed to store observation: {}", e))?;

        // Update embedding ID
        cwa_db::queries::observations::update_embedding_id(db, &id, &embedding_id).await
            .map_err(|e| anyhow::anyhow!("Failed to update embedding ID: {}", e))?;

        // Upsert to Qdrant
        let payload = serde_json::json!({
            "id": id,
            "project_id": project_id,
            "obs_type": obs_type,
            "title": title,
            "narrative": narrative.unwrap_or(""),
            "created_at": chrono::Utc::now().to_rfc3339(),
        });

        self.qdrant.upsert(OBSERVATIONS_COLLECTION, &id, embedding, payload).await
            .context("Failed to upsert observation to Qdrant")?;

        info!(id = %id, obs_type, dim, "Observation added");

        Ok(AddObservationResult { id, embedding_dim: dim })
    }

    /// Search observations by semantic similarity.
    pub async fn search_observations(
        &self,
        query: &str,
        project_id: &str,
        top_k: u64,
    ) -> Result<Vec<ObservationSearchResult>> {
        let query_vector = self.ollama.embed(query).await
            .context("Failed to embed observation search query")?;

        debug!(query, dim = query_vector.len(), "Generated observation query embedding");

        let results = self.qdrant.search_filtered(
            OBSERVATIONS_COLLECTION,
            query_vector,
            top_k,
            project_id,
        ).await.context("Failed to search observations in Qdrant")?;

        let search_results = results.into_iter().map(|r| {
            let payload = &r.payload;
            ObservationSearchResult {
                id: payload.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                title: payload.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                obs_type: payload.get("obs_type").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                score: r.score,
                created_at: payload.get("created_at").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            }
        }).collect();

        Ok(search_results)
    }
}
