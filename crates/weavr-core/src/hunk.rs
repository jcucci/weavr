//! Conflict hunk types.
//!
//! All types in this module are **stable** and covered by semantic versioning.

use serde::{Deserialize, Serialize};

use crate::Resolution;

/// Unique identifier for a conflict hunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HunkId(pub u32);

/// Content within a conflict hunk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HunkContent {
    /// The conflicting text.
    pub text: String,
}

/// Context surrounding a conflict hunk.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct HunkContext {
    /// Lines before the conflict.
    pub before: Vec<String>,
    /// Lines after the conflict.
    pub after: Vec<String>,
    /// Starting line in left version.
    pub start_line_left: usize,
    /// Starting line in right version.
    pub start_line_right: usize,
}

/// State of a single hunk.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum HunkState {
    /// No resolution chosen.
    #[default]
    Unresolved,
    /// Candidate resolutions available.
    Proposed(Vec<Resolution>),
    /// Resolution selected.
    Resolved(Resolution),
    /// Resolution rejected by validation.
    Invalid,
}

/// A contiguous region of conflicting content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConflictHunk {
    /// Unique identifier.
    pub id: HunkId,
    /// Left side content.
    pub left: HunkContent,
    /// Right side content.
    pub right: HunkContent,
    /// Base content (if 3-way merge).
    pub base: Option<HunkContent>,
    /// Surrounding context.
    pub context: HunkContext,
    /// Resolution state.
    pub state: HunkState,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hunk_id_equality() {
        assert_eq!(HunkId(1), HunkId(1));
        assert_ne!(HunkId(1), HunkId(2));
    }

    #[test]
    fn hunk_id_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(HunkId(1));
        set.insert(HunkId(2));
        assert!(set.contains(&HunkId(1)));
        assert!(!set.contains(&HunkId(3)));
    }

    #[test]
    fn hunk_state_default() {
        assert_eq!(HunkState::default(), HunkState::Unresolved);
    }

    #[test]
    fn hunk_context_default() {
        let ctx = HunkContext::default();
        assert!(ctx.before.is_empty());
        assert!(ctx.after.is_empty());
        assert_eq!(ctx.start_line_left, 0);
        assert_eq!(ctx.start_line_right, 0);
    }
}
