//! Context-window truncation helpers. We use a very simple heuristic: assume
//! roughly four characters per token, then drop the oldest non-system messages
//! until the total estimated tokens fits within `budget`.

use crate::core::llm::types::{ChatMessage, Role};

/// Rough token estimator. Counts ~4 chars per token plus a fixed 4-token
/// overhead per message for role / metadata.
pub fn estimate_tokens(message: &ChatMessage) -> usize {
    4 + message.content.len() / 4
}

/// Estimates the token cost of an entire conversation.
pub fn estimate_total_tokens(messages: &[ChatMessage]) -> usize {
    messages.iter().map(estimate_tokens).sum()
}

/// Truncates the conversation to fit within `budget` tokens. System messages
/// are always preserved. Oldest user/assistant/tool messages are dropped
/// first until the budget is met. Returns the new slice + a count of how
/// many messages were dropped.
pub fn truncate_to_budget(messages: Vec<ChatMessage>, budget: usize) -> (Vec<ChatMessage>, usize) {
    let mut total = estimate_total_tokens(&messages);
    if total <= budget {
        return (messages, 0);
    }

    let mut kept: Vec<ChatMessage> = Vec::with_capacity(messages.len());
    let mut dropped = 0usize;
    let mut indices: Vec<usize> = (0..messages.len()).collect();

    // First pass: keep all system messages.
    let mut working: Vec<Option<ChatMessage>> = messages.into_iter().map(Some).collect();
    for slot in working.iter_mut() {
        if slot.as_ref().map(|m| m.role == Role::System).unwrap_or(false) {
            if let Some(m) = slot.take() {
                kept.push(m);
            }
        }
    }
    // Recompute non-system indices in original order.
    indices.retain(|i| working[*i].is_some());

    // Drop oldest until budget fits.
    while total > budget {
        let Some(idx) = indices.first().copied() else { break };
        indices.remove(0);
        if let Some(m) = working[idx].take() {
            total = total.saturating_sub(estimate_tokens(&m));
            dropped += 1;
        }
    }

    // Append remaining non-system messages in original order.
    let extras: Vec<ChatMessage> = working.into_iter().flatten().collect();
    let mut combined = kept;
    combined.extend(extras);
    (combined, dropped)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn does_not_truncate_within_budget() {
        let msgs = vec![ChatMessage::user("hello")];
        let (out, dropped) = truncate_to_budget(msgs.clone(), 1024);
        assert_eq!(out.len(), msgs.len());
        assert_eq!(dropped, 0);
    }

    #[test]
    fn drops_oldest_when_over_budget() {
        let mut msgs = vec![ChatMessage::system("you are RUNE")];
        for i in 0..50 {
            msgs.push(ChatMessage::user("x".repeat(400) + &i.to_string()));
        }
        let (out, dropped) = truncate_to_budget(msgs.clone(), 200);
        assert!(out.len() < msgs.len());
        assert!(dropped > 0);
        // System message preserved.
        assert!(matches!(out.first().unwrap().role, Role::System));
    }
}
