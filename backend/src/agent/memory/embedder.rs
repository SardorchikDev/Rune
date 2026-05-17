//! Embedding helper used by the semantic memory store.

use std::sync::Arc;

use crate::core::llm::LlmRouter;
use crate::error::{AppError, AppResult};

/// Wraps an [`LlmRouter`] to produce embeddings from the configured provider.
pub struct Embedder {
    router: Arc<LlmRouter>,
    provider: String,
    #[allow(dead_code)]
    model: String,
    expected_dim: usize,
}

impl Embedder {
    /// Constructs a new embedder.
    pub fn new(
        router: Arc<LlmRouter>,
        provider: String,
        model: String,
        expected_dim: usize,
    ) -> Self {
        Self { router, provider, model, expected_dim }
    }

    /// Computes an embedding for `text`. If the provider returns a vector of
    /// the wrong dimensionality we surface a clear error.
    pub async fn embed(&self, text: &str) -> AppResult<Vec<f32>> {
        let vector = self.router.embed(&self.provider, text).await?;
        if !vector.is_empty() && vector.len() != self.expected_dim {
            return Err(AppError::Internal(format!(
                "embedder: expected {} dims, got {}",
                self.expected_dim,
                vector.len()
            )));
        }
        Ok(vector)
    }

    /// Computes a deterministic, zero-cost embedding for unit tests. The
    /// vector length matches `expected_dim` and is filled with hashed bytes.
    pub fn embed_deterministic(&self, text: &str) -> Vec<f32> {
        use sha2::{Digest, Sha256};
        let mut out = vec![0.0f32; self.expected_dim];
        let mut hasher = Sha256::new();
        hasher.update(text.as_bytes());
        let hash = hasher.finalize();
        for (i, slot) in out.iter_mut().enumerate() {
            let byte = hash[i % hash.len()] as f32;
            *slot = (byte / 255.0) * 2.0 - 1.0;
        }
        out
    }
}
