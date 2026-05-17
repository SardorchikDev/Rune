//! Unified application error type. Every fallible operation in Rune that
//! crosses a public boundary returns [`AppResult<T>`] which is an alias for
//! `Result<T, AppError>`.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

/// Convenience alias for results returned from the application.
pub type AppResult<T> = Result<T, AppError>;

/// Top-level error enum used by REST handlers, the agent loop and the
/// Telegram interface. Each variant maps to a stable HTTP status code and a
/// JSON payload that is safe to expose to the dashboard.
#[derive(Debug, Error)]
pub enum AppError {
    /// Request was unauthenticated or carried an invalid JWT.
    #[error("unauthorized: {0}")]
    Unauthorized(String),

    /// Request body / parameters failed validation.
    #[error("bad request: {0}")]
    BadRequest(String),

    /// Requested resource was not found.
    #[error("not found: {0}")]
    NotFound(String),

    /// Caller exceeded a rate limit.
    #[error("rate limited")]
    RateLimited,

    /// Caller attempted to escape the workspace sandbox.
    #[error("forbidden: {0}")]
    Forbidden(String),

    /// Database error (sqlx).
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    /// Migration error (sqlx::migrate).
    #[error("migration error: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),

    /// Configuration error.
    #[error("config error: {0}")]
    Config(String),

    /// Outbound HTTP error (LLM providers, web search, etc.).
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),

    /// JSON (de)serialisation error.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// JWT encoding / decoding error.
    #[error("jwt error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),

    /// LLM provider exhausted all retries / failovers.
    #[error("llm error: {0}")]
    Llm(String),

    /// Tool execution failure.
    #[error("tool error: {0}")]
    Tool(String),

    /// Catch-all for anyhow errors raised inside subsystems.
    #[error("internal error: {0}")]
    Internal(String),
}

impl AppError {
    /// Maps an [`AppError`] to its HTTP status code.
    pub fn status(&self) -> StatusCode {
        match self {
            AppError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            AppError::Forbidden(_) => StatusCode::FORBIDDEN,
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            AppError::Database(_)
            | AppError::Migrate(_)
            | AppError::Config(_)
            | AppError::Network(_)
            | AppError::Json(_)
            | AppError::Jwt(_)
            | AppError::Llm(_)
            | AppError::Tool(_)
            | AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Short machine-readable error code that the dashboard can switch on.
    pub fn code(&self) -> &'static str {
        match self {
            AppError::Unauthorized(_) => "unauthorized",
            AppError::Forbidden(_) => "forbidden",
            AppError::BadRequest(_) => "bad_request",
            AppError::NotFound(_) => "not_found",
            AppError::RateLimited => "rate_limited",
            AppError::Database(_) => "db_error",
            AppError::Migrate(_) => "migrate_error",
            AppError::Config(_) => "config_error",
            AppError::Network(_) => "network_error",
            AppError::Json(_) => "json_error",
            AppError::Jwt(_) => "jwt_error",
            AppError::Llm(_) => "llm_error",
            AppError::Tool(_) => "tool_error",
            AppError::Internal(_) => "internal_error",
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status();
        let code = self.code();
        let message = self.to_string();
        tracing::error!(error.code = %code, error.message = %message, "request failed");
        let body = Json(json!({
            "error": {
                "code": code,
                "message": message,
            }
        }));
        (status, body).into_response()
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::Internal(format!("{err:#}"))
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::Internal(format!("io: {err}"))
    }
}

impl From<toml::de::Error> for AppError {
    fn from(err: toml::de::Error) -> Self {
        AppError::Config(format!("toml parse: {err}"))
    }
}
