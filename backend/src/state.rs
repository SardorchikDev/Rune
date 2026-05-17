//! Shared [`AppState`] passed to every Axum handler, Telegram callback and
//! agent loop invocation.

use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::{broadcast, oneshot};

use crate::agent::memory::MemoryStore;
use crate::config::SharedConfig;
use crate::core::llm::LlmRouter;
use crate::interfaces::api::ws::WsEvent;
use crate::tools::ToolRegistry;

/// Channel capacity for the broadcast bus that fans WebSocket events out to
/// every connected dashboard client.
pub const WS_CHANNEL_CAPACITY: usize = 1024;

/// Application-wide state. Cloned cheaply via `Arc`.
pub struct AppState {
    /// Hot-reloadable configuration.
    pub config: SharedConfig,
    /// Process start time. Used to compute `/api/status.uptime_secs`.
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// SQLite connection pool.
    pub db: sqlx::SqlitePool,
    /// LLM router with failover.
    pub llm_router: Arc<LlmRouter>,
    /// Currently-running tasks, mapped to their abort handles.
    pub active_tasks: Arc<DashMap<String, AbortToken>>,
    /// Broadcast bus for WebSocket events.
    pub ws_broadcast: broadcast::Sender<WsEvent>,
    /// Memory store (episodic + semantic).
    pub memory: Arc<MemoryStore>,
    /// Tool registry.
    pub tools: Arc<ToolRegistry>,
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("started_at", &self.started_at)
            .field("active_tasks", &self.active_tasks.len())
            .finish()
    }
}

/// A handle that can be used to cancel a running agent task. Wraps a
/// `oneshot::Sender` so that callers can call `.abort()` from anywhere
/// without holding the agent loop's `JoinHandle`.
pub struct AbortToken {
    sender: Option<oneshot::Sender<()>>,
}

impl AbortToken {
    /// Creates a new pair of `(token, rx)` — store the rx inside the agent
    /// loop and the token inside `active_tasks`.
    pub fn new() -> (Self, oneshot::Receiver<()>) {
        let (tx, rx) = oneshot::channel();
        (Self { sender: Some(tx) }, rx)
    }

    /// Sends the abort signal. Returns `true` if the agent loop was still
    /// listening for cancellations.
    pub fn abort(&mut self) -> bool {
        if let Some(tx) = self.sender.take() {
            tx.send(()).is_ok()
        } else {
            false
        }
    }
}

/// Convenience alias for the shared state wrapped in an `Arc`.
pub type Shared = Arc<AppState>;
