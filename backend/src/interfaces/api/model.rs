//! `GET /api/model` and `PUT /api/model`.

use std::sync::Arc;

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::error::AppResult;
use crate::state::AppState;

/// Current model response.
#[derive(Debug, Serialize)]
pub struct ModelResponse {
    /// Active provider.
    pub provider: String,
    /// Active model.
    pub model: String,
    /// Names of every registered provider.
    pub providers: Vec<&'static str>,
}

/// `GET /api/model`.
pub async fn get_model(State(state): State<Arc<AppState>>) -> AppResult<Json<ModelResponse>> {
    Ok(Json(ModelResponse {
        provider: state.llm_router.default_provider().await,
        model: state.llm_router.default_model().await,
        providers: state.llm_router.provider_names(),
    }))
}

/// Body for `PUT /api/model`.
#[derive(Debug, Deserialize)]
pub struct SetModelRequest {
    /// Provider name.
    pub provider: String,
    /// Model identifier.
    pub model: String,
}

/// `PUT /api/model`.
pub async fn set_model(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SetModelRequest>,
) -> AppResult<Json<ModelResponse>> {
    state
        .llm_router
        .set_default(&body.provider, &body.model)
        .await?;
    Ok(Json(ModelResponse {
        provider: body.provider,
        model: body.model,
        providers: state.llm_router.provider_names(),
    }))
}
