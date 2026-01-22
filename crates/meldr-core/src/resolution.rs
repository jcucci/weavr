//! Resolution types for merge conflicts.
//!
//! All types in this module are **stable** and covered by semantic versioning.

use serde::{Deserialize, Serialize};

use crate::hunk::ConflictHunk;

/// Order for `AcceptBoth` strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BothOrder {
    /// Left content first, then right.
    #[default]
    LeftThenRight,
    /// Right content first, then left.
    RightThenLeft,
}

/// Options for the `AcceptBoth` strategy.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AcceptBothOptions {
    /// Order of content combination.
    pub order: BothOrder,
    /// Remove duplicate lines.
    pub deduplicate: bool,
    /// Normalize whitespace before comparison.
    pub trim_whitespace: bool,
}

/// Describes the source/method of a resolution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResolutionStrategyKind {
    /// Use left content verbatim.
    AcceptLeft,
    /// Use right content verbatim.
    AcceptRight,
    /// Combine left and right.
    AcceptBoth(AcceptBothOptions),
    /// User-provided content.
    Manual,
    /// Language-specific AST merge.
    AstMerged {
        /// The language used for AST merging.
        language: String,
    },
    /// AI-generated suggestion.
    AiSuggested {
        /// The AI provider name.
        provider: String,
    },
}

/// Source of a resolution.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ResolutionSource {
    /// Resolution made by user.
    #[default]
    User,
    /// Resolution suggested by AI.
    Ai,
    /// Resolution from AST analysis.
    Ast,
}

/// Metadata about a resolution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ResolutionMetadata {
    /// Source of the resolution.
    pub source: ResolutionSource,
    /// Optional notes.
    pub notes: Option<String>,
}

/// An explicit decision applied to a hunk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resolution {
    /// How the resolution was chosen.
    pub kind: ResolutionStrategyKind,
    /// The resolved content.
    pub content: String,
    /// Additional metadata.
    pub metadata: ResolutionMetadata,
}

impl Resolution {
    /// Create a resolution that accepts the left (`HEAD`/ours) content verbatim.
    #[must_use]
    pub fn accept_left(hunk: &ConflictHunk) -> Resolution {
        Resolution {
            kind: ResolutionStrategyKind::AcceptLeft,
            content: hunk.left.text.clone(),
            metadata: ResolutionMetadata::default(),
        }
    }

    /// Create a resolution that accepts the right (`MERGE_HEAD`/theirs) content verbatim.
    #[must_use]
    pub fn accept_right(hunk: &ConflictHunk) -> Resolution {
        Resolution {
            kind: ResolutionStrategyKind::AcceptRight,
            content: hunk.right.text.clone(),
            metadata: ResolutionMetadata::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hunk::{HunkContent, HunkContext, HunkId, HunkState};

    fn test_hunk(left: &str, right: &str) -> ConflictHunk {
        ConflictHunk {
            id: HunkId(1),
            left: HunkContent {
                text: left.to_string(),
            },
            right: HunkContent {
                text: right.to_string(),
            },
            base: None,
            context: HunkContext::default(),
            state: HunkState::default(),
        }
    }

    #[test]
    fn both_order_default() {
        assert_eq!(BothOrder::default(), BothOrder::LeftThenRight);
    }

    #[test]
    fn accept_both_options_default() {
        let opts = AcceptBothOptions::default();
        assert_eq!(opts.order, BothOrder::LeftThenRight);
        assert!(!opts.deduplicate);
        assert!(!opts.trim_whitespace);
    }

    #[test]
    fn resolution_source_default() {
        assert_eq!(ResolutionSource::default(), ResolutionSource::User);
    }

    #[test]
    fn resolution_metadata_default() {
        let meta = ResolutionMetadata::default();
        assert_eq!(meta.source, ResolutionSource::User);
        assert!(meta.notes.is_none());
    }

    #[test]
    fn resolution_creation() {
        let resolution = Resolution {
            kind: ResolutionStrategyKind::AcceptLeft,
            content: String::from("left content"),
            metadata: ResolutionMetadata::default(),
        };
        assert_eq!(resolution.kind, ResolutionStrategyKind::AcceptLeft);
    }

    #[test]
    fn accept_left_returns_exact_left_content() {
        let hunk = test_hunk("left content", "right content");
        let resolution = Resolution::accept_left(&hunk);
        assert_eq!(resolution.content, "left content");
        assert_eq!(resolution.kind, ResolutionStrategyKind::AcceptLeft);
    }

    #[test]
    fn accept_right_returns_exact_right_content() {
        let hunk = test_hunk("left content", "right content");
        let resolution = Resolution::accept_right(&hunk);
        assert_eq!(resolution.content, "right content");
        assert_eq!(resolution.kind, ResolutionStrategyKind::AcceptRight);
    }

    #[test]
    fn accept_left_with_empty_content() {
        let hunk = test_hunk("", "right content");
        let resolution = Resolution::accept_left(&hunk);
        assert_eq!(resolution.content, "");
        assert_eq!(resolution.kind, ResolutionStrategyKind::AcceptLeft);
    }

    #[test]
    fn accept_right_with_empty_content() {
        let hunk = test_hunk("left content", "");
        let resolution = Resolution::accept_right(&hunk);
        assert_eq!(resolution.content, "");
        assert_eq!(resolution.kind, ResolutionStrategyKind::AcceptRight);
    }

    #[test]
    fn accept_left_is_idempotent() {
        let hunk = test_hunk("left content", "right content");
        let res1 = Resolution::accept_left(&hunk);
        let res2 = Resolution::accept_left(&hunk);
        assert_eq!(res1, res2);
    }

    #[test]
    fn accept_right_is_idempotent() {
        let hunk = test_hunk("left content", "right content");
        let res1 = Resolution::accept_right(&hunk);
        let res2 = Resolution::accept_right(&hunk);
        assert_eq!(res1, res2);
    }
}
