//! Arbitrary URL fetcher with a hostname allowlist.

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use super::types::ToolOutcome;
use super::Tool;
use crate::config::ToolsConfig;
use crate::error::{AppError, AppResult};

/// HTTP fetch tool. Strict hostname allowlist enforced from
/// `config.tools.http_fetch_allowlist`.
pub struct HttpFetchTool {
    http: reqwest::Client,
    allowlist: Vec<String>,
}

impl HttpFetchTool {
    /// Constructs a new fetcher backed by a fresh `reqwest::Client`.
    pub fn new(cfg: ToolsConfig) -> AppResult<Self> {
        Ok(Self {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .user_agent("rune-agent/1.0")
                .build()
                .map_err(AppError::from)?,
            allowlist: cfg.http_fetch_allowlist,
        })
    }

    fn host_allowed(&self, host: &str) -> bool {
        let host = host.to_lowercase();
        self.allowlist.iter().any(|allowed| {
            let allowed = allowed.to_lowercase();
            host == allowed || host.ends_with(&format!(".{allowed}"))
        })
    }
}

#[derive(Debug, Deserialize)]
struct Args {
    url: String,
}

#[async_trait]
impl Tool for HttpFetchTool {
    fn name(&self) -> &'static str {
        "http_fetch"
    }
    fn description(&self) -> &'static str {
        "Fetches a single URL (GET) from the configured allowlist. Returns \
         the raw response body and status code."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "url": { "type": "string", "format": "uri" }
            },
            "required": ["url"]
        })
    }

    async fn execute(&self, params: serde_json::Value) -> AppResult<ToolOutcome> {
        let Args { url } = serde_json::from_value(params)
            .map_err(|e| AppError::Tool(format!("invalid http_fetch args: {e}")))?;

        let parsed =
            url::Url::parse(&url).map_err(|e| AppError::Tool(format!("invalid url: {e}")))?;
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err(AppError::Forbidden(format!(
                "http_fetch only supports http(s), got {}",
                parsed.scheme()
            )));
        }
        let host = parsed
            .host_str()
            .ok_or_else(|| AppError::Tool("url missing host".into()))?;
        if !self.host_allowed(host) {
            return Err(AppError::Forbidden(format!(
                "host {host} is not in the allowlist"
            )));
        }

        let resp = self
            .http
            .get(parsed.clone())
            // Strip outbound Authorization header by not setting any. We rely
            // on reqwest's default behaviour (no headers carry over from the
            // calling agent) and explicitly do not forward credentials.
            .send()
            .await
            .map_err(|e| AppError::Tool(format!("http_fetch network: {e}")))?;
        let status = resp.status().as_u16();
        let body = resp
            .text()
            .await
            .map_err(|e| AppError::Tool(format!("http_fetch body: {e}")))?;

        Ok(ToolOutcome::Structured {
            value: json!({
                "url": url,
                "status": status,
                "body": body,
            }),
            duration_ms: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn tool(allow: Vec<&str>) -> HttpFetchTool {
        let cfg = ToolsConfig {
            workspace_dir: PathBuf::from("./workspace"),
            terminal_timeout_secs: 5,
            allow_web_search: false,
            allow_http_fetch: true,
            http_fetch_allowlist: allow.into_iter().map(String::from).collect(),
        };
        HttpFetchTool::new(cfg).unwrap()
    }

    #[test]
    fn matches_apex_and_subdomain() {
        let t = tool(vec!["github.com"]);
        assert!(t.host_allowed("github.com"));
        assert!(t.host_allowed("api.github.com"));
        assert!(!t.host_allowed("evil.com"));
    }
}
