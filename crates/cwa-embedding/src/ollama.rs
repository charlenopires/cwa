//! Ollama HTTP client for embedding generation.
//!
//! Uses the Ollama API at /api/embeddings to generate vectors
//! with the nomic-embed-text model (768 dimensions).

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::debug;

/// Default Ollama API URL.
pub const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434";

/// Default embedding model.
pub const DEFAULT_MODEL: &str = "nomic-embed-text";

/// Expected embedding dimension for nomic-embed-text.
pub const EMBEDDING_DIM: usize = 768;

/// Ollama embedding client.
#[derive(Clone)]
pub struct OllamaClient {
    base_url: String,
    model: String,
    client: reqwest::Client,
}

#[derive(Serialize)]
struct EmbeddingRequest {
    model: String,
    prompt: String,
}

#[derive(Deserialize)]
struct EmbeddingResponse {
    embedding: Vec<f32>,
}

impl OllamaClient {
    /// Create a new Ollama client with specified URL and model.
    pub fn new(base_url: &str, model: &str) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.to_string(),
            client,
        }
    }

    /// Create a client with default settings (localhost:11434, nomic-embed-text).
    pub fn default_client() -> Self {
        Self::new(DEFAULT_OLLAMA_URL, DEFAULT_MODEL)
    }

    /// Generate an embedding vector for the given text.
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let request = EmbeddingRequest {
            model: self.model.clone(),
            prompt: text.to_string(),
        };

        let response = self.client
            .post(format!("{}/api/embeddings", self.base_url))
            .json(&request)
            .send()
            .await
            .context("Failed to connect to Ollama")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Ollama API error ({}): {}", status, body);
        }

        let result: EmbeddingResponse = response
            .json()
            .await
            .context("Failed to parse Ollama response")?;

        debug!(dim = result.embedding.len(), "Generated embedding");

        Ok(result.embedding)
    }

    /// Check if the Ollama service is healthy and the model is available.
    pub async fn health_check(&self) -> Result<bool> {
        let response = self.client
            .get(format!("{}/api/tags", self.base_url))
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                let text = resp.text().await.unwrap_or_default();
                Ok(text.contains(&self.model))
            }
            _ => Ok(false),
        }
    }
}
