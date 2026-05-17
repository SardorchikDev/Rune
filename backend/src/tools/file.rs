//! Sandboxed file operations.

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use super::types::ToolOutcome;
use super::Tool;
use crate::config::ToolsConfig;
use crate::error::{AppError, AppResult};

/// File operations restricted to the workspace directory.
pub struct FileTool {
    cfg: ToolsConfig,
}

impl FileTool {
    /// Creates a new file tool.
    pub fn new(cfg: ToolsConfig) -> Self {
        Self { cfg }
    }

    /// Resolves `path` against the workspace root and asserts that the
    /// canonical path does not escape the sandbox.
    pub fn safe_path(&self, path: &str) -> AppResult<PathBuf> {
        let workspace = std::fs::canonicalize(&self.cfg.workspace_dir).or_else(|_| {
            std::fs::create_dir_all(&self.cfg.workspace_dir)
                .map_err(|e| AppError::Tool(format!("workspace dir: {e}")))?;
            std::fs::canonicalize(&self.cfg.workspace_dir)
                .map_err(|e| AppError::Tool(format!("workspace dir: {e}")))
        })?;
        let candidate = workspace.join(path);
        // Manually resolve `..` without requiring the file to exist.
        let normalised = normalise(&candidate);
        if !normalised.starts_with(&workspace) {
            return Err(AppError::Forbidden(format!(
                "path escape detected: {} resolves outside workspace",
                path
            )));
        }
        Ok(normalised)
    }
}

fn normalise(p: &Path) -> PathBuf {
    let mut stack: Vec<std::ffi::OsString> = Vec::new();
    for comp in p.components() {
        match comp {
            std::path::Component::ParentDir => {
                stack.pop();
            }
            std::path::Component::CurDir => {}
            other => stack.push(other.as_os_str().to_owned()),
        }
    }
    let mut out = PathBuf::new();
    for s in stack {
        out.push(s);
    }
    out
}

#[derive(Debug, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
enum Args {
    Read { path: String },
    Write { path: String, content: String },
    List { path: String },
    Delete { path: String },
}

#[async_trait]
impl Tool for FileTool {
    fn name(&self) -> &'static str {
        "file"
    }
    fn description(&self) -> &'static str {
        "Reads, writes, lists or deletes files inside the workspace sandbox. \
         All paths are resolved relative to the workspace root and are not \
         allowed to escape it."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["read", "write", "list", "delete"]
                },
                "path": { "type": "string" },
                "content": { "type": "string" }
            },
            "required": ["operation", "path"]
        })
    }

    async fn execute(&self, params: serde_json::Value) -> AppResult<ToolOutcome> {
        let args: Args = serde_json::from_value(params)
            .map_err(|e| AppError::Tool(format!("invalid file args: {e}")))?;

        match args {
            Args::Read { path } => {
                let p = self.safe_path(&path)?;
                let content = tokio::fs::read_to_string(&p)
                    .await
                    .map_err(|e| AppError::Tool(format!("read {}: {e}", p.display())))?;
                Ok(ToolOutcome::Text {
                    output: content,
                    stderr: String::new(),
                    exit_code: Some(0),
                    duration_ms: 0,
                })
            }
            Args::Write { path, content } => {
                let p = self.safe_path(&path)?;
                if let Some(parent) = p.parent() {
                    tokio::fs::create_dir_all(parent).await.ok();
                }
                tokio::fs::write(&p, content.as_bytes())
                    .await
                    .map_err(|e| AppError::Tool(format!("write {}: {e}", p.display())))?;
                Ok(ToolOutcome::Text {
                    output: format!("wrote {} bytes to {}", content.len(), path),
                    stderr: String::new(),
                    exit_code: Some(0),
                    duration_ms: 0,
                })
            }
            Args::List { path } => {
                let p = self.safe_path(&path)?;
                let mut entries: Vec<serde_json::Value> = Vec::new();
                let mut rd = tokio::fs::read_dir(&p)
                    .await
                    .map_err(|e| AppError::Tool(format!("list {}: {e}", p.display())))?;
                while let Some(entry) = rd
                    .next_entry()
                    .await
                    .map_err(|e| AppError::Tool(format!("list iter: {e}")))?
                {
                    let ft = entry.file_type().await.ok();
                    entries.push(json!({
                        "name": entry.file_name().to_string_lossy(),
                        "is_dir": ft.as_ref().map(|t| t.is_dir()).unwrap_or(false),
                    }));
                }
                Ok(ToolOutcome::Structured {
                    value: json!({ "entries": entries }),
                    duration_ms: 0,
                })
            }
            Args::Delete { path } => {
                let p = self.safe_path(&path)?;
                if p.is_dir() {
                    tokio::fs::remove_dir_all(&p)
                        .await
                        .map_err(|e| AppError::Tool(format!("delete dir {}: {e}", p.display())))?;
                } else {
                    tokio::fs::remove_file(&p)
                        .await
                        .map_err(|e| AppError::Tool(format!("delete {}: {e}", p.display())))?;
                }
                Ok(ToolOutcome::Text {
                    output: format!("deleted {}", path),
                    stderr: String::new(),
                    exit_code: Some(0),
                    duration_ms: 0,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tool() -> (FileTool, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let cfg = ToolsConfig {
            workspace_dir: dir.path().to_path_buf(),
            terminal_timeout_secs: 5,
            allow_web_search: false,
            allow_http_fetch: false,
            http_fetch_allowlist: vec![],
        };
        (FileTool::new(cfg), dir)
    }

    #[tokio::test]
    async fn write_then_read_roundtrip() {
        let (t, _dir) = tool();
        t.execute(json!({
            "operation": "write",
            "path": "hello.txt",
            "content": "hi",
        }))
        .await
        .unwrap();
        let outcome = t
            .execute(json!({ "operation": "read", "path": "hello.txt" }))
            .await
            .unwrap();
        match outcome {
            ToolOutcome::Text { output, .. } => assert_eq!(output, "hi"),
            o => panic!("unexpected outcome: {o:?}"),
        }
    }

    #[tokio::test]
    async fn path_escape_is_blocked() {
        let (t, _dir) = tool();
        let err = t
            .execute(json!({ "operation": "read", "path": "../../etc/passwd" }))
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Forbidden(_)));
    }
}
