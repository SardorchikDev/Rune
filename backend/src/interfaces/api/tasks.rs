//! Task CRUD endpoints + abort.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::agent::run_agent_task;
use crate::error::{AppError, AppResult};
use crate::state::{AbortToken, AppState};

/// Body for `POST /api/tasks`.
#[derive(Debug, Deserialize)]
pub struct CreateTaskRequest {
    /// User prompt.
    pub prompt: String,
    /// Optional preferred provider (otherwise router default).
    #[serde(default)]
    pub provider: Option<String>,
    /// Optional preferred model.
    #[serde(default)]
    pub model: Option<String>,
}

/// Response for `POST /api/tasks`.
#[derive(Debug, Serialize)]
pub struct CreateTaskResponse {
    /// Task id (UUID).
    pub task_id: String,
}

/// Stored task row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRow {
    /// Task id.
    pub id: String,
    /// Owning session id.
    pub session_id: String,
    /// User prompt.
    pub prompt: String,
    /// Current status.
    pub status: String,
    /// Provider that handled the task (if any).
    pub provider: Option<String>,
    /// Model used (if any).
    pub model: Option<String>,
    /// Input tokens.
    pub total_input_tokens: i64,
    /// Output tokens.
    pub total_output_tokens: i64,
    /// Accumulated cost (USD).
    pub cost_usd: f64,
    /// Created at.
    pub created_at: DateTime<Utc>,
    /// Last update.
    pub updated_at: DateTime<Utc>,
    /// Completion timestamp (if any).
    pub completed_at: Option<DateTime<Utc>>,
}

/// `POST /api/tasks`.
pub async fn create_task(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateTaskRequest>,
) -> AppResult<Json<CreateTaskResponse>> {
    if req.prompt.trim().is_empty() {
        return Err(AppError::BadRequest("prompt must not be empty".into()));
    }

    let session_id = ensure_session(&state.db, "web", None).await?;
    let task_id = Uuid::new_v4().to_string();

    let (provider, model) = match (&req.provider, &req.model) {
        (Some(p), Some(m)) => (Some(p.clone()), Some(m.clone())),
        _ => (
            Some(state.llm_router.default_provider().await),
            Some(state.llm_router.default_model().await),
        ),
    };

    sqlx::query(
        "INSERT INTO tasks (id, session_id, prompt, status, provider, model) VALUES (?, ?, ?, 'pending', ?, ?)",
    )
    .bind(&task_id)
    .bind(&session_id)
    .bind(&req.prompt)
    .bind(&provider)
    .bind(&model)
    .execute(&state.db)
    .await?;

    let (token, rx) = AbortToken::new();
    state.active_tasks.insert(task_id.clone(), token);

    let task_id_for_loop = task_id.clone();
    let prompt = req.prompt.clone();
    let state_for_loop = state.clone();
    tokio::spawn(async move {
        let task_id = task_id_for_loop.clone();
        if let Err(e) = run_agent_task(state_for_loop.clone(), task_id_for_loop, prompt, rx).await {
            tracing::error!(task_id = %task_id, "agent task failed: {e}");
            let _ = sqlx::query("UPDATE tasks SET status = 'failed' WHERE id = ?")
                .bind(&task_id)
                .execute(&state_for_loop.db)
                .await;
        }
        state_for_loop.active_tasks.remove(&task_id);
    });

    Ok(Json(CreateTaskResponse { task_id }))
}

/// Pagination + filter parameters for `GET /api/tasks`.
#[derive(Debug, Deserialize)]
pub struct ListTasksQuery {
    /// Optional status filter.
    #[serde(default)]
    pub status: Option<String>,
    /// Page size (default 50).
    #[serde(default)]
    pub limit: Option<i64>,
}

/// `GET /api/tasks`.
pub async fn list_tasks(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ListTasksQuery>,
) -> AppResult<Json<Vec<TaskRow>>> {
    let limit = q.limit.unwrap_or(50).clamp(1, 500);
    let rows = match q.status {
        Some(s) => {
            sqlx::query_as::<_, TaskRow>(
                "SELECT id, session_id, prompt, status, provider, model, total_input_tokens, total_output_tokens, cost_usd, created_at, updated_at, completed_at FROM tasks WHERE status = ? ORDER BY created_at DESC LIMIT ?",
            )
            .bind(s)
            .bind(limit)
            .fetch_all(&state.db)
            .await?
        }
        None => {
            sqlx::query_as::<_, TaskRow>(
                "SELECT id, session_id, prompt, status, provider, model, total_input_tokens, total_output_tokens, cost_usd, created_at, updated_at, completed_at FROM tasks ORDER BY created_at DESC LIMIT ?",
            )
            .bind(limit)
            .fetch_all(&state.db)
            .await?
        }
    };
    Ok(Json(rows))
}

impl<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow> for TaskRow {
    fn from_row(row: &'r sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;
        Ok(Self {
            id: row.try_get("id")?,
            session_id: row.try_get("session_id")?,
            prompt: row.try_get("prompt")?,
            status: row.try_get("status")?,
            provider: row.try_get("provider")?,
            model: row.try_get("model")?,
            total_input_tokens: row.try_get("total_input_tokens")?,
            total_output_tokens: row.try_get("total_output_tokens")?,
            cost_usd: row.try_get("cost_usd")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
            completed_at: row.try_get("completed_at")?,
        })
    }
}

/// `GET /api/tasks/:id` — task with embedded log entries.
pub async fn get_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let row: TaskRow = sqlx::query_as(
        "SELECT id, session_id, prompt, status, provider, model, total_input_tokens, total_output_tokens, cost_usd, created_at, updated_at, completed_at FROM tasks WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("task {id} not found")))?;

    let logs: Vec<(i64, i64, String, String, DateTime<Utc>)> = sqlx::query_as(
        "SELECT id, iteration, phase, metadata, created_at FROM agent_logs WHERE task_id = ? ORDER BY id ASC",
    )
    .bind(&id)
    .fetch_all(&state.db)
    .await?;

    let logs_json: Vec<_> = logs
        .into_iter()
        .map(|(log_id, iteration, phase, metadata, created_at)| {
            let value: serde_json::Value =
                serde_json::from_str(&metadata).unwrap_or(serde_json::Value::Null);
            serde_json::json!({
                "id": log_id,
                "iteration": iteration,
                "phase": phase,
                "metadata": value,
                "created_at": created_at,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "task": row,
        "logs": logs_json,
    })))
}

/// Body for `POST /api/agent/abort`.
#[derive(Debug, Deserialize)]
pub struct AbortBody {
    /// Task id to abort.
    pub task_id: String,
}

/// `POST /api/agent/abort`.
pub async fn abort_task(
    State(state): State<Arc<AppState>>,
    Json(body): Json<AbortBody>,
) -> AppResult<Json<serde_json::Value>> {
    let mut entry = state
        .active_tasks
        .get_mut(&body.task_id)
        .ok_or_else(|| AppError::NotFound(format!("no active task {}", body.task_id)))?;
    let aborted = entry.abort();
    drop(entry);
    Ok(Json(serde_json::json!({ "aborted": aborted })))
}

/// Inserts a new session row and returns its id.
pub async fn ensure_session(
    db: &sqlx::SqlitePool,
    interface: &str,
    telegram_user_id: Option<i64>,
) -> AppResult<String> {
    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO sessions (id, interface, telegram_user_id) VALUES (?, ?, ?)")
        .bind(&id)
        .bind(interface)
        .bind(telegram_user_id)
        .execute(db)
        .await?;
    Ok(id)
}
