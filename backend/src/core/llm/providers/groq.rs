//! Groq provider (OpenAI-compatible API).

use async_trait::async_trait;

use super::openai_compatible::OpenAiCompatibleClient;
use crate::config::GroqProviderConfig;
use crate::core::llm::types::{LlmRequest, LlmResponse, TokenStream};
use crate::core::llm::LlmProvider;
use crate::error::{AppError, AppResult};

/// Groq provider.
pub struct GroqProvider {
    client: OpenAiCompatibleClient,
    cfg: GroqProviderConfig,
}

impl GroqProvider {
    /// Creates a new Groq provider.
    pub fn new(http: reqwest::Client, cfg: GroqProviderConfig) -> Self {
        let client = OpenAiCompatibleClient {
            http,
            provider_name: "groq",
            base_url: cfg.base_url.clone(),
            api_key: cfg.api_key.clone(),
        };
        Self { client, cfg }
    }
}

#[async_trait]
impl LlmProvider for GroqProvider {
    fn name(&self) -> &'static str {
        "groq"
    }
    fn supported_models(&self) -> Vec<String> {
        if self.cfg.models.is_empty() {
            vec!["llama3-70b-8192".into(), "mixtral-8x7b-32768".into()]
        } else {
            self.cfg.models.clone()
        }
    }
    fn is_available(&self) -> bool {
        !self.cfg.api_key.is_empty()
    }

    async fn complete(&self, req: LlmRequest) -> AppResult<LlmResponse> {
        if self.cfg.api_key.is_empty() {
            return Err(AppError::Llm("groq: api_key is empty".into()));
        }
        self.client.complete(req).await
    }

    async fn stream(&self, req: LlmRequest) -> AppResult<TokenStream> {
        if self.cfg.api_key.is_empty() {
            return Err(AppError::Llm("groq: api_key is empty".into()));
        }
        self.client.stream(req).await
    }

    async fn embed(&self, _text: &str) -> AppResult<Vec<f32>> {
        Err(AppError::Llm("groq: embedding not supported".into()))
    }
}
