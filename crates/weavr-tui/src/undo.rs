//! Undo system for resolution changes.
//!
//! Provides an undo stack that tracks resolution changes and allows
//! reverting to previous states.

use weavr_core::{HunkId, Resolution};

/// Maximum depth of the undo stack.
const MAX_DEPTH: usize = 100;

/// An entry in the undo stack representing a previous resolution state.
#[derive(Debug, Clone)]
pub struct UndoEntry {
    /// The hunk that was modified.
    pub hunk_id: HunkId,
    /// The previous resolution (None if it was unresolved).
    pub previous_resolution: Option<Resolution>,
    /// Description of the action for status messages.
    pub action: String,
}

/// A stack of undo entries with automatic depth limiting.
#[derive(Debug, Clone, Default)]
pub struct UndoStack {
    entries: Vec<UndoEntry>,
}

impl UndoStack {
    /// Creates a new empty undo stack.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Pushes an undo entry onto the stack.
    ///
    /// If the stack exceeds the maximum depth, the oldest entry is removed.
    pub fn push(&mut self, hunk_id: HunkId, previous: Option<Resolution>, action: &str) {
        self.entries.push(UndoEntry {
            hunk_id,
            previous_resolution: previous,
            action: action.to_string(),
        });

        // Trim to max depth: only exceeds by at most 1 per push
        if self.entries.len() > MAX_DEPTH {
            self.entries.remove(0);
        }
    }

    /// Pops the most recent undo entry from the stack.
    pub fn pop(&mut self) -> Option<UndoEntry> {
        self.entries.pop()
    }

    /// Returns true if the stack is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clears all entries from the stack.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_stack_is_empty() {
        let stack = UndoStack::new();
        assert!(stack.is_empty());
    }

    #[test]
    fn push_and_pop() {
        let mut stack = UndoStack::new();
        stack.push(HunkId(1), None, "Test action");

        assert!(!stack.is_empty());

        let entry = stack.pop().unwrap();
        assert_eq!(entry.hunk_id, HunkId(1));
        assert!(entry.previous_resolution.is_none());
        assert_eq!(entry.action, "Test action");

        assert!(stack.is_empty());
    }

    #[test]
    fn pop_empty_returns_none() {
        let mut stack = UndoStack::new();
        assert!(stack.pop().is_none());
    }

    #[test]
    fn clear_removes_all_entries() {
        let mut stack = UndoStack::new();
        stack.push(HunkId(1), None, "Action 1");
        stack.push(HunkId(2), None, "Action 2");

        stack.clear();
        assert!(stack.is_empty());
    }

    #[test]
    fn lifo_order() {
        let mut stack = UndoStack::new();
        stack.push(HunkId(1), None, "First");
        stack.push(HunkId(2), None, "Second");
        stack.push(HunkId(3), None, "Third");

        assert_eq!(stack.pop().unwrap().action, "Third");
        assert_eq!(stack.pop().unwrap().action, "Second");
        assert_eq!(stack.pop().unwrap().action, "First");
    }
}
