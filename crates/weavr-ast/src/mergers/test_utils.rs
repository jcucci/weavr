//! Shared test helpers used by all language-specific merger tests.

use weavr_core::{ConflictHunk, HunkContent, HunkContext, HunkId, HunkState};

/// Builds a `ConflictHunk` from raw text strings (convenience for tests).
pub(crate) fn make_hunk(left: &str, right: &str, base: Option<&str>) -> ConflictHunk {
    ConflictHunk::new(
        HunkId(1),
        HunkContent {
            text: left.to_string(),
        },
        HunkContent {
            text: right.to_string(),
        },
        base.map(|b| HunkContent {
            text: b.to_string(),
        }),
        HunkContext::default(),
        HunkState::default(),
    )
}
