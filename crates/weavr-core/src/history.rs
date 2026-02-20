//! Action history for undo/redo support.
//!
//! Tracks resolution changes as reversible actions with two stacks:
//! an undo stack (actions performed) and a redo stack (actions undone).
//! When a new action is recorded, the redo stack is cleared.
//!
//! All types in this module are **stable** and covered by semantic versioning.

use std::collections::VecDeque;

use crate::{HunkId, Resolution};

/// Default maximum history depth.
const DEFAULT_MAX_DEPTH: usize = 100;

/// A reversible action performed on a merge session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// A resolution was set (or overridden) on a hunk.
    SetResolution {
        /// The hunk that was modified.
        hunk_id: HunkId,
        /// The previous resolution (`None` if unresolved before).
        old: Option<Resolution>,
        /// The new resolution that was applied.
        new: Resolution,
    },
    /// A resolution was cleared from a hunk.
    ClearResolution {
        /// The hunk that was cleared.
        hunk_id: HunkId,
        /// The resolution that was removed.
        old: Resolution,
    },
}

impl Action {
    /// Returns the hunk ID affected by this action.
    #[must_use]
    pub fn hunk_id(&self) -> HunkId {
        match self {
            Action::SetResolution { hunk_id, .. } | Action::ClearResolution { hunk_id, .. } => {
                *hunk_id
            }
        }
    }

    /// Returns a human-readable description of the action.
    #[must_use]
    pub fn description(&self) -> &'static str {
        match self {
            Action::SetResolution { old: None, .. } => "Set resolution",
            Action::SetResolution { old: Some(_), .. } => "Change resolution",
            Action::ClearResolution { .. } => "Clear resolution",
        }
    }
}

/// Tracks a history of actions for undo/redo support.
///
/// Uses two stacks: actions that have been performed (undo stack) and
/// actions that have been undone (redo stack). When a new action is
/// recorded, the redo stack is cleared.
#[derive(Debug, Clone)]
pub struct ActionHistory {
    undo_stack: VecDeque<Action>,
    redo_stack: Vec<Action>,
    max_depth: usize,
}

impl Default for ActionHistory {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionHistory {
    /// Creates a new empty action history with the default max depth (100).
    #[must_use]
    pub fn new() -> Self {
        Self {
            undo_stack: VecDeque::new(),
            redo_stack: Vec::new(),
            max_depth: DEFAULT_MAX_DEPTH,
        }
    }

    /// Creates a new empty action history with the specified max depth.
    #[must_use]
    pub fn with_max_depth(max_depth: usize) -> Self {
        Self {
            undo_stack: VecDeque::new(),
            redo_stack: Vec::new(),
            max_depth,
        }
    }

    /// Records a new action. Clears the redo stack.
    ///
    /// If the undo stack exceeds `max_depth`, the oldest entry is removed.
    pub fn record(&mut self, action: Action) {
        self.redo_stack.clear();
        self.undo_stack.push_back(action);
        if self.undo_stack.len() > self.max_depth {
            self.undo_stack.pop_front();
        }
    }

    /// Pops the most recent action from the undo stack and pushes it
    /// onto the redo stack.
    ///
    /// Returns the action so the caller can replay the inverse.
    /// Returns `None` if the undo stack is empty.
    pub fn undo(&mut self) -> Option<Action> {
        let action = self.undo_stack.pop_back()?;
        self.redo_stack.push(action.clone());
        Some(action)
    }

    /// Pops the most recent action from the redo stack and pushes it
    /// back onto the undo stack.
    ///
    /// Returns the action so the caller can replay it forward.
    /// Returns `None` if the redo stack is empty.
    pub fn redo(&mut self) -> Option<Action> {
        let action = self.redo_stack.pop()?;
        self.undo_stack.push_back(action.clone());
        Some(action)
    }

    /// Returns true if there are actions that can be undone.
    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Returns true if there are actions that can be redone.
    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Returns the number of actions in the undo stack.
    #[must_use]
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    /// Returns the number of actions in the redo stack.
    #[must_use]
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }

    /// Clears all history (both undo and redo stacks).
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    /// Returns the maximum depth of the history.
    #[must_use]
    pub fn max_depth(&self) -> usize {
        self.max_depth
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_resolution(content: &str) -> Resolution {
        Resolution::manual(content.to_string())
    }

    fn set_action(id: u32, old: Option<&str>, new: &str) -> Action {
        Action::SetResolution {
            hunk_id: HunkId(id),
            old: old.map(make_resolution),
            new: make_resolution(new),
        }
    }

    fn clear_action(id: u32, old: &str) -> Action {
        Action::ClearResolution {
            hunk_id: HunkId(id),
            old: make_resolution(old),
        }
    }

    // --- Basic functionality ---

    #[test]
    fn new_history_is_empty() {
        let h = ActionHistory::new();
        assert!(!h.can_undo());
        assert!(!h.can_redo());
        assert_eq!(h.undo_count(), 0);
        assert_eq!(h.redo_count(), 0);
    }

    #[test]
    fn default_creates_empty_history() {
        let h = ActionHistory::default();
        assert!(!h.can_undo());
        assert!(!h.can_redo());
        assert_eq!(h.max_depth(), DEFAULT_MAX_DEPTH);
    }

    #[test]
    fn record_enables_undo() {
        let mut h = ActionHistory::new();
        h.record(set_action(0, None, "new"));
        assert!(h.can_undo());
        assert!(!h.can_redo());
        assert_eq!(h.undo_count(), 1);
    }

    #[test]
    fn undo_returns_action_and_enables_redo() {
        let mut h = ActionHistory::new();
        h.record(set_action(0, None, "new"));

        let action = h.undo();
        assert!(action.is_some());
        assert_eq!(action.unwrap().hunk_id(), HunkId(0));
        assert!(!h.can_undo());
        assert!(h.can_redo());
    }

    #[test]
    fn redo_returns_action_and_enables_undo() {
        let mut h = ActionHistory::new();
        h.record(set_action(0, None, "new"));
        h.undo();

        let action = h.redo();
        assert!(action.is_some());
        assert_eq!(action.unwrap().hunk_id(), HunkId(0));
        assert!(h.can_undo());
        assert!(!h.can_redo());
    }

    // --- Edge cases ---

    #[test]
    fn undo_at_start_is_noop() {
        let mut h = ActionHistory::new();
        assert!(h.undo().is_none());
    }

    #[test]
    fn redo_at_end_is_noop() {
        let mut h = ActionHistory::new();
        assert!(h.redo().is_none());
    }

    #[test]
    fn redo_without_prior_undo_is_noop() {
        let mut h = ActionHistory::new();
        h.record(set_action(0, None, "new"));
        assert!(h.redo().is_none());
    }

    // --- New action clears redo ---

    #[test]
    fn new_action_clears_redo_stack() {
        let mut h = ActionHistory::new();
        h.record(set_action(0, None, "first"));
        h.undo();
        assert!(h.can_redo());

        h.record(set_action(0, None, "second"));
        assert!(!h.can_redo());
        assert_eq!(h.redo_count(), 0);
    }

    // --- LIFO ordering ---

    #[test]
    fn undo_is_lifo() {
        let mut h = ActionHistory::new();
        h.record(set_action(1, None, "first"));
        h.record(set_action(2, None, "second"));
        h.record(set_action(3, None, "third"));

        assert_eq!(h.undo().unwrap().hunk_id(), HunkId(3));
        assert_eq!(h.undo().unwrap().hunk_id(), HunkId(2));
        assert_eq!(h.undo().unwrap().hunk_id(), HunkId(1));
        assert!(h.undo().is_none());
    }

    #[test]
    fn redo_replays_in_order() {
        let mut h = ActionHistory::new();
        h.record(set_action(1, None, "first"));
        h.record(set_action(2, None, "second"));
        h.undo();
        h.undo();

        assert_eq!(h.redo().unwrap().hunk_id(), HunkId(1));
        assert_eq!(h.redo().unwrap().hunk_id(), HunkId(2));
        assert!(h.redo().is_none());
    }

    // --- Max depth ---

    #[test]
    fn max_depth_trims_oldest() {
        let mut h = ActionHistory::with_max_depth(3);
        for i in 0..5 {
            h.record(set_action(i, None, &format!("r{i}")));
        }
        assert_eq!(h.undo_count(), 3);

        // Most recent entries remain
        assert_eq!(h.undo().unwrap().hunk_id(), HunkId(4));
        assert_eq!(h.undo().unwrap().hunk_id(), HunkId(3));
        assert_eq!(h.undo().unwrap().hunk_id(), HunkId(2));
        assert!(h.undo().is_none());
    }

    #[test]
    fn with_max_depth_sets_depth() {
        let h = ActionHistory::with_max_depth(50);
        assert_eq!(h.max_depth(), 50);
    }

    // --- Cross-hunk undo ---

    #[test]
    fn undo_works_across_different_hunks() {
        let mut h = ActionHistory::new();
        h.record(set_action(0, None, "hunk0"));
        h.record(set_action(5, None, "hunk5"));
        h.record(set_action(2, None, "hunk2"));

        assert_eq!(h.undo().unwrap().hunk_id(), HunkId(2));
        assert_eq!(h.undo().unwrap().hunk_id(), HunkId(5));
        assert_eq!(h.undo().unwrap().hunk_id(), HunkId(0));
    }

    // --- ClearResolution variant ---

    #[test]
    fn clear_resolution_action_roundtrip() {
        let mut h = ActionHistory::new();
        h.record(clear_action(0, "was_here"));

        let action = h.undo().unwrap();
        assert!(matches!(action, Action::ClearResolution { .. }));
        assert_eq!(action.hunk_id(), HunkId(0));

        let action = h.redo().unwrap();
        assert!(matches!(action, Action::ClearResolution { .. }));
    }

    // --- Clear ---

    #[test]
    fn clear_empties_both_stacks() {
        let mut h = ActionHistory::new();
        h.record(set_action(0, None, "a"));
        h.record(set_action(1, None, "b"));
        h.undo();

        assert!(h.can_undo());
        assert!(h.can_redo());

        h.clear();
        assert!(!h.can_undo());
        assert!(!h.can_redo());
        assert_eq!(h.undo_count(), 0);
        assert_eq!(h.redo_count(), 0);
    }

    // --- Action methods ---

    #[test]
    fn action_hunk_id() {
        assert_eq!(set_action(42, None, "x").hunk_id(), HunkId(42));
        assert_eq!(clear_action(7, "x").hunk_id(), HunkId(7));
    }

    #[test]
    fn action_description() {
        assert_eq!(set_action(0, None, "a").description(), "Set resolution");
        assert_eq!(
            set_action(0, Some("old"), "new").description(),
            "Change resolution"
        );
        assert_eq!(clear_action(0, "a").description(), "Clear resolution");
    }

    // --- Multiple undo/redo cycles ---

    #[test]
    fn multiple_undo_redo_cycles() {
        let mut h = ActionHistory::new();
        h.record(set_action(0, None, "a"));

        // Cycle 1
        h.undo();
        h.redo();
        assert_eq!(h.undo_count(), 1);
        assert_eq!(h.redo_count(), 0);

        // Cycle 2
        h.undo();
        h.redo();
        assert_eq!(h.undo_count(), 1);
        assert_eq!(h.redo_count(), 0);

        // Action is preserved through cycles
        let action = h.undo().unwrap();
        assert_eq!(action.hunk_id(), HunkId(0));
    }
}
