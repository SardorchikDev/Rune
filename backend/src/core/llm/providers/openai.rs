//! OpenAI provider.

use async_trait::async_trait;

use super::openai_compatible::OpenAiCompatibleClient;
use crate::config::OpenAiProviderConfig;
use crate::core::llm::types::{LlmRequest, LlmResponse, TokenStream};
use crate::core::llm::LlmProvider;
use crate::error::{AppError, AppResult};

/// OpenAI GPT-* provider.
pub struct OpenAiProvider {
    client: OpenAiCompatibleClient,
    cfg: OpenAiProviderConfig,
}

impl OpenAiProvider {
    /// Creates a new OpenAI provider.
    pub fn new(http: reqwest::Client, cfg: OpenAiProviderConfig) -> Self {
        let client = OpenAiCompatibleClient {
            http,
            provider_name: "openai",
            base_url: cfg.base_url.clone(),
            api_key: cfg.api_key.clone(),
        };
        Self { client, cfg }
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    fn name(&self) -> &'static str { "openai" }
    fn supported_models(&self) -> Vec<String> { vec![self.cfg.default_model.clone()] }
    fn is_available(&self) -> bool { !self.cfg.api_key.is_empty() }

    async fn complete(&self, req: LlmRequest) -> AppResult<LlmResponse> {
        ensure_key(&self.cfg.api_key)?;
        self.client.complete(req).await
    }

    async fn stream(&self, req: LlmRequest) -> AppResult<TokenStream> {
        ensure_key(&self.cfg.api_key)?;
        self.client.stream(req).await
    }

    async fn embed(&self, text: &str) -> AppResult<Vec<f32>> {
        ensure_key(&self.cfg.api_key)?;
        self.client.embed("text-embedding-3-small", text).await
    }
}

fn ensure_key(k: &str) -> AppResult<()> {
    if k.is_empty() {
        Err(AppError::Llm("openai: api_key is empty".into()))
    } else {
        Ok(())
    }
}
