//! Local Ollama provider (`http://localhost:11434`).

use async_trait::async_trait;
use futures::StreamExt;
use serde::Deserialize;
use serde_json::json;
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::config::OllamaProviderConfig;
use crate::core::llm::types::{
    ChatMessage, LlmRequest, LlmResponse, Role, StreamChunk, TokenStream,
};
use crate::core::llm::LlmProvider;
use crate::error::{AppError, AppResult};

/// Ollama provider.
pub struct OllamaProvider {
    http: reqwest::Client,
    cfg: OllamaProviderConfig,
}

impl OllamaProvider {
    /// Creates a new Ollama provider.
    pub fn new(http: reqwest::Client, cfg: OllamaProviderConfig) -> Self {
        Self { http, cfg }
    }

    fn convert_messages(messages: &[ChatMessage]) -> Vec<serde_json::Value> {
        messages
            .iter()
            .map(|m| {
                let role = match m.role {
                    Role::System => "system",
                    Role::User => "user",
                    Role::Assistant => "assistant",
                    Role::Tool => "tool",
                };
                json!({ "role": role, "content": m.content })
            })
            .collect()
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    fn name(&self) -> &'static str { "ollama" }
    fn supported_models(&self) -> Vec<String> { vec![self.cfg.default_model.clone()] }
    fn is_available(&self) -> bool { self.cfg.enabled }

    async fn complete(&self, req: LlmRequest) -> AppResult<LlmResponse> {
        if !self.cfg.enabled {
            return Err(AppError::Llm("ollama: provider disabled".into()));
        }
        let messages = Self::convert_messages(&req.messages);
        let body = json!({
            "model": req.model,
            "messages": messages,
            "stream": false,
            "options": {
                "temperature": req.temperature.unwrap_or(0.2),
            }
        });
        let resp = self
            .http
            .post(format!("{}/api/chat", self.cfg.base_url.trim_end_matches('/')))
            .json(&body)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| AppError::Llm(format!("ollama http error: {e}")))?
            .json::<OllamaResponse>()
            .await?;

        let content = resp.message.content.clone();
        let tool_calls = super::extract_inline_tool_calls(&content, "ollama");

        Ok(LlmResponse {
            content,
            tool_calls,
            input_tokens: resp.prompt_eval_count.unwrap_or(0),
            output_tokens: resp.eval_count.unwrap_or(0),
            provider: "ollama".into(),
            model: req.model,
        })
    }

    async fn stream(&self, req: LlmRequest) -> AppResult<TokenStream> {
        if !self.cfg.enabled {
            return Err(AppError::Llm("ollama: provider disabled".into()));
        }
        let messages = Self::convert_messages(&req.messages);
        let body = json!({
            "model": req.model,
            "messages": messages,
            "stream": true,
            "options": {
                "temperature": req.temperature.unwrap_or(0.2),
            }
        });
        let resp = self
            .http
            .post(format!("{}/api/chat", self.cfg.base_url.trim_end_matches('/')))
            .json(&body)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| AppError::Llm(format!("ollama http error: {e}")))?;

        // Ollama streams newline-delimited JSON objects rather than SSE.
        let mut byte_stream = resp.bytes_stream();
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<AppResult<StreamChunk>>();
        tokio::spawn(async move {
            let mut buf = String::new();
            while let Some(chunk) = byte_stream.next().await {
                match chunk {
                    Ok(bytes) => {
                        buf.push_str(&String::from_utf8_lossy(&bytes));
                        while let Some(idx) = buf.find('\n') {
                            let line = buf[..idx].to_string();
                            buf.drain(..=idx);
                            if line.trim().is_empty() { continue; }
                            if let Ok(value) = serde_json::from_str::<OllamaResponse>(&line) {
                                if !value.message.content.is_empty()
                                    && tx.send(Ok(value.message.content)).is_err()
                                {
                                    return;
                                }
                                if value.done.unwrap_or(false) { return; }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(AppError::Llm(format!("ollama stream error: {e}"))));
                        return;
                    }
                }
            }
        });
        Ok(Box::pin(UnboundedReceiverStream::new(rx)))
    }

    async fn embed(&self, text: &str) -> AppResult<Vec<f32>> {
        if !self.cfg.enabled {
            return Err(AppError::Llm("ollama: provider disabled".into()));
        }
        let body = json!({ "model": "nomic-embed-text", "prompt": text });
        let resp = self
            .http
            .post(format!("{}/api/embeddings", self.cfg.base_url.trim_end_matches('/')))
            .json(&body)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| AppError::Llm(format!("ollama embed error: {e}")))?
            .json::<OllamaEmbedResponse>()
            .await?;
        Ok(resp.embedding)
    }
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    message: OllamaMessage,
    #[serde(default)]
    done: Option<bool>,
    #[serde(default)]
    prompt_eval_count: Option<u32>,
    #[serde(default)]
    eval_count: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct OllamaMessage {
    #[serde(default)]
    content: String,
}

#[derive(Debug, Deserialize)]
struct OllamaEmbedResponse {
    embedding: Vec<f32>,
}
