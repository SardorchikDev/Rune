//! Memory subsystem: short-lived episodic log + long-lived semantic store.

pub mod embedder;
pub mod episodic;
pub mod semantic;

use std::sync::Arc;

use sqlx::SqlitePool;

use crate::config::RuneConfig;
use crate::core::llm::LlmRouter;
use crate::error::AppResult;

pub use embedder::Embedder;
pub use episodic::EpisodicMemory;
pub use semantic::{MemoryItem, SemanticMemory, VectorBackend};

/// Top-level memory façade. Owns both the episodic log and the semantic
/// store; exposes a single high-level API used by the agent loop.
pub struct MemoryStore {
    /// Volatile in-memory log of recent interactions.
    pub episodic: Arc<EpisodicMemory>,
    /// Persistent semantic vector store.
    pub semantic: Arc<SemanticMemory>,
    /// Configured top-K for recall.
    pub top_k: usize,
}

impl MemoryStore {
    /// Builds a memory store from configuration.
    pub async fn from_config(
        cfg: &RuneConfig,
        db: SqlitePool,
        router: Arc<LlmRouter>,
    ) -> AppResult<Arc<Self>> {
        let embedder = Arc::new(Embedder::new(
            router,
            cfg.memory.embedding_provider.clone(),
            cfg.memory.embedding_model.clone(),
            cfg.memory.embedding_dim,
        ));
        let backend = if cfg.memory.vector_backend == "qdrant" {
            Arc::new(VectorBackend::qdrant(
                cfg.memory.qdrant_url.clone(),
                cfg.memory.collection_name.clone(),
                cfg.memory.embedding_dim,
            ))
        } else {
            Arc::new(VectorBackend::in_memory(cfg.memory.embedding_dim))
        };
        let semantic = Arc::new(SemanticMemory::new(db, embedder, backend));
        let episodic = Arc::new(EpisodicMemory::new(256));
        Ok(Arc::new(Self {
            episodic,
            semantic,
            top_k: cfg.memory.top_k,
        }))
    }

    /// Records a memory derived from a finished task.
    pub async fn store(&self, task_id: &str, content: &str) -> AppResult<()> {
        self.semantic.store(task_id, content).await?;
        Ok(())
    }

    /// Recalls semantically similar memories. Returns at most `top_k`
    /// content strings.
    pub async fn recall(&self, query: &str) -> AppResult<Vec<String>> {
        let items = self.semantic.search(query, self.top_k).await?;
        Ok(items.into_iter().map(|i| i.content).collect())
    }
}
