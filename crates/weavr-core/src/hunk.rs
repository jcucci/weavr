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
#[non_exhaustive]
pub struct ConflictHunk {
    /// Unique identifier.
    pub id: HunkId,
    /// Left side content.
    pub left: HunkContent,
    /// Right side content.
    pub right: HunkContent,
    /// Base content (if 3-way merge).
    pub base: Option<HunkContent>,
    /// Additional sides beyond left and right (for N-way merges).
    #[serde(default)]
    pub extra_sides: Vec<HunkContent>,
    /// Additional bases beyond the primary base (for N-way merges).
    #[serde(default)]
    pub extra_bases: Vec<HunkContent>,
    /// Surrounding context.
    pub context: HunkContext,
    /// Resolution state.
    pub state: HunkState,
}

impl ConflictHunk {
    /// Creates a new `ConflictHunk` with the given sides, base, context, and state.
    ///
    /// `extra_sides` and `extra_bases` default to empty. Use the fields directly
    /// (within `weavr-core`) or the setter methods to populate them.
    #[must_use]
    pub fn new(
        id: HunkId,
        left: HunkContent,
        right: HunkContent,
        base: Option<HunkContent>,
        context: HunkContext,
        state: HunkState,
    ) -> Self {
        Self {
            id,
            left,
            right,
            base,
            extra_sides: vec![],
            extra_bases: vec![],
            context,
            state,
        }
    }

    /// Returns a hash of the hunk's content (all sides and bases).
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
        for side in &self.extra_sides {
            side.text.hash(&mut hasher);
        }
        for base in &self.extra_bases {
            base.text.hash(&mut hasher);
        }
        hasher.finish()
    }

    /// Iterates over all sides: left, right, then extra sides.
    pub fn sides(&self) -> impl Iterator<Item = &HunkContent> {
        std::iter::once(&self.left)
            .chain(std::iter::once(&self.right))
            .chain(self.extra_sides.iter())
    }

    /// Iterates over all bases: primary base (if present), then extra bases.
    pub fn bases(&self) -> impl Iterator<Item = &HunkContent> {
        self.base.iter().chain(self.extra_bases.iter())
    }

    /// Returns the side at the given index (0=left, 1=right, 2+=extra).
    #[must_use]
    pub fn side(&self, index: usize) -> Option<&HunkContent> {
        match index {
            0 => Some(&self.left),
            1 => Some(&self.right),
            i => self.extra_sides.get(i - 2),
        }
    }

    /// Returns the total number of sides (always >= 2).
    #[must_use]
    pub fn side_count(&self) -> usize {
        2 + self.extra_sides.len()
    }

    /// Returns the total number of bases.
    #[must_use]
    pub fn base_count(&self) -> usize {
        usize::from(self.base.is_some()) + self.extra_bases.len()
    }

    /// Returns true when this conflict has more than 2 sides.
    #[must_use]
    pub fn is_multi_sided(&self) -> bool {
        !self.extra_sides.is_empty()
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
            extra_sides: vec![],
            extra_bases: vec![],
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

    fn make_multi_hunk(extras: &[&str], extra_bases: &[&str]) -> ConflictHunk {
        let mut hunk = make_hunk("left", "right", Some("base"));
        hunk.extra_sides = extras
            .iter()
            .map(|s| HunkContent {
                text: (*s).to_string(),
            })
            .collect();
        hunk.extra_bases = extra_bases
            .iter()
            .map(|b| HunkContent {
                text: (*b).to_string(),
            })
            .collect();
        hunk
    }

    #[test]
    fn sides_iterator_two_sided() {
        let hunk = make_hunk("left", "right", None);
        let sides: Vec<&str> = hunk.sides().map(|s| s.text.as_str()).collect();
        assert_eq!(sides, vec!["left", "right"]);
    }

    #[test]
    fn sides_iterator_multi_sided() {
        let hunk = make_multi_hunk(&["third", "fourth"], &[]);
        let sides: Vec<&str> = hunk.sides().map(|s| s.text.as_str()).collect();
        assert_eq!(sides, vec!["left", "right", "third", "fourth"]);
    }

    #[test]
    fn bases_iterator_no_base() {
        let hunk = make_hunk("left", "right", None);
        assert_eq!(hunk.bases().count(), 0);
    }

    #[test]
    fn bases_iterator_single_base() {
        let hunk = make_hunk("left", "right", Some("base"));
        let bases: Vec<&str> = hunk.bases().map(|b| b.text.as_str()).collect();
        assert_eq!(bases, vec!["base"]);
    }

    #[test]
    fn bases_iterator_multi_base() {
        let hunk = make_multi_hunk(&[], &["base2", "base3"]);
        let bases: Vec<&str> = hunk.bases().map(|b| b.text.as_str()).collect();
        assert_eq!(bases, vec!["base", "base2", "base3"]);
    }

    #[test]
    fn side_by_index() {
        let hunk = make_multi_hunk(&["third"], &[]);
        assert_eq!(hunk.side(0).unwrap().text, "left");
        assert_eq!(hunk.side(1).unwrap().text, "right");
        assert_eq!(hunk.side(2).unwrap().text, "third");
        assert!(hunk.side(3).is_none());
    }

    #[test]
    fn side_count_two_sided() {
        let hunk = make_hunk("left", "right", None);
        assert_eq!(hunk.side_count(), 2);
    }

    #[test]
    fn side_count_multi_sided() {
        let hunk = make_multi_hunk(&["third", "fourth"], &[]);
        assert_eq!(hunk.side_count(), 4);
    }

    #[test]
    fn base_count_none() {
        let hunk = make_hunk("left", "right", None);
        assert_eq!(hunk.base_count(), 0);
    }

    #[test]
    fn base_count_with_extras() {
        let hunk = make_multi_hunk(&[], &["base2"]);
        assert_eq!(hunk.base_count(), 2);
    }

    #[test]
    fn is_multi_sided_false_for_two_sided() {
        let hunk = make_hunk("left", "right", None);
        assert!(!hunk.is_multi_sided());
    }

    #[test]
    fn is_multi_sided_true_for_three_sided() {
        let hunk = make_multi_hunk(&["third"], &[]);
        assert!(hunk.is_multi_sided());
    }

    #[test]
    fn content_hash_differs_with_extra_sides() {
        let a = make_hunk("left", "right", Some("base"));
        let b = make_multi_hunk(&["third"], &[]);
        assert_ne!(a.content_hash(), b.content_hash());
    }

    #[test]
    fn content_hash_differs_with_extra_bases() {
        let a = make_hunk("left", "right", Some("base"));
        let b = make_multi_hunk(&[], &["base2"]);
        assert_ne!(a.content_hash(), b.content_hash());
    }

    #[test]
    fn default_construction_matches_existing_behavior() {
        let hunk = make_hunk("left", "right", Some("base"));
        assert!(!hunk.is_multi_sided());
        assert_eq!(hunk.side_count(), 2);
        assert_eq!(hunk.base_count(), 1);
        assert!(hunk.extra_sides.is_empty());
        assert!(hunk.extra_bases.is_empty());
    }
}
