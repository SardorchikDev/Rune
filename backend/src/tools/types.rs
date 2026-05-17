//! Public types used by the tool registry.

use serde::{Deserialize, Serialize};

/// Lightweight descriptor surfaced to the dashboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDescriptor {
    /// Tool name.
    pub name: String,
    /// Human-readable description.
    pub description: String,
}

/// Wrapper around an invocation request — used in tests and the REST API
/// endpoint that lets callers run a tool ad-hoc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInvocation {
    /// Tool name (must be registered).
    pub name: String,
    /// JSON arguments matching the tool's parameter schema.
    pub arguments: serde_json::Value,
}

/// Outcome of a tool execution. Wrapped in the `WsEvent::ToolResult` for the
/// dashboard and inserted into the agent context as a `Role::Tool` message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ToolOutcome {
    /// Standard textual output.
    Text {
        /// Combined stdout / human-readable summary.
        output: String,
        /// stderr (terminal) or supplementary context.
        #[serde(default)]
        stderr: String,
        /// Optional exit code (terminal).
        #[serde(default)]
        exit_code: Option<i32>,
        /// Duration in milliseconds.
        duration_ms: u64,
    },
    /// Structured JSON output (web search results, file listings, …).
    Structured {
        /// JSON value.
        value: serde_json::Value,
        /// Duration in milliseconds.
        duration_ms: u64,
    },
    /// Tool timed out before completing.
    Timeout {
        /// Duration after which we gave up.
        duration_ms: u64,
    },
    /// Tool returned an error.
    Error {
        /// Error description.
        message: String,
        /// Duration in milliseconds.
        duration_ms: u64,
    },
}

impl ToolOutcome {
    /// Returns a human-readable summary of the outcome for log writing /
    /// dashboard display.
    pub fn summary(&self) -> String {
        match self {
            ToolOutcome::Text { output, stderr, exit_code, .. } => {
                let mut buf = String::new();
                if let Some(code) = exit_code {
                    buf.push_str(&format!("[exit {code}] "));
                }
                buf.push_str(output);
                if !stderr.is_empty() {
                    buf.push_str("\n[stderr]\n");
                    buf.push_str(stderr);
                }
                buf
            }
            ToolOutcome::Structured { value, .. } => {
                serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
            }
            ToolOutcome::Timeout { duration_ms } => {
                format!("timeout after {duration_ms} ms")
            }
            ToolOutcome::Error { message, .. } => message.clone(),
        }
    }

    /// Returns the exit code (if known).
    pub fn exit_code(&self) -> Option<i32> {
        match self {
            ToolOutcome::Text { exit_code, .. } => *exit_code,
            ToolOutcome::Error { .. } => Some(-1),
            ToolOutcome::Timeout { .. } => Some(-2),
            ToolOutcome::Structured { .. } => Some(0),
        }
    }

    /// Returns the elapsed time in milliseconds.
    pub fn duration_ms(&self) -> u64 {
        match self {
            ToolOutcome::Text { duration_ms, .. }
            | ToolOutcome::Structured { duration_ms, .. }
            | ToolOutcome::Timeout { duration_ms }
            | ToolOutcome::Error { duration_ms, .. } => *duration_ms,
        }
    }

    /// Mutates the duration field. Used by the tool registry to record the
    /// wall-clock time spent on dispatch.
    pub fn set_duration_ms(&mut self, ms: u64) {
        match self {
            ToolOutcome::Text { duration_ms, .. }
            | ToolOutcome::Structured { duration_ms, .. }
            | ToolOutcome::Timeout { duration_ms }
            | ToolOutcome::Error { duration_ms, .. } => *duration_ms = ms,
        }
    }
}
