//! Shared test helpers used by all language-specific merger tests.

use weavr_core::{ConflictHunk, HunkContent, HunkContext, HunkId, HunkState};

/// Builds a `ConflictHunk` from raw text strings (convenience for tests).
pub(crate) fn make_hunk(left: &str, right: &str, base: Option<&str>) -> ConflictHunk {
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
