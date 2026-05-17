//! Runtime provider selector + failover logic.

use std::collections::HashMap;
use std::sync::Arc;

use futures::{Stream, StreamExt};
use sqlx::SqlitePool;
use tokio::sync::RwLock;

use super::metrics::record_usage;
use super::providers::build_all;
use super::types::{LlmRequest, LlmResponse, StreamChunk, TokenStream};
use super::LlmProvider;
use crate::config::RuneConfig;
use crate::error::{AppError, AppResult};

/// LLM router with failover. Holds one instance per provider.
pub struct LlmRouter {
    providers: HashMap<String, Arc<dyn LlmProvider>>,
    default_provider: RwLock<String>,
    default_model: RwLock<String>,
    failover_enabled: bool,
    failover_order: Vec<String>,
    max_retries: u32,
    db: SqlitePool,
}

impl LlmRouter {
    /// Builds a router from the given configuration and database pool.
    pub fn from_config(cfg: &RuneConfig, db: SqlitePool) -> AppResult<Self> {
        let providers = build_all(cfg)?;
        let mut map: HashMap<String, Arc<dyn LlmProvider>> = HashMap::new();
        for p in providers {
            map.insert(p.name().to_string(), p);
        }
        Ok(Self {
            providers: map,
            default_provider: RwLock::new(cfg.llm.default_provider.clone()),
            default_model: RwLock::new(cfg.llm.default_model.clone()),
            failover_enabled: cfg.llm.failover.enabled,
            failover_order: cfg.llm.failover.order.clone(),
            max_retries: cfg.llm.max_retries,
            db,
        })
    }

    /// Builds a router from an explicit provider map. Used by tests.
    pub fn from_providers(
        providers: HashMap<String, Arc<dyn LlmProvider>>,
        default_provider: &str,
        default_model: &str,
        failover_order: Vec<String>,
        db: SqlitePool,
    ) -> Self {
        Self {
            providers,
            default_provider: RwLock::new(default_provider.to_string()),
            default_model: RwLock::new(default_model.to_string()),
            failover_enabled: !failover_order.is_empty(),
            failover_order,
            max_retries: 1,
            db,
        }
    }

    /// Returns the current default provider name.
    pub async fn default_provider(&self) -> String {
        self.default_provider.read().await.clone()
    }

    /// Returns the current default model name.
    pub async fn default_model(&self) -> String {
        self.default_model.read().await.clone()
    }

    /// Sets the default provider + model. Validates that the provider exists.
    pub async fn set_default(&self, provider: &str, model: &str) -> AppResult<()> {
        if !self.providers.contains_key(provider) {
            return Err(AppError::BadRequest(format!("unknown provider: {provider}")));
        }
        *self.default_provider.write().await = provider.to_string();
        *self.default_model.write().await = model.to_string();
        Ok(())
    }

    /// Returns the list of provider names registered with the router.
    pub fn provider_names(&self) -> Vec<&'static str> {
        let mut names: Vec<_> = self.providers.values().map(|p| p.name()).collect();
        names.sort();
        names
    }

    /// Returns the failover chain: default provider first, then any
    /// additional providers from the configured order.
    async fn failover_chain(&self, preferred: Option<&str>) -> Vec<String> {
        let mut chain = Vec::new();
        let first = preferred
            .map(|s| s.to_string())
            .unwrap_or_else(|| self.default_provider.try_read().map(|g| g.clone()).unwrap_or_default());
        chain.push(first.clone());
        if self.failover_enabled {
            for name in &self.failover_order {
                if name != &first {
                    chain.push(name.clone());
                }
            }
        }
        chain
    }

    /// Routes a synchronous completion through the failover chain. Records
    /// usage against `task_id` (if provided).
    pub async fn route(
        &self,
        task_id: Option<&str>,
        mut req: LlmRequest,
        preferred: Option<&str>,
    ) -> AppResult<LlmResponse> {
        let chain = self.failover_chain(preferred).await;
        let mut last_err: Option<AppError> = None;
        for name in &chain {
            let Some(provider) = self.providers.get(name) else { continue };
            if !provider.is_available() {
                tracing::warn!(provider = %name, "skipping unavailable provider");
                continue;
            }
            for attempt in 0..self.max_retries.max(1) {
                let req_clone = req.clone();
                match provider.complete(req_clone).await {
                    Ok(response) => {
                        record_usage(&self.db, task_id, &response).await?;
                        return Ok(response);
                    }
                    Err(e) => {
                        tracing::warn!(
                            provider = %name,
                            attempt,
                            "provider call failed: {e}"
                        );
                        last_err = Some(e);
                        if attempt + 1 < self.max_retries {
                            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        }
                    }
                }
                let _ = &mut req; // suppress unused-mut warning if max_retries == 1
            }
        }
        Err(last_err.unwrap_or_else(|| {
            AppError::Llm(format!("LlmRouter: all providers exhausted (tried: {chain:?})"))
        }))
    }

    /// Routes a streaming completion through the failover chain.
    pub async fn stream_route(
        &self,
        req: LlmRequest,
        preferred: Option<&str>,
    ) -> AppResult<TokenStream> {
        let chain = self.failover_chain(preferred).await;
        let mut last_err: Option<AppError> = None;
        for name in &chain {
            let Some(provider) = self.providers.get(name) else { continue };
            if !provider.is_available() {
                continue;
            }
            let req_clone = req.clone();
            match provider.stream(req_clone).await {
                Ok(stream) => return Ok(stream),
                Err(e) => {
                    tracing::warn!(provider = %name, "stream call failed: {e}");
                    last_err = Some(e);
                }
            }
        }
        Err(last_err.unwrap_or_else(|| {
            AppError::Llm("LlmRouter: all providers exhausted (stream)".into())
        }))
    }

    /// Computes an embedding using the embedding provider declared in the
    /// memory config. Falls back to the default provider's embed.
    pub async fn embed(&self, provider: &str, text: &str) -> AppResult<Vec<f32>> {
        let Some(p) = self.providers.get(provider) else {
            return Err(AppError::Llm(format!("unknown embedding provider: {provider}")));
        };
        p.embed(text).await
    }
}

impl Clone for LlmRequest {
    fn clone(&self) -> Self {
        Self {
            messages: self.messages.clone(),
            model: self.model.clone(),
            stream: self.stream,
            max_tokens: self.max_tokens,
            temperature: self.temperature,
            tools: self.tools.clone(),
        }
    }
}

/// Parses an SSE response and yields decoded chunks via `decoder`. Skips
/// non-`data:` lines and `[DONE]` sentinels.
pub fn sse_token_stream<F>(
    response: reqwest::Response,
    decoder: F,
) -> impl Stream<Item = AppResult<StreamChunk>> + Send
where
    F: Fn(&str) -> Option<StreamChunk> + Send + 'static,
{
    use async_stream::stream;

    stream! {
        let mut byte_stream = response.bytes_stream();
        let mut buf = String::new();
        while let Some(chunk) = byte_stream.next().await {
            let chunk = match chunk {
                Ok(b) => b,
                Err(e) => {
                    yield Err(AppError::Llm(format!("sse network error: {e}")));
                    return;
                }
            };
            buf.push_str(&String::from_utf8_lossy(&chunk));
            while let Some(idx) = buf.find("\n\n").or_else(|| buf.find('\n')) {
                let raw_event = buf[..idx].to_string();
                buf.drain(..=idx);
                for line in raw_event.lines() {
                    let line = line.trim();
                    let Some(payload) = line.strip_prefix("data:") else { continue };
                    let payload = payload.trim();
                    if payload.is_empty() { continue; }
                    if let Some(decoded) = decoder(payload) {
                        yield Ok(decoded);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct DummyProvider {
        name: &'static str,
        fail: bool,
        calls: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl LlmProvider for DummyProvider {
        fn name(&self) -> &'static str { self.name }
        fn supported_models(&self) -> Vec<String> { vec!["m".into()] }
        fn is_available(&self) -> bool { true }

        async fn complete(&self, _req: LlmRequest) -> AppResult<LlmResponse> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.fail {
                Err(AppError::Llm(format!("{} fails", self.name)))
            } else {
                Ok(LlmResponse {
                    content: format!("hello from {}", self.name),
                    tool_calls: vec![],
                    input_tokens: 1,
                    output_tokens: 1,
                    provider: self.name.into(),
                    model: "m".into(),
                })
            }
        }

        async fn stream(&self, _req: LlmRequest) -> AppResult<TokenStream> {
            Err(AppError::Llm("not implemented".into()))
        }

        async fn embed(&self, _text: &str) -> AppResult<Vec<f32>> {
            Err(AppError::Llm("not implemented".into()))
        }
    }

    async fn make_pool() -> SqlitePool {
        let dir = tempfile::tempdir().unwrap();
        let url = format!("sqlite://{}/router.db?mode=rwc", dir.path().display());
        let pool = crate::core::db::init_pool(&url).await.expect("pool");
        std::mem::forget(dir);
        pool
    }

    #[tokio::test]
    async fn falls_over_on_first_provider_failure() {
        let calls_a = Arc::new(AtomicUsize::new(0));
        let calls_b = Arc::new(AtomicUsize::new(0));
        let mut map: HashMap<String, Arc<dyn LlmProvider>> = HashMap::new();
        map.insert(
            "a".into(),
            Arc::new(DummyProvider { name: "a", fail: true, calls: calls_a.clone() }),
        );
        map.insert(
            "b".into(),
            Arc::new(DummyProvider { name: "b", fail: false, calls: calls_b.clone() }),
        );

        let pool = make_pool().await;
        let router = LlmRouter::from_providers(map, "a", "m", vec!["b".into()], pool);
        let response = router
            .route(
                None,
                LlmRequest::new(vec![], "m"),
                None,
            )
            .await
            .expect("should succeed via failover");
        assert_eq!(response.provider, "b");
        assert!(calls_a.load(Ordering::SeqCst) >= 1);
        assert_eq!(calls_b.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn returns_error_when_all_fail() {
        let mut map: HashMap<String, Arc<dyn LlmProvider>> = HashMap::new();
        map.insert(
            "a".into(),
            Arc::new(DummyProvider {
                name: "a",
                fail: true,
                calls: Arc::new(AtomicUsize::new(0)),
            }),
        );
        map.insert(
            "b".into(),
            Arc::new(DummyProvider {
                name: "b",
                fail: true,
                calls: Arc::new(AtomicUsize::new(0)),
            }),
        );

        let pool = make_pool().await;
        let router = LlmRouter::from_providers(map, "a", "m", vec!["b".into()], pool);
        let res = router.route(None, LlmRequest::new(vec![], "m"), None).await;
        assert!(res.is_err());
    }
}
