//! # CWA Embedding
//!
//! Vector embeddings via Ollama and semantic search via Qdrant for CWA.
//!
//! Provides memory indexing, embedding generation, and similarity search.

pub mod ollama;
pub mod qdrant;
pub mod memory;
pub mod search;

pub use ollama::OllamaClient;
pub use qdrant::QdrantStore;
pub use memory::{MemoryPipeline, MemoryType, AddMemoryResult};
pub use search::{SemanticSearch, SemanticSearchResult};
