//! Sandboxed bash executor.

use std::time::Duration;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use tokio::process::Command;
use tokio::time::timeout;

use super::types::ToolOutcome;
use super::Tool;
use crate::config::ToolsConfig;
use crate::error::{AppError, AppResult};
use crate::utils::sanitize::{check_command, SanitizeOutcome};

/// Bash executor.
pub struct TerminalTool {
    cfg: ToolsConfig,
}

impl TerminalTool {
    /// Creates a new terminal tool.
    pub fn new(cfg: ToolsConfig) -> Self {
        Self { cfg }
    }
}

#[derive(Debug, Deserialize)]
struct Args {
    command: String,
}

#[async_trait]
impl Tool for TerminalTool {
    fn name(&self) -> &'static str {
        "terminal"
    }
    fn description(&self) -> &'static str {
        "Runs a single bash command inside the workspace sandbox. Output \
         and stderr are returned along with the exit code."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "Single bash command line to execute"
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, params: serde_json::Value) -> AppResult<ToolOutcome> {
        let Args { command } = serde_json::from_value(params)
            .map_err(|e| AppError::Tool(format!("invalid terminal args: {e}")))?;

        if let SanitizeOutcome::Rejected(pattern) = check_command(&command) {
            return Err(AppError::Forbidden(format!(
                "terminal command rejected by sanitiser (matched {pattern:?})"
            )));
        }

        std::fs::create_dir_all(&self.cfg.workspace_dir)
            .map_err(|e| AppError::Tool(format!("could not create workspace dir: {e}")))?;

        let mut cmd = Command::new("bash");
        cmd.arg("-c")
            .arg(&command)
            .current_dir(&self.cfg.workspace_dir)
            .kill_on_drop(true)
            .env("PATH", std::env::var("PATH").unwrap_or_default())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let child = cmd
            .spawn()
            .map_err(|e| AppError::Tool(format!("failed to spawn bash: {e}")))?;

        let timeout_dur = Duration::from_secs(self.cfg.terminal_timeout_secs.max(1));

        let output = match timeout(timeout_dur, child.wait_with_output()).await {
            Ok(Ok(o)) => o,
            Ok(Err(e)) => {
                return Err(AppError::Tool(format!("terminal io error: {e}")));
            }
            Err(_) => {
                return Ok(ToolOutcome::Timeout {
                    duration_ms: timeout_dur.as_millis() as u64,
                });
            }
        };

        Ok(ToolOutcome::Text {
            output: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            exit_code: output.status.code(),
            duration_ms: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn tool() -> (TerminalTool, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let cfg = ToolsConfig {
            workspace_dir: PathBuf::from(dir.path()),
            terminal_timeout_secs: 5,
            allow_web_search: false,
            allow_http_fetch: false,
            http_fetch_allowlist: vec![],
        };
        (TerminalTool::new(cfg), dir)
    }

    #[tokio::test]
    async fn echo_works() {
        let (t, _dir) = tool();
        let outcome = t
            .execute(serde_json::json!({ "command": "echo hello" }))
            .await
            .unwrap();
        match outcome {
            ToolOutcome::Text {
                output, exit_code, ..
            } => {
                assert!(output.contains("hello"));
                assert_eq!(exit_code, Some(0));
            }
            other => panic!("unexpected outcome: {other:?}"),
        }
    }

    #[tokio::test]
    async fn rm_rf_root_is_rejected() {
        let (t, _dir) = tool();
        let err = t
            .execute(serde_json::json!({ "command": "rm -rf /" }))
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Forbidden(_)));
    }
}
