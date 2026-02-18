//! Domain object embedding pipeline.
//!
//! Embeds domain objects into Qdrant for semantic search across the domain model.

use anyhow::{Context, Result};
use tracing::{debug, info};

use crate::ollama::OllamaClient;
use crate::qdrant::{QdrantStore, DOMAIN_OBJECTS_COLLECTION};

/// Pipeline for embedding domain objects.
pub struct DomainObjectPipeline {
    ollama: OllamaClient,
    qdrant: QdrantStore,
}

/// A search result from domain object vector search.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DomainObjectSearchResult {
    pub id: String,
    pub name: String,
    pub object_type: String,
    pub context_name: String,
    pub description: String,
    pub score: f32,
}

impl DomainObjectPipeline {
    /// Create a new pipeline with the given clients.
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

    /// Embed a domain object into Qdrant.
    ///
    /// Returns the embedding dimension on success.
    pub async fn embed_domain_object(
        &self,
        project_id: &str,
        obj_id: &str,
        name: &str,
        object_type: &str,
        context_name: &str,
        description: &str,
    ) -> Result<usize> {
        let embed_text = format!("{} ({} in {}): {}", name, object_type, context_name, description);

        let embedding = self.ollama.embed(&embed_text).await
            .context("Failed to generate domain object embedding")?;
        let dim = embedding.len();

        let payload = serde_json::json!({
            "id": obj_id,
            "project_id": project_id,
            "context_name": context_name,
            "name": name,
            "object_type": object_type,
            "description": description,
            "created_at": chrono::Utc::now().to_rfc3339(),
        });

        self.qdrant.upsert(DOMAIN_OBJECTS_COLLECTION, obj_id, embedding, payload).await
            .context("Failed to upsert domain object to Qdrant")?;

        info!(id = %obj_id, name, object_type, dim, "Domain object embedded");

        Ok(dim)
    }

    /// Search domain objects by semantic similarity.
    pub async fn search_domain_objects(
        &self,
        query: &str,
        project_id: &str,
        top_k: u64,
    ) -> Result<Vec<DomainObjectSearchResult>> {
        let query_vector = self.ollama.embed(query).await
            .context("Failed to embed domain object search query")?;

        debug!(query, dim = query_vector.len(), "Generated domain object query embedding");

        let results = self.qdrant.search_filtered(
            DOMAIN_OBJECTS_COLLECTION,
            query_vector,
            top_k,
            project_id,
        ).await.context("Failed to search domain objects in Qdrant")?;

        let search_results = results.into_iter().map(|r| {
            let payload = &r.payload;
            DomainObjectSearchResult {
                id: payload.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                name: payload.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                object_type: payload.get("object_type").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                context_name: payload.get("context_name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                description: payload.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                score: r.score,
            }
        }).collect();

        Ok(search_results)
    }
}
