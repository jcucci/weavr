//! Resolution types for merge conflicts.
//!
//! All types in this module are **stable** and covered by semantic versioning.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::hunk::ConflictHunk;

/// Simple concatenation with proper newline handling.
fn combine_simple(first: &str, second: &str) -> String {
    if first.ends_with('\n') {
        format!("{first}{second}")
    } else {
        format!("{first}\n{second}")
    }
}

/// Combine with deduplication, preserving first occurrence.
fn combine_with_dedup(first: &str, second: &str, trim_whitespace: bool) -> String {
    let mut seen: HashSet<String> = HashSet::new();
    let mut result_lines: Vec<&str> = Vec::new();

    // Process first side - all lines are "first occurrence"
    for line in first.lines() {
        let key = if trim_whitespace {
            line.trim().to_string()
        } else {
            line.to_string()
        };
        seen.insert(key);
        result_lines.push(line);
    }

    // Process second side - skip duplicates
    for line in second.lines() {
        let key = if trim_whitespace {
            line.trim().to_string()
        } else {
            line.to_string()
        };

        if seen.insert(key) {
            result_lines.push(line);
        }
    }

    // Join with newlines
    let mut result = result_lines.join("\n");

    // Preserve trailing newline if either input had one
    let first_has_trailing = first.ends_with('\n');
    let second_has_trailing = second.ends_with('\n');
    if first_has_trailing || second_has_trailing {
        result.push('\n');
    }

    result
}

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
    /// Confidence score for AI-generated resolutions (0-100 percentage).
    /// Only meaningful when `source` is `ResolutionSource::Ai`.
    pub confidence: Option<u8>,
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

    /// Create a resolution that combines both left and right content.
    ///
    /// Options control the combination behavior:
    /// - `order`: Whether left or right content appears first
    /// - `deduplicate`: Remove lines that appear identically in both sides
    /// - `trim_whitespace`: Normalize whitespace before deduplication comparison
    #[must_use]
    pub fn accept_both(hunk: &ConflictHunk, options: &AcceptBothOptions) -> Resolution {
        // Determine ordering
        let (first, second) = match options.order {
            BothOrder::LeftThenRight => (&hunk.left.text, &hunk.right.text),
            BothOrder::RightThenLeft => (&hunk.right.text, &hunk.left.text),
        };

        // Handle empty content cases
        if first.is_empty() && second.is_empty() {
            return Resolution {
                kind: ResolutionStrategyKind::AcceptBoth(options.clone()),
                content: String::new(),
                metadata: ResolutionMetadata::default(),
            };
        }
        if first.is_empty() {
            return Resolution {
                kind: ResolutionStrategyKind::AcceptBoth(options.clone()),
                content: second.clone(),
                metadata: ResolutionMetadata::default(),
            };
        }
        if second.is_empty() {
            return Resolution {
                kind: ResolutionStrategyKind::AcceptBoth(options.clone()),
                content: first.clone(),
                metadata: ResolutionMetadata::default(),
            };
        }

        // Combine content
        let content = if options.deduplicate {
            combine_with_dedup(first, second, options.trim_whitespace)
        } else {
            combine_simple(first, second)
        };

        Resolution {
            kind: ResolutionStrategyKind::AcceptBoth(options.clone()),
            content,
            metadata: ResolutionMetadata::default(),
        }
    }

    /// Create a resolution with user-provided content.
    ///
    /// This is the escape hatch for complex merges where automated strategies
    /// don't fit. The content is preserved exactly as provided - no trimming
    /// or normalization is performed.
    ///
    /// # Arguments
    /// * `content` - Arbitrary content, preserved exactly
    ///
    /// # Examples
    /// ```
    /// use weavr_core::Resolution;
    ///
    /// // User provides custom merged content
    /// let resolution = Resolution::manual("custom merged content".to_string());
    /// assert_eq!(resolution.content, "custom merged content");
    /// ```
    #[must_use]
    pub fn manual(content: String) -> Resolution {
        Resolution {
            kind: ResolutionStrategyKind::Manual,
            content,
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

    // accept_both tests

    #[test]
    fn accept_both_left_then_right_order() {
        let hunk = test_hunk("left\n", "right\n");
        let opts = AcceptBothOptions::default();
        let resolution = Resolution::accept_both(&hunk, &opts);
        assert_eq!(resolution.content, "left\nright\n");
    }

    #[test]
    fn accept_both_right_then_left_order() {
        let hunk = test_hunk("left\n", "right\n");
        let opts = AcceptBothOptions {
            order: BothOrder::RightThenLeft,
            ..Default::default()
        };
        let resolution = Resolution::accept_both(&hunk, &opts);
        assert_eq!(resolution.content, "right\nleft\n");
    }

    #[test]
    fn accept_both_dedup_removes_exact_matches() {
        let hunk = test_hunk("import foo\nimport bar\n", "import bar\nimport baz\n");
        let opts = AcceptBothOptions {
            deduplicate: true,
            ..Default::default()
        };
        let resolution = Resolution::accept_both(&hunk, &opts);
        assert_eq!(resolution.content, "import foo\nimport bar\nimport baz\n");
    }

    #[test]
    fn accept_both_no_dedup_keeps_duplicates() {
        let hunk = test_hunk("import foo\nimport bar\n", "import bar\nimport baz\n");
        let opts = AcceptBothOptions::default();
        let resolution = Resolution::accept_both(&hunk, &opts);
        assert_eq!(
            resolution.content,
            "import foo\nimport bar\nimport bar\nimport baz\n"
        );
    }

    #[test]
    fn accept_both_dedup_preserves_first_occurrence() {
        let hunk = test_hunk("  indented\n", "indented\n");
        let opts = AcceptBothOptions {
            deduplicate: true,
            trim_whitespace: true,
            ..Default::default()
        };
        let resolution = Resolution::accept_both(&hunk, &opts);
        // First occurrence (with indent) is preserved
        assert_eq!(resolution.content, "  indented\n");
    }

    #[test]
    fn accept_both_trim_whitespace_for_comparison() {
        let hunk = test_hunk("  foo  \n", "foo\n");
        let opts = AcceptBothOptions {
            deduplicate: true,
            trim_whitespace: true,
            ..Default::default()
        };
        let resolution = Resolution::accept_both(&hunk, &opts);
        // Only one "foo" line, preserving first occurrence's whitespace
        assert_eq!(resolution.content, "  foo  \n");
    }

    #[test]
    fn accept_both_no_trim_keeps_whitespace_variants() {
        let hunk = test_hunk("  foo  \n", "foo\n");
        let opts = AcceptBothOptions {
            deduplicate: true,
            trim_whitespace: false,
            ..Default::default()
        };
        let resolution = Resolution::accept_both(&hunk, &opts);
        // Different whitespace = different lines
        assert_eq!(resolution.content, "  foo  \nfoo\n");
    }

    #[test]
    fn accept_both_left_empty() {
        let hunk = test_hunk("", "right content\n");
        let opts = AcceptBothOptions::default();
        let resolution = Resolution::accept_both(&hunk, &opts);
        assert_eq!(resolution.content, "right content\n");
    }

    #[test]
    fn accept_both_right_empty() {
        let hunk = test_hunk("left content\n", "");
        let opts = AcceptBothOptions::default();
        let resolution = Resolution::accept_both(&hunk, &opts);
        assert_eq!(resolution.content, "left content\n");
    }

    #[test]
    fn accept_both_both_empty() {
        let hunk = test_hunk("", "");
        let opts = AcceptBothOptions::default();
        let resolution = Resolution::accept_both(&hunk, &opts);
        assert_eq!(resolution.content, "");
    }

    #[test]
    fn accept_both_sets_correct_kind() {
        let hunk = test_hunk("left\n", "right\n");
        let opts = AcceptBothOptions {
            order: BothOrder::RightThenLeft,
            deduplicate: true,
            trim_whitespace: true,
        };
        let resolution = Resolution::accept_both(&hunk, &opts);

        match resolution.kind {
            ResolutionStrategyKind::AcceptBoth(stored_opts) => {
                assert_eq!(stored_opts.order, BothOrder::RightThenLeft);
                assert!(stored_opts.deduplicate);
                assert!(stored_opts.trim_whitespace);
            }
            _ => panic!("Expected AcceptBoth kind"),
        }
    }

    #[test]
    fn accept_both_metadata_defaults_to_user_source() {
        let hunk = test_hunk("left\n", "right\n");
        let opts = AcceptBothOptions::default();
        let resolution = Resolution::accept_both(&hunk, &opts);
        assert_eq!(resolution.metadata.source, ResolutionSource::User);
        assert!(resolution.metadata.notes.is_none());
    }

    #[test]
    fn accept_both_is_idempotent() {
        let hunk = test_hunk("import foo\n", "import bar\n");
        let opts = AcceptBothOptions {
            deduplicate: true,
            ..Default::default()
        };
        let res1 = Resolution::accept_both(&hunk, &opts);
        let res2 = Resolution::accept_both(&hunk, &opts);
        assert_eq!(res1, res2);
    }

    // manual() tests

    #[test]
    fn manual_preserves_content_exactly() {
        let content = "user provided content";
        let resolution = Resolution::manual(content.to_string());
        assert_eq!(resolution.content, content);
        assert_eq!(resolution.kind, ResolutionStrategyKind::Manual);
    }

    #[test]
    fn manual_allows_empty_content() {
        let resolution = Resolution::manual(String::new());
        assert_eq!(resolution.content, "");
        assert_eq!(resolution.kind, ResolutionStrategyKind::Manual);
    }

    #[test]
    fn manual_preserves_content_with_conflict_markers() {
        // Edge case: user content may contain conflict markers
        // This is valid but will fail validation later
        let content = "<<<<<<< HEAD\nfoo\n=======\nbar\n>>>>>>>";
        let resolution = Resolution::manual(content.to_string());
        assert_eq!(resolution.content, content);
    }

    #[test]
    fn manual_preserves_multiline_content() {
        let content = "line1\nline2\nline3\n";
        let resolution = Resolution::manual(content.to_string());
        assert_eq!(resolution.content, content);
    }

    #[test]
    fn manual_preserves_crlf_line_endings() {
        let content = "line1\r\nline2\r\n";
        let resolution = Resolution::manual(content.to_string());
        assert_eq!(resolution.content, content);
    }

    #[test]
    fn manual_preserves_mixed_line_endings() {
        let content = "line1\nline2\r\nline3\r";
        let resolution = Resolution::manual(content.to_string());
        assert_eq!(resolution.content, content);
    }

    #[test]
    fn manual_preserves_whitespace() {
        let content = "  indented\n\ttabbed\n  \n";
        let resolution = Resolution::manual(content.to_string());
        assert_eq!(resolution.content, content);
    }

    #[test]
    fn manual_metadata_source_is_user() {
        let resolution = Resolution::manual("content".to_string());
        assert_eq!(resolution.metadata.source, ResolutionSource::User);
        assert!(resolution.metadata.notes.is_none());
    }

    #[test]
    fn manual_is_idempotent() {
        let content = "same content".to_string();
        let res1 = Resolution::manual(content.clone());
        let res2 = Resolution::manual(content);
        assert_eq!(res1, res2);
    }
}
