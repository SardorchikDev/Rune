//! DuckDuckGo-backed web search tool.

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use super::types::ToolOutcome;
use super::Tool;
use crate::error::{AppError, AppResult};

/// Web search tool. Uses DuckDuckGo's HTML endpoint as a low-cost, no-key
/// search backend.
pub struct WebSearchTool {
    http: reqwest::Client,
}

impl WebSearchTool {
    /// Creates a new web search tool.
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .user_agent("rune-agent/1.0 (+https://github.com/SardorchikDev/Rune)")
                .build()
                .expect("reqwest client"),
        }
    }
}

impl Default for WebSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct Args {
    query: String,
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_limit() -> usize {
    5
}

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &'static str {
        "web_search"
    }
    fn description(&self) -> &'static str {
        "Searches the public web via DuckDuckGo and returns a ranked list of \
         title/url/snippet results."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Free-text query" },
                "limit": { "type": "integer", "minimum": 1, "maximum": 25 }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, params: serde_json::Value) -> AppResult<ToolOutcome> {
        let Args { query, limit } = serde_json::from_value(params)
            .map_err(|e| AppError::Tool(format!("invalid web_search args: {e}")))?;

        let url = format!("https://duckduckgo.com/html/?q={}", urlencoding(&query));
        let body = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| AppError::Tool(format!("web_search network: {e}")))?
            .error_for_status()
            .map_err(|e| AppError::Tool(format!("web_search status: {e}")))?
            .text()
            .await
            .map_err(|e| AppError::Tool(format!("web_search body: {e}")))?;

        let results = parse_ddg(&body, limit);
        Ok(ToolOutcome::Structured {
            value: json!({ "query": query, "results": results }),
            duration_ms: 0,
        })
    }
}

fn urlencoding(s: &str) -> String {
    use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
    utf8_percent_encode(s, NON_ALPHANUMERIC).to_string()
}

fn parse_ddg(html: &str, limit: usize) -> Vec<serde_json::Value> {
    // Very small handwritten parser. DuckDuckGo's HTML endpoint wraps each
    // result in <a class="result__a" href="..."> ... </a> blocks with a
    // sibling <a class="result__snippet"> for the snippet. We do not pull
    // in a full HTML parser here to keep the dependency tree light; the
    // fallback is graceful (empty list).
    let mut results = Vec::new();
    for chunk in html.split("result__body").skip(1) {
        if results.len() >= limit {
            break;
        }
        let url = extract_attr(chunk, "result__a", "href").unwrap_or_default();
        let title = extract_text(chunk, "result__a").unwrap_or_default();
        let snippet = extract_text(chunk, "result__snippet").unwrap_or_default();
        if !url.is_empty() && !title.is_empty() {
            results.push(json!({
                "title": title,
                "url": url,
                "snippet": snippet,
            }));
        }
    }
    results
}

fn extract_attr(chunk: &str, class_marker: &str, attr: &str) -> Option<String> {
    let idx = chunk.find(class_marker)?;
    let rest = &chunk[idx..];
    let needle = format!("{attr}=\"");
    let attr_idx = rest.find(&needle)?;
    let after = &rest[attr_idx + needle.len()..];
    let end = after.find('"')?;
    Some(after[..end].to_string())
}

fn extract_text(chunk: &str, class_marker: &str) -> Option<String> {
    let idx = chunk.find(class_marker)?;
    let rest = &chunk[idx..];
    let gt = rest.find('>')?;
    let after = &rest[gt + 1..];
    let lt = after.find('<')?;
    let text = after[..lt].trim();
    Some(text.to_string())
}
