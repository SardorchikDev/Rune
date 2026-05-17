//! Wire types shared between the LLM router and every provider.

use std::pin::Pin;

use futures::Stream;
use serde::{Deserialize, Serialize};

use crate::error::AppResult;

/// Role of a chat message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// System / instruction-tuning message.
    System,
    /// End-user message.
    User,
    /// Assistant (LLM) message.
    Assistant,
    /// Result of a tool call surfaced back to the LLM.
    Tool,
}

/// A single chat message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Speaker role.
    pub role: Role,
    /// Plain-text content.
    pub content: String,
    /// Optional `tool_call_id` correlating a `Role::Tool` result back to a
    /// specific call emitted by the assistant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// Optional human-readable name for a tool message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl ChatMessage {
    /// Constructs a system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self { role: Role::System, content: content.into(), tool_call_id: None, name: None }
    }
    /// Constructs a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self { role: Role::User, content: content.into(), tool_call_id: None, name: None }
    }
    /// Constructs an assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self { role: Role::Assistant, content: content.into(), tool_call_id: None, name: None }
    }
    /// Constructs a tool-result message tied to a specific `tool_call_id`.
    pub fn tool(content: impl Into<String>, tool_call_id: impl Into<String>) -> Self {
        Self {
            role: Role::Tool,
            content: content.into(),
            tool_call_id: Some(tool_call_id.into()),
            name: None,
        }
    }
}

/// Definition of a tool exposed to the LLM. Mirrors the OpenAI / Anthropic
/// "tools" parameter format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Tool name.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// JSON Schema describing the parameters object.
    pub parameters: serde_json::Value,
}

/// A tool call emitted by the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique identifier (provider-generated or synthesised by Rune).
    pub id: String,
    /// Tool name (must match a registered tool).
    pub name: String,
    /// Parsed JSON arguments object.
    pub arguments: serde_json::Value,
}

/// Token usage / cost accounting for a single call.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    /// Input tokens consumed.
    pub input_tokens: u32,
    /// Output tokens generated.
    pub output_tokens: u32,
}

/// Request to the LLM router. The router will pick the right provider based
/// on `model` (or fall back to the configured default).
#[derive(Debug)]
pub struct LlmRequest {
    /// Messages making up the conversation.
    pub messages: Vec<ChatMessage>,
    /// Specific model identifier, e.g. `"gemini-2.5-pro"`.
    pub model: String,
    /// Whether the caller wants streaming output. The router still returns a
    /// non-streaming response on `complete()`; this hint propagates to
    /// providers that have to set a different endpoint or request flag.
    pub stream: bool,
    /// Optional max output tokens.
    pub max_tokens: Option<u32>,
    /// Optional sampling temperature.
    pub temperature: Option<f32>,
    /// Optional list of tools the LLM may call.
    pub tools: Option<Vec<ToolDefinition>>,
}

impl LlmRequest {
    /// Constructs a basic request with sensible defaults.
    pub fn new(messages: Vec<ChatMessage>, model: impl Into<String>) -> Self {
        Self {
            messages,
            model: model.into(),
            stream: false,
            max_tokens: None,
            temperature: Some(0.2),
            tools: None,
        }
    }

    /// Forces streaming on the request and returns it for builder-style use.
    pub fn streaming(mut self) -> Self {
        self.stream = true;
        self
    }

    /// Attaches a list of tool definitions.
    pub fn with_tools(mut self, tools: Vec<ToolDefinition>) -> Self {
        self.tools = Some(tools);
        self
    }
}

/// Non-streaming completion response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    /// Free-text content emitted by the LLM (may be empty when only tool
    /// calls are produced).
    pub content: String,
    /// Tool calls parsed out of the response (if any).
    pub tool_calls: Vec<ToolCall>,
    /// Input token count.
    pub input_tokens: u32,
    /// Output token count.
    pub output_tokens: u32,
    /// Provider that actually answered (may differ from the request's model
    /// when failover happened).
    pub provider: String,
    /// Model identifier returned by the provider.
    pub model: String,
}

impl LlmResponse {
    /// Returns a [`Usage`] view over the token counts.
    pub fn usage(&self) -> Usage {
        Usage { input_tokens: self.input_tokens, output_tokens: self.output_tokens }
    }
}

/// A single chunk in a streaming response.
pub type StreamChunk = String;

/// Boxed dynamic stream of decoded tokens.
pub type TokenStream = Pin<Box<dyn Stream<Item = AppResult<StreamChunk>> + Send>>;
