//! Shared logic for providers that speak the OpenAI Chat Completions API:
//! Groq, OpenRouter, Fireworks AI, OpenAI itself and (in chat mode) Ollama.

use futures::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::core::llm::router::sse_token_stream;
use crate::core::llm::types::{LlmRequest, LlmResponse, Role, TokenStream, ToolCall};
use crate::error::{AppError, AppResult};

/// Common settings for an OpenAI-compatible provider.
pub struct OpenAiCompatibleClient {
    /// HTTP client (already configured with timeouts and user-agent).
    pub http: reqwest::Client,
    /// Provider identifier (e.g. `"openai"`).
    pub provider_name: &'static str,
    /// Base URL ending in `/v1` (or equivalent).
    pub base_url: String,
    /// API key.
    pub api_key: String,
}

impl OpenAiCompatibleClient {
    /// Synchronous chat completion.
    pub async fn complete(&self, req: LlmRequest) -> AppResult<LlmResponse> {
        let body = build_body(&req, false);
        let resp = self
            .http
            .post(format!(
                "{}/chat/completions",
                self.base_url.trim_end_matches('/')
            ))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| AppError::Llm(format!("{} http error: {e}", self.provider_name)))?
            .json::<OpenAiChatResponse>()
            .await?;

        let choice = resp
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| AppError::Llm(format!("{}: empty choices", self.provider_name)))?;

        let mut tool_calls = Vec::new();
        if let Some(calls) = choice.message.tool_calls {
            for c in calls {
                let arguments =
                    serde_json::from_str(&c.function.arguments).unwrap_or_else(|_| json!({}));
                tool_calls.push(ToolCall {
                    id: c.id,
                    name: c.function.name,
                    arguments,
                });
            }
        }

        let content = choice.message.content.unwrap_or_default();
        if tool_calls.is_empty() {
            tool_calls = super::extract_inline_tool_calls(&content, self.provider_name);
        }

        Ok(LlmResponse {
            content,
            tool_calls,
            input_tokens: resp.usage.as_ref().map(|u| u.prompt_tokens).unwrap_or(0),
            output_tokens: resp
                .usage
                .as_ref()
                .map(|u| u.completion_tokens)
                .unwrap_or(0),
            provider: self.provider_name.into(),
            model: req.model,
        })
    }

    /// Streaming chat completion.
    pub async fn stream(&self, req: LlmRequest) -> AppResult<TokenStream> {
        let body = build_body(&req, true);
        let resp = self
            .http
            .post(format!(
                "{}/chat/completions",
                self.base_url.trim_end_matches('/')
            ))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| AppError::Llm(format!("{} http error: {e}", self.provider_name)))?;

        let stream = sse_token_stream(resp, |payload| {
            if payload == "[DONE]" {
                return None;
            }
            let parsed: Result<OpenAiStreamChunk, _> = serde_json::from_str(payload);
            match parsed {
                Ok(chunk) => {
                    let mut buf = String::new();
                    for choice in &chunk.choices {
                        if let Some(content) = &choice.delta.content {
                            buf.push_str(content);
                        }
                    }
                    if buf.is_empty() {
                        None
                    } else {
                        Some(buf)
                    }
                }
                Err(_) => None,
            }
        });

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        tokio::spawn(async move {
            tokio::pin!(stream);
            while let Some(item) = stream.next().await {
                if tx.send(item).is_err() {
                    break;
                }
            }
        });
        Ok(Box::pin(UnboundedReceiverStream::new(rx)))
    }

    /// OpenAI-style embeddings (`/embeddings` endpoint).
    pub async fn embed(&self, model: &str, text: &str) -> AppResult<Vec<f32>> {
        let body = json!({ "model": model, "input": text });
        let resp = self
            .http
            .post(format!(
                "{}/embeddings",
                self.base_url.trim_end_matches('/')
            ))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| AppError::Llm(format!("{} embed error: {e}", self.provider_name)))?
            .json::<OpenAiEmbeddingResponse>()
            .await?;
        resp.data
            .into_iter()
            .next()
            .map(|e| e.embedding)
            .ok_or_else(|| AppError::Llm(format!("{}: empty embedding", self.provider_name)))
    }
}

fn build_body(req: &LlmRequest, stream: bool) -> serde_json::Value {
    let messages: Vec<_> = req
        .messages
        .iter()
        .map(|m| {
            let role_str = match m.role {
                Role::System => "system",
                Role::User => "user",
                Role::Assistant => "assistant",
                Role::Tool => "tool",
            };
            let mut entry = serde_json::Map::new();
            entry.insert("role".into(), json!(role_str));
            entry.insert("content".into(), json!(m.content));
            if let Some(id) = &m.tool_call_id {
                entry.insert("tool_call_id".into(), json!(id));
            }
            if let Some(name) = &m.name {
                entry.insert("name".into(), json!(name));
            }
            serde_json::Value::Object(entry)
        })
        .collect();
    let mut body = json!({
        "model": req.model,
        "messages": messages,
        "stream": stream,
    });
    if let Some(t) = req.temperature {
        body["temperature"] = json!(t);
    }
    if let Some(m) = req.max_tokens {
        body["max_tokens"] = json!(m);
    }
    if let Some(tools) = &req.tools {
        let openai_tools: Vec<_> = tools
            .iter()
            .map(|t| {
                json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters,
                    }
                })
            })
            .collect();
        body["tools"] = json!(openai_tools);
        body["tool_choice"] = json!("auto");
    }
    body
}

#[derive(Debug, Deserialize)]
struct OpenAiChatResponse {
    choices: Vec<OpenAiChoice>,
    #[serde(default)]
    usage: Option<OpenAiUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessage,
}

#[derive(Debug, Deserialize)]
struct OpenAiMessage {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<OpenAiToolCall>>,
}

#[derive(Debug, Deserialize)]
struct OpenAiToolCall {
    id: String,
    function: OpenAiToolFn,
}

#[derive(Debug, Deserialize)]
struct OpenAiToolFn {
    name: String,
    arguments: String,
}

#[derive(Debug, Deserialize)]
struct OpenAiUsage {
    #[serde(default)]
    prompt_tokens: u32,
    #[serde(default)]
    completion_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct OpenAiStreamChunk {
    choices: Vec<OpenAiStreamChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAiStreamChoice {
    delta: OpenAiStreamDelta,
}

#[derive(Debug, Deserialize, Serialize)]
struct OpenAiStreamDelta {
    #[serde(default)]
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiEmbeddingResponse {
    data: Vec<OpenAiEmbedding>,
}

#[derive(Debug, Deserialize)]
struct OpenAiEmbedding {
    embedding: Vec<f32>,
}
