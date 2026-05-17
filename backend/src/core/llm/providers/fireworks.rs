//! Fireworks AI provider (OpenAI-compatible API).

use async_trait::async_trait;

use super::openai_compatible::OpenAiCompatibleClient;
use crate::config::FireworksProviderConfig;
use crate::core::llm::types::{LlmRequest, LlmResponse, TokenStream};
use crate::core::llm::LlmProvider;
use crate::error::{AppError, AppResult};

/// Fireworks AI provider.
pub struct FireworksProvider {
    client: OpenAiCompatibleClient,
    cfg: FireworksProviderConfig,
}

impl FireworksProvider {
    /// Creates a new Fireworks provider.
    pub fn new(http: reqwest::Client, cfg: FireworksProviderConfig) -> Self {
        let client = OpenAiCompatibleClient {
            http,
            provider_name: "fireworks",
            base_url: cfg.base_url.clone(),
            api_key: cfg.api_key.clone(),
        };
        Self { client, cfg }
    }
}

#[async_trait]
impl LlmProvider for FireworksProvider {
    fn name(&self) -> &'static str {
        "fireworks"
    }
    fn supported_models(&self) -> Vec<String> {
        vec![self.cfg.default_model.clone()]
    }
    fn is_available(&self) -> bool {
        !self.cfg.api_key.is_empty()
    }

    async fn complete(&self, req: LlmRequest) -> AppResult<LlmResponse> {
        if self.cfg.api_key.is_empty() {
            return Err(AppError::Llm("fireworks: api_key is empty".into()));
        }
        self.client.complete(req).await
    }

    async fn stream(&self, req: LlmRequest) -> AppResult<TokenStream> {
        if self.cfg.api_key.is_empty() {
            return Err(AppError::Llm("fireworks: api_key is empty".into()));
        }
        self.client.stream(req).await
    }

    async fn embed(&self, _text: &str) -> AppResult<Vec<f32>> {
        Err(AppError::Llm("fireworks: embedding not supported".into()))
    }
}
