//! LLM abstraction layer. Defines the [`LlmProvider`] trait and aggregates
//! all provider implementations under one namespace. The runtime selector +
//! failover logic lives in [`router`].

pub mod metrics;
pub mod providers;
pub mod router;
pub mod types;

use async_trait::async_trait;

pub use router::LlmRouter;
pub use types::{
    ChatMessage, LlmRequest, LlmResponse, Role, StreamChunk, TokenStream, ToolCall, ToolDefinition,
    Usage,
};

use crate::error::AppResult;

/// A pluggable LLM provider. Every concrete provider (Gemini, Groq, …) ships
/// an implementation in [`providers`].
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Short identifier (`"gemini"`, `"groq"`, …).
    fn name(&self) -> &'static str;

    /// List of model identifiers supported by this provider.
    fn supported_models(&self) -> Vec<String>;

    /// Whether the provider is currently usable (has credentials, is enabled).
    fn is_available(&self) -> bool;

    /// Synchronous, non-streaming completion.
    async fn complete(&self, req: LlmRequest) -> AppResult<LlmResponse>;

    /// Streaming completion. The returned stream yields decoded tokens.
    async fn stream(&self, req: LlmRequest) -> AppResult<TokenStream>;

    /// Computes an embedding for the given input text. Providers that do not
    /// support embeddings return `Err(AppError::Llm(...))`.
    async fn embed(&self, text: &str) -> AppResult<Vec<f32>>;
}
