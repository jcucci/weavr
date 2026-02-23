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
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

impl ConflictHunk {
    /// Returns a hash of the hunk's content (left, right, base text).
    ///
    /// Used for caching AI explanations — identical content produces the same
    /// hash, and any change in content automatically invalidates the cache.
    #[must_use]
    pub fn content_hash(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.left.text.hash(&mut hasher);
        self.right.text.hash(&mut hasher);
        if let Some(base) = &self.base {
            base.text.hash(&mut hasher);
        }
        hasher.finish()
    }
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

    fn make_hunk(left: &str, right: &str, base: Option<&str>) -> ConflictHunk {
        ConflictHunk {
            id: HunkId(1),
            left: HunkContent {
                text: left.to_string(),
            },
            right: HunkContent {
                text: right.to_string(),
            },
            base: base.map(|b| HunkContent {
                text: b.to_string(),
            }),
            context: HunkContext::default(),
            state: HunkState::default(),
        }
    }

    #[test]
    fn content_hash_stable_for_same_content() {
        let a = make_hunk("left", "right", Some("base"));
        let b = make_hunk("left", "right", Some("base"));
        assert_eq!(a.content_hash(), b.content_hash());
    }

    #[test]
    fn content_hash_differs_for_different_content() {
        let a = make_hunk("left", "right", Some("base"));
        let b = make_hunk("LEFT", "right", Some("base"));
        assert_ne!(a.content_hash(), b.content_hash());
    }

    #[test]
    fn content_hash_differs_with_and_without_base() {
        let a = make_hunk("left", "right", Some("base"));
        let b = make_hunk("left", "right", None);
        assert_ne!(a.content_hash(), b.content_hash());
    }

    #[test]
    fn content_hash_ignores_id_and_state() {
        let mut a = make_hunk("left", "right", None);
        let mut b = make_hunk("left", "right", None);
        a.id = HunkId(1);
        b.id = HunkId(99);
        b.state = HunkState::Invalid;
        assert_eq!(a.content_hash(), b.content_hash());
    }
}
