//! OpenRouter provider (OpenAI-compatible API).

use async_trait::async_trait;

use super::openai_compatible::OpenAiCompatibleClient;
use crate::config::OpenRouterProviderConfig;
use crate::core::llm::types::{LlmRequest, LlmResponse, TokenStream};
use crate::core::llm::LlmProvider;
use crate::error::{AppError, AppResult};

/// OpenRouter multi-model proxy.
pub struct OpenRouterProvider {
    client: OpenAiCompatibleClient,
    cfg: OpenRouterProviderConfig,
}

impl OpenRouterProvider {
    /// Creates a new OpenRouter provider.
    pub fn new(http: reqwest::Client, cfg: OpenRouterProviderConfig) -> Self {
        let client = OpenAiCompatibleClient {
            http,
            provider_name: "openrouter",
            base_url: cfg.base_url.clone(),
            api_key: cfg.api_key.clone(),
        };
        Self { client, cfg }
    }
}

#[async_trait]
impl LlmProvider for OpenRouterProvider {
    fn name(&self) -> &'static str { "openrouter" }
    fn supported_models(&self) -> Vec<String> { vec![self.cfg.default_model.clone()] }
    fn is_available(&self) -> bool { !self.cfg.api_key.is_empty() }

    async fn complete(&self, req: LlmRequest) -> AppResult<LlmResponse> {
        if self.cfg.api_key.is_empty() {
            return Err(AppError::Llm("openrouter: api_key is empty".into()));
        }
        self.client.complete(req).await
    }

    async fn stream(&self, req: LlmRequest) -> AppResult<TokenStream> {
        if self.cfg.api_key.is_empty() {
            return Err(AppError::Llm("openrouter: api_key is empty".into()));
        }
        self.client.stream(req).await
    }

    async fn embed(&self, _text: &str) -> AppResult<Vec<f32>> {
        Err(AppError::Llm("openrouter: embedding not supported".into()))
    }
}
