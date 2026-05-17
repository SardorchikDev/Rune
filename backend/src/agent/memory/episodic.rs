//! Episodic memory: a bounded ring buffer of recent agent messages.

use std::collections::VecDeque;

use tokio::sync::RwLock;

use crate::core::llm::types::ChatMessage;

/// Volatile log of recent messages, bounded by capacity.
pub struct EpisodicMemory {
    capacity: usize,
    entries: RwLock<VecDeque<ChatMessage>>,
}

impl EpisodicMemory {
    /// Creates a new episodic log with the given capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            entries: RwLock::new(VecDeque::new()),
        }
    }

    /// Appends a new message, evicting the oldest entry once the capacity is
    /// exceeded.
    pub async fn push(&self, message: ChatMessage) {
        let mut guard = self.entries.write().await;
        if guard.len() >= self.capacity {
            guard.pop_front();
        }
        guard.push_back(message);
    }

    /// Returns a snapshot of the current contents.
    pub async fn snapshot(&self) -> Vec<ChatMessage> {
        self.entries.read().await.iter().cloned().collect()
    }

    /// Current number of stored messages.
    pub async fn len(&self) -> usize {
        self.entries.read().await.len()
    }

    /// Whether the log is empty.
    pub async fn is_empty(&self) -> bool {
        self.entries.read().await.is_empty()
    }
}
