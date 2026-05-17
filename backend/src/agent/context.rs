//! In-flight context window manager for the agent loop.

use crate::core::llm::types::{ChatMessage, Role};
use crate::utils::truncate;

/// Conversation context with budget enforcement.
#[derive(Debug, Clone)]
pub struct AgentContext {
    system_prompt: String,
    messages: Vec<ChatMessage>,
    budget_tokens: usize,
    summarise_threshold: usize,
    memories: Vec<String>,
}

impl AgentContext {
    /// Creates a new context bound to a budget.
    pub fn new(
        system_prompt: impl Into<String>,
        budget_tokens: usize,
        summarise_threshold: usize,
    ) -> Self {
        Self {
            system_prompt: system_prompt.into(),
            messages: Vec::new(),
            budget_tokens: budget_tokens.max(256),
            summarise_threshold: summarise_threshold.max(1),
            memories: Vec::new(),
        }
    }

    /// Appends a new message.
    pub fn push(&mut self, message: ChatMessage) {
        self.messages.push(message);
    }

    /// Replaces the recalled memory list (called from the recall phase).
    pub fn set_memories(&mut self, memories: Vec<String>) {
        self.memories = memories;
    }

    /// Total non-system messages currently in flight.
    pub fn message_count(&self) -> usize {
        self.messages.iter().filter(|m| m.role != Role::System).count()
    }

    /// Whether the auto-summarisation threshold has been exceeded.
    pub fn needs_summarisation(&self) -> bool {
        self.message_count() >= self.summarise_threshold
    }

    /// Replaces all non-system messages with a single summary user message.
    pub fn summarise(&mut self, summary: impl Into<String>) {
        let summary_msg = ChatMessage::user(format!(
            "[CONTEXT SUMMARY OF PRIOR MESSAGES]\n{}",
            summary.into()
        ));
        self.messages.retain(|m| m.role == Role::System);
        self.messages.push(summary_msg);
    }

    /// Builds the final messages slice for an LLM request. Prepends the
    /// system prompt (augmented with any recalled memories) and truncates to
    /// the budget.
    pub fn build_request_messages(&self) -> Vec<ChatMessage> {
        let mut system = String::new();
        system.push_str(&self.system_prompt);
        if !self.memories.is_empty() {
            system.push_str("\n\n# Memory context\nThe following recalled memories may be relevant. Treat them as background — do not assume they are still true.\n");
            for (i, m) in self.memories.iter().enumerate() {
                system.push_str(&format!("\n[{}] {}", i + 1, m));
            }
        }

        let mut combined = Vec::with_capacity(self.messages.len() + 1);
        combined.push(ChatMessage::system(system));
        combined.extend(self.messages.iter().cloned());

        let (truncated, dropped) =
            truncate::truncate_to_budget(combined, self.budget_tokens);
        if dropped > 0 {
            tracing::debug!(dropped, "truncated context to fit budget");
        }
        truncated
    }

    /// Returns the full message log (without truncation). Used by the
    /// reflector to compute a summary of the entire task.
    pub fn full_log(&self) -> Vec<ChatMessage> {
        let mut combined = Vec::with_capacity(self.messages.len() + 1);
        combined.push(ChatMessage::system(self.system_prompt.clone()));
        combined.extend(self.messages.iter().cloned());
        combined
    }
}
