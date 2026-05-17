//! Google Gemini provider.

use async_trait::async_trait;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::config::GeminiProviderConfig;
use crate::core::llm::types::{
    ChatMessage, LlmRequest, LlmResponse, Role, TokenStream, ToolCall,
};
use crate::core::llm::LlmProvider;
use crate::error::{AppError, AppResult};

/// Concrete implementation of [`LlmProvider`] backed by Google's
/// `generativelanguage.googleapis.com` API.
pub struct GeminiProvider {
    http: reqwest::Client,
    cfg: GeminiProviderConfig,
}

impl GeminiProvider {
    /// Creates a new provider.
    pub fn new(http: reqwest::Client, cfg: GeminiProviderConfig) -> Self {
        Self { http, cfg }
    }

    fn endpoint(&self, model: &str, action: &str) -> String {
        format!("{}/models/{}:{}", self.cfg.base_url.trim_end_matches('/'), model, action)
    }

    fn convert_messages(messages: &[ChatMessage]) -> (Option<String>, Vec<GeminiContent>) {
        let mut system: Option<String> = None;
        let mut contents = Vec::new();
        for m in messages {
            match m.role {
                Role::System => {
                    let entry = m.content.clone();
                    system = match system {
                        Some(prev) => Some(format!("{prev}\n\n{entry}")),
                        None => Some(entry),
                    };
                }
                Role::User => contents.push(GeminiContent {
                    role: "user".into(),
                    parts: vec![GeminiPart::Text { text: m.content.clone() }],
                }),
                Role::Assistant => contents.push(GeminiContent {
                    role: "model".into(),
                    parts: vec![GeminiPart::Text { text: m.content.clone() }],
                }),
                Role::Tool => contents.push(GeminiContent {
                    role: "function".into(),
                    parts: vec![GeminiPart::Text {
                        text: m.content.clone(),
                    }],
                }),
            }
        }
        (system, contents)
    }
}

#[async_trait]
impl LlmProvider for GeminiProvider {
    fn name(&self) -> &'static str { "gemini" }

    fn supported_models(&self) -> Vec<String> {
        if self.cfg.models.is_empty() {
            vec!["gemini-2.5-pro".into(), "gemini-2.5-flash".into()]
        } else {
            self.cfg.models.clone()
        }
    }

    fn is_available(&self) -> bool { !self.cfg.api_key.is_empty() }

    async fn complete(&self, req: LlmRequest) -> AppResult<LlmResponse> {
        if self.cfg.api_key.is_empty() {
            return Err(AppError::Llm("gemini: api_key is empty".into()));
        }
        let (system, contents) = Self::convert_messages(&req.messages);
        let mut body = json!({ "contents": contents });
        if let Some(s) = system {
            body["systemInstruction"] = json!({ "parts": [{ "text": s }] });
        }
        let mut gen = serde_json::Map::new();
        if let Some(t) = req.temperature { gen.insert("temperature".into(), json!(t)); }
        if let Some(m) = req.max_tokens { gen.insert("maxOutputTokens".into(), json!(m)); }
        if !gen.is_empty() {
            body["generationConfig"] = serde_json::Value::Object(gen);
        }

        let url = format!("{}?key={}", self.endpoint(&req.model, "generateContent"), self.cfg.api_key);
        let resp = self
            .http
            .post(&url)
            .json(&body)
            .send()
            .await?
            .error_for_status()
            .map_err(map_http_err)?
            .json::<GeminiCompletionResponse>()
            .await?;

        let mut content = String::new();
        for cand in &resp.candidates {
            for part in &cand.content.parts {
                let GeminiPart::Text { text } = part;
                content.push_str(text);
            }
        }

        let usage = resp.usage_metadata.unwrap_or_default();
        let tool_calls = extract_tool_calls(&content);

        Ok(LlmResponse {
            content,
            tool_calls,
            input_tokens: usage.prompt_token_count,
            output_tokens: usage.candidates_token_count,
            provider: "gemini".into(),
            model: req.model,
        })
    }

    async fn stream(&self, req: LlmRequest) -> AppResult<TokenStream> {
        if self.cfg.api_key.is_empty() {
            return Err(AppError::Llm("gemini: api_key is empty".into()));
        }
        let (system, contents) = Self::convert_messages(&req.messages);
        let mut body = json!({ "contents": contents });
        if let Some(s) = system {
            body["systemInstruction"] = json!({ "parts": [{ "text": s }] });
        }
        let mut gen = serde_json::Map::new();
        if let Some(t) = req.temperature { gen.insert("temperature".into(), json!(t)); }
        if let Some(m) = req.max_tokens { gen.insert("maxOutputTokens".into(), json!(m)); }
        if !gen.is_empty() {
            body["generationConfig"] = serde_json::Value::Object(gen);
        }

        let url = format!(
            "{}?alt=sse&key={}",
            self.endpoint(&req.model, "streamGenerateContent"),
            self.cfg.api_key
        );
        let resp = self
            .http
            .post(&url)
            .json(&body)
            .send()
            .await?
            .error_for_status()
            .map_err(map_http_err)?;

        let stream = super::super::router::sse_token_stream(resp, |payload| {
            let parsed: Result<GeminiCompletionResponse, _> = serde_json::from_str(payload);
            match parsed {
                Ok(value) => {
                    let mut buf = String::new();
                    for cand in &value.candidates {
                        for part in &cand.content.parts {
                            let GeminiPart::Text { text } = part;
                            buf.push_str(text);
                        }
                    }
                    if buf.is_empty() { None } else { Some(buf) }
                }
                Err(_) => None,
            }
        });
        // Wrap into TokenStream
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

    async fn embed(&self, text: &str) -> AppResult<Vec<f32>> {
        if self.cfg.api_key.is_empty() {
            return Err(AppError::Llm("gemini: api_key is empty".into()));
        }
        let url = format!(
            "{}/models/text-embedding-004:embedContent?key={}",
            self.cfg.base_url.trim_end_matches('/'),
            self.cfg.api_key
        );
        let body = json!({
            "model": "models/text-embedding-004",
            "content": { "parts": [{ "text": text }] },
        });
        let resp = self
            .http
            .post(&url)
            .json(&body)
            .send()
            .await?
            .error_for_status()
            .map_err(map_http_err)?
            .json::<GeminiEmbedResponse>()
            .await?;
        Ok(resp.embedding.values)
    }
}

fn map_http_err(e: reqwest::Error) -> AppError {
    AppError::Llm(format!("gemini http error: {e}"))
}

fn extract_tool_calls(content: &str) -> Vec<ToolCall> {
    super::extract_inline_tool_calls(content, "gemini")
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiContent {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum GeminiPart {
    Text { text: String },
}

#[derive(Debug, Deserialize)]
struct GeminiCompletionResponse {
    #[serde(default)]
    candidates: Vec<GeminiCandidate>,
    #[serde(rename = "usageMetadata")]
    usage_metadata: Option<GeminiUsage>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: GeminiContent,
}

#[derive(Debug, Default, Deserialize)]
struct GeminiUsage {
    #[serde(rename = "promptTokenCount", default)]
    prompt_token_count: u32,
    #[serde(rename = "candidatesTokenCount", default)]
    candidates_token_count: u32,
}

#[derive(Debug, Deserialize)]
struct GeminiEmbedResponse {
    embedding: GeminiEmbedVector,
}

#[derive(Debug, Deserialize)]
struct GeminiEmbedVector {
    values: Vec<f32>,
}
