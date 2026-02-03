//! Diff computation for conflict visualization.
//!
//! This module provides line-level and word-level diff computation
//! for highlighting changes between conflict sides in the TUI.

use similar::{ChangeTag, TextDiff};

/// Represents a line with diff information for rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    /// The text content of the line.
    pub text: String,
    /// The diff tag indicating the line's status.
    pub tag: ChangeTag,
}

impl DiffLine {
    /// Creates a new diff line.
    #[must_use]
    pub fn new(text: impl Into<String>, tag: ChangeTag) -> Self {
        Self {
            text: text.into(),
            tag,
        }
    }
}

/// Represents a word-level change within a line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WordChange {
    /// The text of the word or segment.
    pub text: String,
    /// The diff tag for this segment.
    pub tag: ChangeTag,
}

impl WordChange {
    /// Creates a new word change.
    #[must_use]
    pub fn new(text: impl Into<String>, tag: ChangeTag) -> Self {
        Self {
            text: text.into(),
            tag,
        }
    }
}

/// Result of a line-level diff computation containing lines for both sides.
#[derive(Debug, Clone, Default)]
pub struct LineDiffs {
    /// Lines for the left (old) side.
    pub left_lines: Vec<DiffLine>,
    /// Lines for the right (new) side.
    pub right_lines: Vec<DiffLine>,
}

/// Computes line-level diffs between left and right content.
///
/// Returns separate line lists for each side with appropriate diff tags:
/// - Left side: `Delete` for lines only in left, `Equal` for shared lines
/// - Right side: `Insert` for lines only in right, `Equal` for shared lines
#[must_use]
pub fn compute_line_diffs(left: &str, right: &str) -> LineDiffs {
    let diff = TextDiff::from_lines(left, right);
    let mut result = LineDiffs::default();

    for change in diff.iter_all_changes() {
        let text = change.value().trim_end_matches('\n').to_string();

        match change.tag() {
            ChangeTag::Delete => {
                // Line exists only in left (old)
                result
                    .left_lines
                    .push(DiffLine::new(text, ChangeTag::Delete));
            }
            ChangeTag::Insert => {
                // Line exists only in right (new)
                result
                    .right_lines
                    .push(DiffLine::new(text, ChangeTag::Insert));
            }
            ChangeTag::Equal => {
                // Line exists in both
                result
                    .left_lines
                    .push(DiffLine::new(text.clone(), ChangeTag::Equal));
                result
                    .right_lines
                    .push(DiffLine::new(text, ChangeTag::Equal));
            }
        }
    }

    result
}

/// Computes word-level diffs between two lines.
///
/// Useful for highlighting specific changes within modified lines.
#[must_use]
pub fn compute_word_diffs(old_line: &str, new_line: &str) -> Vec<WordChange> {
    let diff = TextDiff::from_words(old_line, new_line);

    diff.iter_all_changes()
        .map(|change| WordChange::new(change.value(), change.tag()))
        .collect()
}

/// Configuration for diff display behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiffConfig {
    /// Enable word-level diff highlighting within changed lines.
    pub word_diff: bool,
}

impl Default for DiffConfig {
    fn default() -> Self {
        Self { word_diff: true }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_identical_content() {
        let content = "line one\nline two\n";
        let diffs = compute_line_diffs(content, content);

        assert_eq!(diffs.left_lines.len(), 2);
        assert_eq!(diffs.right_lines.len(), 2);

        for line in &diffs.left_lines {
            assert_eq!(line.tag, ChangeTag::Equal);
        }
        for line in &diffs.right_lines {
            assert_eq!(line.tag, ChangeTag::Equal);
        }
    }

    #[test]
    fn diff_completely_different() {
        let left = "old line\n";
        let right = "new line\n";
        let diffs = compute_line_diffs(left, right);

        assert_eq!(diffs.left_lines.len(), 1);
        assert_eq!(diffs.right_lines.len(), 1);
        assert_eq!(diffs.left_lines[0].tag, ChangeTag::Delete);
        assert_eq!(diffs.right_lines[0].tag, ChangeTag::Insert);
    }

    #[test]
    fn diff_with_additions() {
        let left = "line one\n";
        let right = "line one\nline two\n";
        let diffs = compute_line_diffs(left, right);

        assert_eq!(diffs.left_lines.len(), 1);
        assert_eq!(diffs.right_lines.len(), 2);

        assert_eq!(diffs.left_lines[0].tag, ChangeTag::Equal);
        assert_eq!(diffs.right_lines[0].tag, ChangeTag::Equal);
        assert_eq!(diffs.right_lines[1].tag, ChangeTag::Insert);
    }

    #[test]
    fn diff_with_deletions() {
        let left = "line one\nline two\n";
        let right = "line one\n";
        let diffs = compute_line_diffs(left, right);

        assert_eq!(diffs.left_lines.len(), 2);
        assert_eq!(diffs.right_lines.len(), 1);

        assert_eq!(diffs.left_lines[0].tag, ChangeTag::Equal);
        assert_eq!(diffs.left_lines[1].tag, ChangeTag::Delete);
        assert_eq!(diffs.right_lines[0].tag, ChangeTag::Equal);
    }

    #[test]
    fn diff_empty_left() {
        let diffs = compute_line_diffs("", "new content\n");

        assert!(diffs.left_lines.is_empty());
        assert_eq!(diffs.right_lines.len(), 1);
        assert_eq!(diffs.right_lines[0].tag, ChangeTag::Insert);
    }

    #[test]
    fn diff_empty_right() {
        let diffs = compute_line_diffs("old content\n", "");

        assert_eq!(diffs.left_lines.len(), 1);
        assert!(diffs.right_lines.is_empty());
        assert_eq!(diffs.left_lines[0].tag, ChangeTag::Delete);
    }

    #[test]
    fn diff_both_empty() {
        let diffs = compute_line_diffs("", "");

        assert!(diffs.left_lines.is_empty());
        assert!(diffs.right_lines.is_empty());
    }

    #[test]
    fn word_diff_single_change() {
        let changes = compute_word_diffs("hello world", "hello universe");

        // Should have: "hello " (equal), "world" (delete) / "universe" (insert)
        let has_equal = changes.iter().any(|c| c.tag == ChangeTag::Equal);
        let has_change = changes.iter().any(|c| c.tag != ChangeTag::Equal);

        assert!(has_equal);
        assert!(has_change);
    }

    #[test]
    fn word_diff_identical() {
        let changes = compute_word_diffs("hello world", "hello world");

        for change in &changes {
            assert_eq!(change.tag, ChangeTag::Equal);
        }
    }

    #[test]
    fn diff_config_default() {
        let config = DiffConfig::default();
        assert!(config.word_diff);
    }
}
