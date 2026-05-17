//! Core perceive → recall → plan → execute → reflect agent loop.

use std::sync::Arc;
use std::time::Instant;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::oneshot;

use crate::agent::context::AgentContext;
use crate::agent::planner::Plan;
use crate::agent::reflector::reflect_and_store;
use crate::core::llm::types::{ChatMessage, LlmRequest, Role};
use crate::core::llm::LlmRouter;
use crate::error::{AppError, AppResult};
use crate::interfaces::api::ws::WsEvent;
use crate::state::AppState;

/// Outcome of a single agent task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskOutcome {
    /// Task id (UUID).
    pub task_id: String,
    /// Whether the task completed successfully.
    pub success: bool,
    /// Final answer text. Empty if the task was aborted or failed before
    /// the assistant emitted a final response.
    pub final_answer: String,
    /// Number of perceive/plan/execute iterations executed.
    pub iterations: u32,
    /// Final task status (`"completed"`, `"failed"`, `"aborted"`).
    pub status: String,
}

/// Marker emitted by the agent to declare it's finished. Matches the
/// declaration rule from the system prompt.
const COMPLETION_TAG: &str = "TASK_COMPLETE:";

/// Drives an agent task end-to-end. Returns when the LLM declares completion,
/// the abort signal fires, or `max_iterations` is reached.
pub async fn run_agent_task(
    state: Arc<AppState>,
    task_id: String,
    prompt: String,
    mut abort_rx: oneshot::Receiver<()>,
) -> AppResult<TaskOutcome> {
    let started_at = Instant::now();
    let cfg = state.config.read().await.clone();
    let system_prompt = load_system_prompt(&cfg.agent.system_prompt_path).await?;
    let mut context = AgentContext::new(
        system_prompt,
        cfg.agent.auto_summarize_threshold * 256,
        cfg.agent.auto_summarize_threshold,
    );
    context.push(ChatMessage::user(&prompt));

    update_task_status(&state.db, &task_id, "running").await?;
    let _ = state
        .ws_broadcast
        .send(WsEvent::status(&task_id, "running"));

    let recalled = state
        .memory
        .recall(&prompt)
        .await
        .inspect_err(|e| tracing::warn!("memory recall failed: {e}"))
        .unwrap_or_default();
    context.set_memories(recalled.clone());
    insert_log(
        &state.db,
        &task_id,
        0,
        "recall",
        &json!({"matches": recalled.len()}),
    )
    .await?;

    let mut plan = Plan::default();
    let mut iterations: u32 = 0;
    let mut success = false;
    let mut final_answer = String::new();

    'outer: loop {
        if iterations >= cfg.agent.max_iterations {
            tracing::warn!(task_id = %task_id, "max iterations reached");
            break;
        }
        if abort_rx.try_recv().is_ok() {
            tracing::info!(task_id = %task_id, "abort signal received");
            update_task_status(&state.db, &task_id, "aborted").await?;
            let _ = state
                .ws_broadcast
                .send(WsEvent::status(&task_id, "aborted"));
            return Ok(TaskOutcome {
                task_id,
                success: false,
                final_answer: String::new(),
                iterations,
                status: "aborted".into(),
            });
        }
        iterations += 1;

        if context.needs_summarisation() {
            tracing::info!(task_id = %task_id, "summarising context");
            let summary = summarise_context(&state.llm_router, &context, &task_id).await?;
            context.summarise(summary);
        }

        let model = state.llm_router.default_model().await;
        let tool_defs = state.tools.definitions();
        let request =
            LlmRequest::new(context.build_request_messages(), &model).with_tools(tool_defs);

        let response = state
            .llm_router
            .route(Some(&task_id), request, None)
            .await?;

        let _ = state
            .ws_broadcast
            .send(WsEvent::token(&task_id, &response.content));
        insert_log(
            &state.db,
            &task_id,
            iterations as i64,
            "llm_response",
            &json!({
                "provider": response.provider,
                "model": response.model,
                "input_tokens": response.input_tokens,
                "output_tokens": response.output_tokens,
                "content": response.content,
                "tool_calls": response.tool_calls.len(),
            }),
        )
        .await?;

        if plan.subtasks.is_empty() {
            plan = Plan::from_markdown(&response.content);
            if !plan.subtasks.is_empty() {
                insert_log(
                    &state.db,
                    &task_id,
                    iterations as i64,
                    "plan",
                    &json!({ "subtasks": plan.subtasks.iter().map(|s| &s.title).collect::<Vec<_>>() }),
                )
                .await?;
            }
        }

        let mut assistant_msg = ChatMessage::assistant(response.content.clone());
        assistant_msg.name = Some(response.provider.clone());
        context.push(assistant_msg);

        // Dispatch tool calls (if any).
        if !response.tool_calls.is_empty() {
            for call in &response.tool_calls {
                let _ = state.ws_broadcast.send(WsEvent::tool_call(&task_id, call));
                insert_log(
                    &state.db,
                    &task_id,
                    iterations as i64,
                    "tool_call",
                    &json!({ "name": call.name, "arguments": call.arguments }),
                )
                .await?;

                let outcome = state.tools.execute(call).await;
                let summary = outcome.summary();

                let _ = state
                    .ws_broadcast
                    .send(WsEvent::tool_result(&task_id, call, &outcome));
                insert_log(
                    &state.db,
                    &task_id,
                    iterations as i64,
                    "tool_result",
                    &json!({
                        "name": call.name,
                        "exit_code": outcome.exit_code(),
                        "duration_ms": outcome.duration_ms(),
                        "summary_preview": preview(&summary, 512),
                    }),
                )
                .await?;

                let mut tool_msg = ChatMessage::tool(summary, &call.id);
                tool_msg.name = Some(call.name.clone());
                context.push(tool_msg);
            }
            continue 'outer;
        }

        // No tool calls — check for explicit completion.
        if response.content.contains(COMPLETION_TAG) {
            final_answer = response
                .content
                .split(COMPLETION_TAG)
                .nth(1)
                .map(|s| s.trim().to_string())
                .unwrap_or_default();
            success = true;
            plan.complete_next();
            break;
        }

        // No tool calls and no completion — feed a nudge back into context.
        context.push(ChatMessage::user(
            "If you are finished, declare `TASK_COMPLETE:` followed by the final answer. \
             Otherwise, call a tool or provide more reasoning.",
        ));

        if started_at.elapsed() > std::time::Duration::from_secs(15 * 60) {
            tracing::warn!(task_id = %task_id, "wall-clock budget exceeded");
            break;
        }
    }

    if cfg.agent.reflection_enabled {
        if let Err(e) = reflect_and_store(
            &state.llm_router,
            &state.memory,
            &task_id,
            &context.full_log(),
        )
        .await
        {
            tracing::warn!(task_id = %task_id, "reflection failed: {e}");
        }
    }

    let status = if success { "completed" } else { "failed" };
    update_task_status(&state.db, &task_id, status).await?;
    let _ = state
        .ws_broadcast
        .send(WsEvent::final_answer(&task_id, &final_answer, status));

    Ok(TaskOutcome {
        task_id,
        success,
        final_answer,
        iterations,
        status: status.to_string(),
    })
}

async fn load_system_prompt(path: &std::path::Path) -> AppResult<String> {
    match tokio::fs::read_to_string(path).await {
        Ok(s) => Ok(s),
        Err(e) => Err(AppError::Config(format!(
            "could not read system prompt at {}: {e}",
            path.display()
        ))),
    }
}

async fn summarise_context(
    router: &LlmRouter,
    context: &AgentContext,
    task_id: &str,
) -> AppResult<String> {
    let model = router.default_model().await;
    let messages = context.full_log();
    let mut prompt = String::from(
        "Summarise the prior conversation in 4-6 sentences. Preserve any open \
         questions or pending subtasks. Be terse.\n\n",
    );
    for m in messages.iter().filter(|m| m.role != Role::System) {
        prompt.push_str(&format!("[{:?}] {}\n", m.role, m.content));
    }
    let req = LlmRequest::new(
        vec![
            ChatMessage::system("You are a concise summariser."),
            ChatMessage::user(prompt),
        ],
        &model,
    );
    let response = router.route(Some(task_id), req, None).await?;
    Ok(response.content)
}

async fn update_task_status(db: &sqlx::SqlitePool, task_id: &str, status: &str) -> AppResult<()> {
    let completed = matches!(status, "completed" | "failed" | "aborted");
    if completed {
        sqlx::query("UPDATE tasks SET status = ?, completed_at = ?, updated_at = ? WHERE id = ?")
            .bind(status)
            .bind(Utc::now())
            .bind(Utc::now())
            .bind(task_id)
            .execute(db)
            .await?;
    } else {
        sqlx::query("UPDATE tasks SET status = ?, updated_at = ? WHERE id = ?")
            .bind(status)
            .bind(Utc::now())
            .bind(task_id)
            .execute(db)
            .await?;
    }
    Ok(())
}

async fn insert_log(
    db: &sqlx::SqlitePool,
    task_id: &str,
    iteration: i64,
    phase: &str,
    metadata: &serde_json::Value,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO agent_logs (task_id, iteration, phase, content, metadata) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(task_id)
    .bind(iteration)
    .bind(phase)
    .bind(metadata.to_string())
    .bind(metadata.to_string())
    .execute(db)
    .await?;
    Ok(())
}

fn preview(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut out = String::with_capacity(max + 3);
    out.push_str(&s[..max]);
    out.push_str("...");
    out
}
