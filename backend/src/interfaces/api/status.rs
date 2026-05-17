//! `GET /api/status`.

use std::sync::Arc;

use axum::{extract::State, Json};
use serde::Serialize;

use crate::error::AppResult;
use crate::state::AppState;

/// Status response body.
#[derive(Debug, Serialize)]
pub struct StatusResponse {
    /// Backend version.
    pub version: String,
    /// Process uptime in seconds.
    pub uptime_secs: i64,
    /// Currently-running task ids.
    pub active_tasks: Vec<String>,
    /// Configured default provider.
    pub default_provider: String,
    /// Configured default model.
    pub default_model: String,
    /// Configured CORS origins (echoed for the dashboard health check).
    pub cors_origins: Vec<String>,
}

/// Returns service status.
pub async fn get_status(State(state): State<Arc<AppState>>) -> AppResult<Json<StatusResponse>> {
    let cfg = state.config.read().await.clone();
    let active: Vec<String> = state
        .active_tasks
        .iter()
        .map(|kv| kv.key().clone())
        .collect();
    let provider = state.llm_router.default_provider().await;
    let model = state.llm_router.default_model().await;
    Ok(Json(StatusResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_secs: chrono::Utc::now()
            .signed_duration_since(state.started_at)
            .num_seconds(),
        active_tasks: active,
        default_provider: provider,
        default_model: model,
        cors_origins: cfg.server.cors_origins,
    }))
}
