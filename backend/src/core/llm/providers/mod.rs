//! Concrete LLM provider implementations. Each module implements
//! [`crate::core::llm::LlmProvider`] for one upstream service.

pub mod anthropic;
pub mod fireworks;
pub mod gemini;
pub mod groq;
pub mod ollama;
pub mod openai;
pub mod openai_compatible;
pub mod openrouter;

use crate::core::llm::types::ToolCall;

/// Best-effort parser for inline tool calls. LLMs that don't natively emit
/// `tool_calls` (e.g. Gemini through the text-only interface) frequently fall
/// back to writing a fenced JSON block with a language hint of `tool`,
/// `tool_call`, or `json`. This helper extracts any number of such fenced
/// blocks and returns them as concrete [`ToolCall`] structs. Malformed blocks
/// are silently skipped.
pub fn extract_inline_tool_calls(content: &str, provider: &str) -> Vec<ToolCall> {
    let mut out = Vec::new();
    let mut counter = 0usize;
    for hint in ["```tool_call", "```tool", "```json"] {
        let mut rest = content;
        while let Some(idx) = rest.find(hint) {
            let after_hint = &rest[idx + hint.len()..];
            let Some(end_idx) = after_hint.find("```") else { break };
            let block = after_hint[..end_idx].trim();
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(block) {
                if let (Some(name), Some(args)) = (
                    value.get("name").and_then(|v| v.as_str()),
                    value.get("arguments").cloned(),
                ) {
                    counter += 1;
                    out.push(ToolCall {
                        id: format!("{provider}-call-{counter}"),
                        name: name.to_string(),
                        arguments: args,
                    });
                }
            }
            rest = &after_hint[end_idx + 3..];
        }
    }
    out
}

use std::sync::Arc;
use std::time::Duration;

use crate::config::RuneConfig;
use crate::core::llm::LlmProvider;
use crate::error::{AppError, AppResult};

pub use anthropic::AnthropicProvider;
pub use fireworks::FireworksProvider;
pub use gemini::GeminiProvider;
pub use groq::GroqProvider;
pub use ollama::OllamaProvider;
pub use openai::OpenAiProvider;
pub use openrouter::OpenRouterProvider;

/// Builds a `reqwest::Client` with sensible timeouts for an LLM provider.
pub fn build_http_client(timeout_secs: u64) -> AppResult<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .user_agent(concat!("rune/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(AppError::from)
}

/// Constructs the full set of provider implementations described in the
/// given configuration.
pub fn build_all(cfg: &RuneConfig) -> AppResult<Vec<Arc<dyn LlmProvider>>> {
    let client = build_http_client(cfg.llm.timeout_secs)?;
    let providers: Vec<Arc<dyn LlmProvider>> = vec![
        Arc::new(GeminiProvider::new(client.clone(), cfg.llm.providers.gemini.clone())),
        Arc::new(GroqProvider::new(client.clone(), cfg.llm.providers.groq.clone())),
        Arc::new(OpenRouterProvider::new(client.clone(), cfg.llm.providers.openrouter.clone())),
        Arc::new(FireworksProvider::new(client.clone(), cfg.llm.providers.fireworks.clone())),
        Arc::new(AnthropicProvider::new(client.clone(), cfg.llm.providers.anthropic.clone())),
        Arc::new(OpenAiProvider::new(client.clone(), cfg.llm.providers.openai.clone())),
        Arc::new(OllamaProvider::new(client, cfg.llm.providers.ollama.clone())),
    ];
    Ok(providers)
}
