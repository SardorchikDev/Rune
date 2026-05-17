//! Post-task LLM summarisation + memory write-back.

use std::sync::Arc;

use crate::agent::memory::MemoryStore;
use crate::core::llm::types::{ChatMessage, LlmRequest};
use crate::core::llm::LlmRouter;
use crate::error::AppResult;

/// Runs the reflection pass for a finished task. Generates a short
/// summary via the LLM, embeds it, and stores it in the memory store.
pub async fn reflect_and_store(
    router: &LlmRouter,
    memory: &Arc<MemoryStore>,
    task_id: &str,
    transcript: &[ChatMessage],
) -> AppResult<String> {
    let model = router.default_model().await;

    let mut prompt = String::new();
    prompt.push_str(
        "Summarise the following agent transcript. Capture: (1) the user's task, \
         (2) the steps taken, (3) the final outcome, (4) lessons or facts worth \
         remembering for next time. Be concise — 5-8 sentences. Do not include \
         tool-call JSON.\n\n",
    );
    for m in transcript {
        let label = match m.role {
            crate::core::llm::types::Role::System => "system",
            crate::core::llm::types::Role::User => "user",
            crate::core::llm::types::Role::Assistant => "assistant",
            crate::core::llm::types::Role::Tool => "tool",
        };
        prompt.push_str(&format!("[{label}] {}\n", m.content));
    }

    let req = LlmRequest::new(
        vec![
            ChatMessage::system("You are a concise summariser."),
            ChatMessage::user(prompt),
        ],
        &model,
    );
    let response = router.route(Some(task_id), req, None).await?;
    let summary = response.content.trim().to_string();

    memory.store(task_id, &summary).await?;
    Ok(summary)
}
