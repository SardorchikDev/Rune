//! `GET /api/config` and `PUT /api/config`.

use std::sync::Arc;

use axum::{extract::State, Json};
use serde::Deserialize;

use crate::config::RuneConfig;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// `GET /api/config` — returns the live config with every secret masked.
pub async fn get_config(State(state): State<Arc<AppState>>) -> AppResult<Json<RuneConfig>> {
    let cfg = state.config.read().await.clone();
    Ok(Json(cfg.masked()))
}

/// `PUT /api/config` body — partial overrides applied on top of the existing
/// in-memory config. Empty / absent fields are left unchanged.
#[derive(Debug, Default, Deserialize)]
pub struct UpdateConfigRequest {
    /// Optional default provider override.
    #[serde(default)]
    pub default_provider: Option<String>,
    /// Optional default model override.
    #[serde(default)]
    pub default_model: Option<String>,
    /// Optional reflection toggle.
    #[serde(default)]
    pub reflection_enabled: Option<bool>,
    /// Optional max iterations override.
    #[serde(default)]
    pub max_iterations: Option<u32>,
}

/// `PUT /api/config`.
pub async fn update_config(
    State(state): State<Arc<AppState>>,
    Json(body): Json<UpdateConfigRequest>,
) -> AppResult<Json<RuneConfig>> {
    let mut cfg = state.config.write().await;
    if let Some(p) = body.default_provider.as_deref() {
        if !cfg.has_provider(p) {
            return Err(AppError::BadRequest(format!("unknown provider: {p}")));
        }
        cfg.llm.default_provider = p.to_string();
        if let Some(m) = body.default_model.as_deref() {
            cfg.llm.default_model = m.to_string();
        }
    } else if let Some(m) = body.default_model.as_deref() {
        cfg.llm.default_model = m.to_string();
    }
    if let Some(b) = body.reflection_enabled {
        cfg.agent.reflection_enabled = b;
    }
    if let Some(n) = body.max_iterations {
        cfg.agent.max_iterations = n;
    }
    let snapshot = cfg.clone();
    drop(cfg);

    // Propagate any model change to the router.
    state
        .llm_router
        .set_default(&snapshot.llm.default_provider, &snapshot.llm.default_model)
        .await?;
    Ok(Json(snapshot.masked()))
}
