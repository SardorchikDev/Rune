//! HTTP & WebSocket interface (axum).

pub mod auth;
pub mod config;
pub mod memory;
pub mod model;
pub mod status;
pub mod tasks;
pub mod tools;
pub mod ws;

use std::sync::Arc;

use axum::{
    middleware,
    routing::{delete, get, post},
    Router,
};
use tower_http::cors::CorsLayer;

use crate::state::AppState;

/// Builds the axum router with every Rune endpoint mounted, including the
/// JWT-protected `/api/*` namespace and the public auth + WebSocket routes.
pub fn build_router(state: Arc<AppState>) -> Router {
    let cors = build_cors_layer(state.clone());

    let public = Router::new()
        .route("/api/auth/login", post(auth::login))
        .route("/api/ws", get(ws::websocket_handler));

    let protected = Router::new()
        .route("/api/status", get(status::get_status))
        .route(
            "/api/tasks",
            post(tasks::create_task).get(tasks::list_tasks),
        )
        .route("/api/tasks/:id", get(tasks::get_task))
        .route("/api/agent/abort", post(tasks::abort_task))
        .route(
            "/api/memory",
            get(memory::list_memory).post(memory::add_memory),
        )
        .route("/api/memory/:id", delete(memory::delete_memory))
        .route("/api/tools", get(tools::list_tools))
        .route("/api/tools/execute", post(tools::execute_tool))
        .route("/api/model", get(model::get_model).put(model::set_model))
        .route(
            "/api/config",
            get(config::get_config).put(config::update_config),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_jwt,
        ));

    Router::new()
        .merge(public)
        .merge(protected)
        .with_state(state)
        .layer(cors)
}

/// Builds the configured CORS layer.
pub fn build_cors_layer(state: Arc<AppState>) -> CorsLayer {
    use axum::http::{HeaderValue, Method};
    let cors_origins = state
        .config
        .try_read()
        .map(|cfg| cfg.server.cors_origins.clone())
        .unwrap_or_default();
    if cors_origins.is_empty() {
        return CorsLayer::permissive();
    }
    let mut layer = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_credentials(true)
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
        ]);
    for origin in cors_origins {
        if let Ok(v) = HeaderValue::from_str(&origin) {
            layer = layer.allow_origin(v);
        }
    }
    layer
}
