//! WebSocket bridge between the agent loop and the dashboard.

use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::IntoResponse,
};
use chrono::Utc;
use futures::{SinkExt, StreamExt};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};

use crate::core::llm::types::ToolCall;
use crate::interfaces::api::auth::Claims;
use crate::state::AppState;
use crate::tools::types::ToolOutcome;

/// Event emitted on the broadcast bus. The dashboard subscribes via the
/// websocket; subscribers filter by `task_id` client-side.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsEvent {
    /// Free-text token delta from the LLM.
    Token {
        /// Task this token belongs to.
        task_id: String,
        /// Delta payload.
        text: String,
    },
    /// Tool call emitted by the LLM.
    ToolCall {
        /// Task id.
        task_id: String,
        /// Tool name.
        name: String,
        /// Tool arguments.
        arguments: serde_json::Value,
    },
    /// Tool execution result.
    ToolResult {
        /// Task id.
        task_id: String,
        /// Tool name.
        name: String,
        /// Result payload.
        outcome: ToolOutcome,
    },
    /// Status transition.
    Status {
        /// Task id.
        task_id: String,
        /// New status string.
        status: String,
    },
    /// Final answer payload (after `TASK_COMPLETE:`).
    FinalAnswer {
        /// Task id.
        task_id: String,
        /// Final answer text.
        text: String,
        /// Final status.
        status: String,
    },
}

impl WsEvent {
    /// Convenience constructor for [`WsEvent::Token`].
    pub fn token(task_id: &str, text: &str) -> Self {
        Self::Token {
            task_id: task_id.into(),
            text: text.into(),
        }
    }
    /// Convenience constructor for [`WsEvent::Status`].
    pub fn status(task_id: &str, status: &str) -> Self {
        Self::Status {
            task_id: task_id.into(),
            status: status.into(),
        }
    }
    /// Convenience constructor for [`WsEvent::ToolCall`].
    pub fn tool_call(task_id: &str, call: &ToolCall) -> Self {
        Self::ToolCall {
            task_id: task_id.into(),
            name: call.name.clone(),
            arguments: call.arguments.clone(),
        }
    }
    /// Convenience constructor for [`WsEvent::ToolResult`].
    pub fn tool_result(task_id: &str, call: &ToolCall, outcome: &ToolOutcome) -> Self {
        Self::ToolResult {
            task_id: task_id.into(),
            name: call.name.clone(),
            outcome: outcome.clone(),
        }
    }
    /// Convenience constructor for [`WsEvent::FinalAnswer`].
    pub fn final_answer(task_id: &str, text: &str, status: &str) -> Self {
        Self::FinalAnswer {
            task_id: task_id.into(),
            text: text.into(),
            status: status.into(),
        }
    }
}

/// Query string for the WebSocket upgrade.
#[derive(Debug, Deserialize)]
pub struct WsAuth {
    /// JWT token (passed via query string because browsers can't set headers
    /// on the WebSocket upgrade request).
    pub token: String,
}

/// Axum handler for `/api/ws`.
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Query(auth): Query<WsAuth>,
) -> impl IntoResponse {
    let secret = state.config.read().await.server.jwt_secret.clone();
    let mut validation = Validation::default();
    validation.set_required_spec_claims(&["exp", "sub"]);
    let claims = decode::<Claims>(
        &auth.token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    );
    if claims.is_err() {
        return axum::http::Response::builder()
            .status(axum::http::StatusCode::UNAUTHORIZED)
            .body(axum::body::Body::from("invalid websocket token"))
            .expect("response build");
    }
    ws.on_upgrade(move |socket| handle_socket(socket, state))
        .into_response()
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.ws_broadcast.subscribe();
    let hello = serde_json::json!({
        "type": "hello",
        "server_time": Utc::now(),
    });
    if let Ok(payload) = serde_json::to_string(&hello) {
        let _ = sender.send(Message::Text(payload)).await;
    }

    let send_task = tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            let Ok(payload) = serde_json::to_string(&event) else {
                continue;
            };
            if sender.send(Message::Text(payload)).await.is_err() {
                break;
            }
        }
    });

    while let Some(Ok(msg)) = receiver.next().await {
        // Treat any inbound message as a heartbeat. We don't process inbound
        // commands over the socket today.
        if matches!(msg, Message::Close(_)) {
            break;
        }
    }
    send_task.abort();
}
