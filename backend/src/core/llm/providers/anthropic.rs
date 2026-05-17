//! Anthropic (Claude) provider.

use async_trait::async_trait;
use futures::StreamExt;
use serde::Deserialize;
use serde_json::json;
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::config::AnthropicProviderConfig;
use crate::core::llm::router::sse_token_stream;
use crate::core::llm::types::{
    ChatMessage, LlmRequest, LlmResponse, Role, TokenStream, ToolCall,
};
use crate::core::llm::LlmProvider;
use crate::error::{AppError, AppResult};

/// Anthropic Messages API client.
pub struct AnthropicProvider {
    http: reqwest::Client,
    cfg: AnthropicProviderConfig,
}

impl AnthropicProvider {
    /// Creates a new Anthropic provider.
    pub fn new(http: reqwest::Client, cfg: AnthropicProviderConfig) -> Self {
        Self { http, cfg }
    }

    fn convert_messages(messages: &[ChatMessage]) -> (Option<String>, Vec<serde_json::Value>) {
        let mut system: Option<String> = None;
        let mut converted = Vec::new();
        for m in messages {
            match m.role {
                Role::System => {
                    let entry = m.content.clone();
                    system = match system {
                        Some(prev) => Some(format!("{prev}\n\n{entry}")),
                        None => Some(entry),
                    };
                }
                Role::User => {
                    converted.push(json!({ "role": "user", "content": m.content }));
                }
                Role::Assistant => {
                    converted.push(json!({ "role": "assistant", "content": m.content }));
                }
                Role::Tool => {
                    converted.push(json!({
                        "role": "user",
                        "content": [{
                            "type": "tool_result",
                            "tool_use_id": m.tool_call_id.clone().unwrap_or_default(),
                            "content": m.content,
                        }],
                    }));
                }
            }
        }
        (system, converted)
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    fn name(&self) -> &'static str { "anthropic" }
    fn supported_models(&self) -> Vec<String> { vec![self.cfg.default_model.clone()] }
    fn is_available(&self) -> bool { !self.cfg.api_key.is_empty() }

    async fn complete(&self, req: LlmRequest) -> AppResult<LlmResponse> {
        if self.cfg.api_key.is_empty() {
            return Err(AppError::Llm("anthropic: api_key is empty".into()));
        }
        let (system, messages) = Self::convert_messages(&req.messages);
        let mut body = json!({
            "model": req.model,
            "messages": messages,
            "max_tokens": req.max_tokens.unwrap_or(4096),
        });
        if let Some(s) = system { body["system"] = json!(s); }
        if let Some(t) = req.temperature { body["temperature"] = json!(t); }

        let resp = self
            .http
            .post(format!("{}/messages", self.cfg.base_url.trim_end_matches('/')))
            .header("x-api-key", &self.cfg.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| AppError::Llm(format!("anthropic http error: {e}")))?
            .json::<AnthropicResponse>()
            .await?;

        let mut content = String::new();
        let mut tool_calls = Vec::new();
        for block in &resp.content {
            match block {
                AnthropicBlock::Text { text } => content.push_str(text),
                AnthropicBlock::ToolUse { id, name, input } => {
                    tool_calls.push(ToolCall {
                        id: id.clone(),
                        name: name.clone(),
                        arguments: input.clone(),
                    });
                }
            }
        }
        if tool_calls.is_empty() {
            tool_calls = super::extract_inline_tool_calls(&content, "anthropic");
        }

        Ok(LlmResponse {
            content,
            tool_calls,
            input_tokens: resp.usage.input_tokens,
            output_tokens: resp.usage.output_tokens,
            provider: "anthropic".into(),
            model: req.model,
        })
    }

    async fn stream(&self, req: LlmRequest) -> AppResult<TokenStream> {
        if self.cfg.api_key.is_empty() {
            return Err(AppError::Llm("anthropic: api_key is empty".into()));
        }
        let (system, messages) = Self::convert_messages(&req.messages);
        let mut body = json!({
            "model": req.model,
            "messages": messages,
            "max_tokens": req.max_tokens.unwrap_or(4096),
            "stream": true,
        });
        if let Some(s) = system { body["system"] = json!(s); }
        if let Some(t) = req.temperature { body["temperature"] = json!(t); }

        let resp = self
            .http
            .post(format!("{}/messages", self.cfg.base_url.trim_end_matches('/')))
            .header("x-api-key", &self.cfg.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| AppError::Llm(format!("anthropic http error: {e}")))?;

        let stream = sse_token_stream(resp, |payload| {
            if payload == "[DONE]" { return None; }
            let parsed: Result<AnthropicStreamEvent, _> = serde_json::from_str(payload);
            match parsed {
                Ok(AnthropicStreamEvent::ContentBlockDelta { delta }) => match delta {
                    AnthropicDelta::TextDelta { text } => Some(text),
                    _ => None,
                },
                _ => None,
            }
        });

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        tokio::spawn(async move {
            tokio::pin!(stream);
            while let Some(item) = stream.next().await {
                if tx.send(item).is_err() { break; }
            }
        });
        Ok(Box::pin(UnboundedReceiverStream::new(rx)))
    }

    async fn embed(&self, _text: &str) -> AppResult<Vec<f32>> {
        Err(AppError::Llm("anthropic: embedding not supported".into()))
    }
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicBlock>,
    usage: AnthropicUsage,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum AnthropicBlock {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    #[serde(default)]
    input_tokens: u32,
    #[serde(default)]
    output_tokens: u32,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum AnthropicStreamEvent {
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { delta: AnthropicDelta },
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum AnthropicDelta {
    #[serde(rename = "text_delta")]
    TextDelta { text: String },
    #[serde(other)]
    Other,
}
