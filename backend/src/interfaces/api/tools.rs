//! `GET /api/tools` and `POST /api/tools/execute`.

use std::sync::Arc;

use axum::{extract::State, Json};

use crate::core::llm::types::{ToolCall, ToolDefinition};
use crate::error::AppResult;
use crate::state::AppState;
use crate::tools::types::{ToolInvocation, ToolOutcome};

/// `GET /api/tools`.
pub async fn list_tools(
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<Vec<ToolDefinition>>> {
    Ok(Json(state.tools.definitions()))
}

/// `POST /api/tools/execute`.
pub async fn execute_tool(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ToolInvocation>,
) -> AppResult<Json<ToolOutcome>> {
    let call = ToolCall {
        id: uuid::Uuid::new_v4().to_string(),
        name: body.name,
        arguments: body.arguments,
    };
    let outcome = state.tools.execute(&call).await;
    Ok(Json(outcome))
}
