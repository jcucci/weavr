//! Conflict marker parsing.
//!
//! All types in this module are **stable** and covered by semantic versioning.

mod git;
mod jj_diff;
mod jj_snapshot;

use serde::{Deserialize, Serialize};

use crate::{ConflictFormat, ConflictHunk, ParseError};

/// Default number of context lines before and after a conflict.
const DEFAULT_CONTEXT_LINES: usize = 3;

/// A segment of a file - either clean text or a conflict.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Segment {
    /// Non-conflicting text (preserved exactly).
    Clean(String),
    /// A conflict hunk (index into `ParsedConflict::hunks`).
    Conflict(usize),
}

/// Result of parsing a conflicted file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ParsedConflict {
    /// All conflict hunks in file order.
    pub hunks: Vec<ConflictHunk>,
    /// File structure with clean text and conflict references.
    pub segments: Vec<Segment>,
    /// The conflict marker format detected in the file.
    #[serde(default)]
    pub format: Option<ConflictFormat>,
}

/// Parses conflict markers from file content.
///
/// Auto-detects the conflict format (Git, jj snapshot, or jj diff) and
/// dispatches to the appropriate parser. Supports standard 2-way, diff3
/// 3-way (Git), jj snapshot format, and jj diff format conflicts.
///
/// # Arguments
///
/// * `content` - The file content containing conflict markers.
///
/// # Errors
///
/// Returns `ParseError::InvalidMarkers` for:
/// - Nested conflict markers
/// - Mismatched or orphaned markers
/// - Unclosed conflicts at EOF
///
/// Returns `ParseError::MalformedContent` for:
/// - Multi-sided jj conflicts (3+ sides)
/// - Mismatched bases in pure diff format
///
/// # Examples
///
/// ```
/// use weavr_core::parse_conflict_markers;
///
/// let content = r#"before
/// <<<<<<< HEAD
/// left content
/// =======
/// right content
/// >>>>>>> branch
/// after"#;
///
/// let parsed = parse_conflict_markers(content).unwrap();
/// assert_eq!(parsed.hunks.len(), 1);
/// ```
pub fn parse_conflict_markers(content: &str) -> Result<ParsedConflict, ParseError> {
    match crate::detect_format(content) {
        Some(ConflictFormat::JjSnapshot) => jj_snapshot::parse_jj_snapshot_markers(content),
        Some(ConflictFormat::JjDiff) => jj_diff::parse_jj_diff_markers(content),
        Some(ConflictFormat::Git) | None => git::parse_git_markers(content),
    }
}

/// Fills in the 'after' context for all hunks by scanning forward from each hunk's position.
fn fill_after_context(hunks: &mut [ConflictHunk], lines: &[&str]) {
    let hunk_count = hunks.len();

    // First pass: collect the start lines of each hunk for boundary checking
    let hunk_starts: Vec<usize> = hunks
        .iter()
        .map(|h| h.context.start_line_left.saturating_sub(1)) // <<<<<<< marker line
        .collect();

    // Second pass: fill in after context
    for hunk_index in 0..hunk_count {
        let hunk = &hunks[hunk_index];

        // Find where this hunk ends by looking at its right content start line
        // and counting forward through the right content
        let right_start = hunk.context.start_line_right;
        let right_line_count = if hunk.right.text.is_empty() {
            0
        } else {
            hunk.right.text.lines().count()
        };
        let end_marker_line = right_start + right_line_count; // Line with >>>>>>>

        // Collect up to DEFAULT_CONTEXT_LINES after the end marker
        let after_start = end_marker_line;
        let after_end = (after_start + DEFAULT_CONTEXT_LINES).min(lines.len());

        // Check if next segment is another conflict
        let next_conflict_start = if hunk_index + 1 < hunk_count {
            hunk_starts[hunk_index + 1]
        } else {
            usize::MAX
        };

        let actual_end = after_end.min(next_conflict_start);

        if after_start < actual_end && after_start < lines.len() {
            hunks[hunk_index].context.after = lines[after_start..actual_end]
                .iter()
                .map(|s| (*s).to_string())
                .collect();
        }
    }
}
