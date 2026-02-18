//! # CWA Embedding
//!
//! Vector embeddings via Ollama and semantic search via Qdrant for CWA.
//!
//! Provides memory indexing, embedding generation, and similarity search.

pub mod hybrid;
pub mod ollama;
pub mod qdrant;
pub mod memory;
pub mod search;
pub mod observation;
pub mod domain_object;

pub use hybrid::{FusionAlgo, HybridSearchRequest, HybridSearchResult, hybrid_search};
pub use ollama::OllamaClient;
pub use qdrant::{QdrantStore, FILES_COLLECTION};
pub use memory::{MemoryPipeline, MemoryType, AddMemoryResult};
pub use search::{SemanticSearch, SemanticSearchResult};
pub use observation::{ObservationPipeline, AddObservationResult, ObservationSearchResult};
pub use domain_object::{DomainObjectPipeline, DomainObjectSearchResult};
