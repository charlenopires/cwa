//! Semantic similarity search.
//!
//! Converts a text query to an embedding via Ollama, then searches
//! Qdrant for the most similar stored vectors.

use anyhow::{Context, Result};
use serde::Serialize;
use tracing::debug;

use crate::ollama::OllamaClient;
use crate::qdrant::{QdrantStore, MEMORIES_COLLECTION};

/// A semantic search result with memory content and similarity score.
#[derive(Debug, Clone, Serialize)]
pub struct SemanticSearchResult {
    pub id: String,
    pub content: String,
    pub entry_type: String,
    pub context: String,
    pub score: f32,
    pub created_at: String,
}

/// Semantic search engine combining Ollama embeddings with Qdrant vector search.
pub struct SemanticSearch {
    ollama: OllamaClient,
    qdrant: QdrantStore,
}

impl SemanticSearch {
    /// Create a new semantic search engine.
    pub fn new(ollama: OllamaClient, qdrant: QdrantStore) -> Self {
        Self { ollama, qdrant }
    }

    /// Create with default client configurations.
    pub fn default_search() -> Result<Self> {
        Ok(Self {
            ollama: OllamaClient::default_client(),
            qdrant: QdrantStore::default_store()?,
        })
    }

    /// Search memories by semantic similarity.
    pub async fn search(
        &self,
        query: &str,
        top_k: u64,
    ) -> Result<Vec<SemanticSearchResult>> {
        // Generate embedding for the query
        let query_vector = self.ollama.embed(query).await
            .context("Failed to embed search query")?;

        debug!(query, dim = query_vector.len(), "Generated query embedding");

        // Search Qdrant
        let results = self.qdrant.search(MEMORIES_COLLECTION, query_vector, top_k).await
            .context("Failed to search Qdrant")?;

        // Map to semantic search results
        let search_results = results.into_iter().map(|r| {
            let payload = &r.payload;
            SemanticSearchResult {
                id: payload.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                content: payload.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                entry_type: payload.get("entry_type").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                context: payload.get("context").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                score: r.score,
                created_at: payload.get("created_at").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            }
        }).collect();

        Ok(search_results)
    }

    /// Search memories filtered by project.
    pub async fn search_project(
        &self,
        query: &str,
        project_id: &str,
        top_k: u64,
    ) -> Result<Vec<SemanticSearchResult>> {
        let query_vector = self.ollama.embed(query).await
            .context("Failed to embed search query")?;

        let results = self.qdrant.search_filtered(
            MEMORIES_COLLECTION,
            query_vector,
            top_k,
            project_id,
        ).await.context("Failed to search Qdrant")?;

        let search_results = results.into_iter().map(|r| {
            let payload = &r.payload;
            SemanticSearchResult {
                id: payload.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                content: payload.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                entry_type: payload.get("entry_type").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                context: payload.get("context").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                score: r.score,
                created_at: payload.get("created_at").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            }
        }).collect();

        Ok(search_results)
    }
}
