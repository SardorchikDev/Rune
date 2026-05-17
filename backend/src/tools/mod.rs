//! Tool registry and concrete tool implementations.

pub mod file;
pub mod http_fetch;
pub mod terminal;
pub mod types;
pub mod web_search;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;

pub use types::{ToolDescriptor, ToolInvocation, ToolOutcome};

use crate::config::RuneConfig;
use crate::core::llm::types::{ToolCall, ToolDefinition};
use crate::error::{AppError, AppResult};

/// A registered tool. Implementations live in sibling modules.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Stable tool name (matches what the LLM emits).
    fn name(&self) -> &'static str;
    /// Human-readable description shown to the LLM.
    fn description(&self) -> &'static str;
    /// JSON Schema for the tool's parameters.
    fn parameters_schema(&self) -> serde_json::Value;
    /// Executes the tool with the given JSON arguments.
    async fn execute(&self, params: serde_json::Value) -> AppResult<ToolOutcome>;
}

/// The tool registry exposes a small façade over a set of registered tools.
pub struct ToolRegistry {
    tools: HashMap<&'static str, Arc<dyn Tool>>,
}

impl ToolRegistry {
    /// Builds the canonical set of tools from the configuration.
    pub fn from_config(cfg: &RuneConfig) -> AppResult<Self> {
        let mut tools: HashMap<&'static str, Arc<dyn Tool>> = HashMap::new();

        let terminal = Arc::new(terminal::TerminalTool::new(cfg.tools.clone()));
        tools.insert(terminal.name(), terminal);

        let file_tool = Arc::new(file::FileTool::new(cfg.tools.clone()));
        tools.insert(file_tool.name(), file_tool);

        if cfg.tools.allow_web_search {
            let web = Arc::new(web_search::WebSearchTool::new());
            tools.insert(web.name(), web);
        }

        if cfg.tools.allow_http_fetch {
            let http = Arc::new(http_fetch::HttpFetchTool::new(cfg.tools.clone())?);
            tools.insert(http.name(), http);
        }

        Ok(Self { tools })
    }

    /// Returns OpenAI / Anthropic-shaped tool definitions for every
    /// registered tool, suitable for passing to an LLM request.
    pub fn definitions(&self) -> Vec<ToolDefinition> {
        let mut defs: Vec<ToolDefinition> = self
            .tools
            .values()
            .map(|t| ToolDefinition {
                name: t.name().to_string(),
                description: t.description().to_string(),
                parameters: t.parameters_schema(),
            })
            .collect();
        defs.sort_by(|a, b| a.name.cmp(&b.name));
        defs
    }

    /// Returns a stable view of registered tool names.
    pub fn names(&self) -> Vec<&'static str> {
        let mut names: Vec<_> = self.tools.keys().copied().collect();
        names.sort();
        names
    }

    /// Executes a tool call, capturing duration and converting errors into
    /// [`ToolOutcome::Error`] for easier downstream handling.
    pub async fn execute(&self, call: &ToolCall) -> ToolOutcome {
        let started = Instant::now();
        let Some(tool) = self.tools.get(call.name.as_str()) else {
            return ToolOutcome::Error {
                message: format!("unknown tool: {}", call.name),
                duration_ms: started.elapsed().as_millis() as u64,
            };
        };
        match tool.execute(call.arguments.clone()).await {
            Ok(mut outcome) => {
                outcome.set_duration_ms(started.elapsed().as_millis() as u64);
                outcome
            }
            Err(AppError::Forbidden(msg)) => ToolOutcome::Error {
                message: format!("forbidden: {msg}"),
                duration_ms: started.elapsed().as_millis() as u64,
            },
            Err(e) => ToolOutcome::Error {
                message: e.to_string(),
                duration_ms: started.elapsed().as_millis() as u64,
            },
        }
    }
}
