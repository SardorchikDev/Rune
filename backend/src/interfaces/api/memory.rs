//! Memory browse / add / delete endpoints.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::agent::memory::MemoryItem;
use crate::error::AppResult;
use crate::state::AppState;

/// Query for `GET /api/memory`.
#[derive(Debug, Deserialize)]
pub struct ListQuery {
    /// Substring search.
    #[serde(default)]
    pub query: Option<String>,
    /// Result limit (default 50).
    #[serde(default)]
    pub limit: Option<i64>,
}

/// `GET /api/memory`.
pub async fn list_memory(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ListQuery>,
) -> AppResult<Json<Vec<MemoryItem>>> {
    let limit = q.limit.unwrap_or(50).clamp(1, 500);
    let items = state.memory.semantic.list(q.query.as_deref(), limit).await?;
    Ok(Json(items))
}

/// Body for `POST /api/memory`.
#[derive(Debug, Deserialize)]
pub struct AddMemoryRequest {
    /// Content to store.
    pub content: String,
    /// Optional task id.
    #[serde(default)]
    pub task_id: Option<String>,
}

/// `POST /api/memory`.
pub async fn add_memory(
    State(state): State<Arc<AppState>>,
    Json(body): Json<AddMemoryRequest>,
) -> AppResult<Json<MemoryItem>> {
    let task_id = body.task_id.unwrap_or_else(|| "manual".to_string());
    let item = state.memory.semantic.store(&task_id, &body.content).await?;
    Ok(Json(item))
}

/// `DELETE /api/memory/:id`.
pub async fn delete_memory(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    state.memory.semantic.delete(&id).await?;
    Ok(Json(serde_json::json!({ "deleted": id })))
}
