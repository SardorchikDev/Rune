//! Task decomposition + lightweight subtask graph.

use serde::{Deserialize, Serialize};

/// A single subtask returned by the planner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subtask {
    /// Sequential index in the plan.
    pub index: usize,
    /// Short imperative title.
    pub title: String,
    /// Optional detailed description.
    #[serde(default)]
    pub details: String,
    /// Whether this subtask is complete.
    #[serde(default)]
    pub completed: bool,
}

/// A naive plan made of a list of subtasks. We intentionally keep this
/// simple — the agent loop is the source of truth for execution, not the
/// plan, but recording the plan is useful for the dashboard.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Plan {
    /// Ordered subtasks.
    pub subtasks: Vec<Subtask>,
}

impl Plan {
    /// Parses a markdown numbered list into a [`Plan`]. Lines that don't
    /// start with `<number>.` are ignored.
    pub fn from_markdown(text: &str) -> Self {
        let mut subtasks = Vec::new();
        for line in text.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix(|c: char| c.is_ascii_digit()) {
                if let Some(rest) = rest.trim_start_matches('.').strip_prefix(' ') {
                    subtasks.push(Subtask {
                        index: subtasks.len() + 1,
                        title: rest.to_string(),
                        details: String::new(),
                        completed: false,
                    });
                }
            }
        }
        Self { subtasks }
    }

    /// Marks the first non-completed subtask as done. Returns the title of
    /// the subtask that was completed, if any.
    pub fn complete_next(&mut self) -> Option<String> {
        for s in &mut self.subtasks {
            if !s.completed {
                s.completed = true;
                return Some(s.title.clone());
            }
        }
        None
    }

    /// Whether every subtask is done.
    pub fn is_done(&self) -> bool {
        !self.subtasks.is_empty() && self.subtasks.iter().all(|s| s.completed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_numbered_list() {
        let plan = Plan::from_markdown(
            "Plan:\n1. Inspect workspace\n2. Run the script\n3. Report results",
        );
        assert_eq!(plan.subtasks.len(), 3);
        assert_eq!(plan.subtasks[0].title, "Inspect workspace");
        assert_eq!(plan.subtasks[2].index, 3);
    }

    #[test]
    fn complete_next_progresses() {
        let mut plan = Plan::from_markdown("1. a\n2. b");
        assert_eq!(plan.complete_next(), Some("a".into()));
        assert_eq!(plan.complete_next(), Some("b".into()));
        assert_eq!(plan.complete_next(), None);
        assert!(plan.is_done());
    }
}
