//! Streams agent events back to Telegram via message-edit updates.

use std::time::{Duration, Instant};

use teloxide::prelude::*;
use teloxide::types::{ChatId, MessageId};
use tokio::sync::broadcast::Receiver;

use crate::interfaces::api::ws::WsEvent;

/// Periodically edits a Telegram message with the latest progress.
pub struct ProgressReporter {
    bot: Bot,
    chat_id: ChatId,
    message_id: MessageId,
    task_id: String,
    rx: Receiver<WsEvent>,
}

impl ProgressReporter {
    /// Creates a new reporter bound to a Telegram chat message.
    pub fn new(
        bot: Bot,
        chat_id: ChatId,
        message_id: MessageId,
        task_id: String,
        rx: Receiver<WsEvent>,
    ) -> Self {
        Self {
            bot,
            chat_id,
            message_id,
            task_id,
            rx,
        }
    }

    /// Consumes the broadcast bus and updates the Telegram message at most
    /// once every 750 ms.
    pub async fn run(mut self) {
        let mut buffer = String::new();
        let mut last_update = Instant::now() - Duration::from_secs(60);
        const FLUSH_INTERVAL: Duration = Duration::from_millis(750);

        loop {
            let event = match tokio::time::timeout(Duration::from_secs(120), self.rx.recv()).await {
                Ok(Ok(ev)) => ev,
                Ok(Err(_)) => return,
                Err(_) => return,
            };
            let task_id = match &event {
                WsEvent::Token { task_id, .. }
                | WsEvent::ToolCall { task_id, .. }
                | WsEvent::ToolResult { task_id, .. }
                | WsEvent::Status { task_id, .. }
                | WsEvent::FinalAnswer { task_id, .. } => task_id.clone(),
            };
            if task_id != self.task_id {
                continue;
            }
            match &event {
                WsEvent::Token { text, .. } => {
                    buffer.push_str(text);
                }
                WsEvent::ToolCall { name, .. } => {
                    buffer.push_str(&format!("\n[tool {name}]"));
                }
                WsEvent::ToolResult { name, outcome, .. } => {
                    let summary = outcome.summary();
                    let truncated = if summary.len() > 200 {
                        format!("{}...", &summary[..200])
                    } else {
                        summary
                    };
                    buffer.push_str(&format!("\n[tool {name} done] {truncated}"));
                }
                WsEvent::Status { status, .. } => {
                    buffer.push_str(&format!("\n[status] {status}"));
                }
                WsEvent::FinalAnswer { text, status, .. } => {
                    buffer.push_str(&format!("\n[{status}] {text}"));
                    let _ = self
                        .bot
                        .edit_message_text(
                            self.chat_id,
                            self.message_id,
                            truncate_for_telegram(&buffer),
                        )
                        .await;
                    return;
                }
            }
            if last_update.elapsed() >= FLUSH_INTERVAL {
                last_update = Instant::now();
                let _ = self
                    .bot
                    .edit_message_text(
                        self.chat_id,
                        self.message_id,
                        truncate_for_telegram(&buffer),
                    )
                    .await;
            }
        }
    }
}

fn truncate_for_telegram(s: &str) -> String {
    const LIMIT: usize = 4000;
    if s.len() <= LIMIT {
        return s.to_string();
    }
    let mut out = String::with_capacity(LIMIT + 3);
    out.push_str(&s[s.len() - LIMIT..]);
    out
}
