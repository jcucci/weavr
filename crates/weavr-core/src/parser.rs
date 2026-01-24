//! Conflict marker parsing.
//!
//! All types in this module are **stable** and covered by semantic versioning.

use serde::{Deserialize, Serialize};

use crate::{ConflictHunk, HunkContent, HunkContext, HunkId, HunkState, ParseError};

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
pub struct ParsedConflict {
    /// All conflict hunks in file order.
    pub hunks: Vec<ConflictHunk>,
    /// File structure with clean text and conflict references.
    pub segments: Vec<Segment>,
}

/// Internal parser state machine states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParserState {
    /// Outside any conflict.
    Clean,
    /// After <<<<<<< before ||||||| or =======
    InLeft,
    /// After ||||||| before ======= (diff3 format).
    InBase,
    /// After ======= before >>>>>>>
    InRight,
}

/// Detected conflict marker type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Marker {
    /// <<<<<<< - Start of conflict.
    Start,
    /// ||||||| - Base content (diff3).
    Base,
    /// ======= - Separator between sides.
    Separator,
    /// >>>>>>> - End of conflict.
    End,
}

/// Detects if a line is a conflict marker.
///
/// Markers must be at the start of the line:
/// - `<<<<<<<` - 7 less-than signs, optionally followed by space and label
/// - `|||||||` - 7 pipe signs, optionally followed by space and label
/// - `=======` - Exactly 7 equals signs (nothing after except whitespace)
/// - `>>>>>>>` - 7 greater-than signs, optionally followed by space and label
fn detect_marker(line: &str) -> Option<Marker> {
    if line.starts_with("<<<<<<<") {
        Some(Marker::Start)
    } else if line.starts_with("|||||||") {
        Some(Marker::Base)
    } else if line == "======="
        || line.starts_with("=======") && line[7..].chars().all(char::is_whitespace)
    {
        Some(Marker::Separator)
    } else if line.starts_with(">>>>>>>") {
        Some(Marker::End)
    } else {
        None
    }
}

/// Parses conflict markers from file content.
///
/// Supports both standard 2-way conflicts and diff3 3-way conflicts.
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
#[allow(clippy::too_many_lines)]
pub fn parse_conflict_markers(content: &str) -> Result<ParsedConflict, ParseError> {
    let lines: Vec<&str> = content.lines().collect();
    let mut state = ParserState::Clean;
    let mut segments: Vec<Segment> = Vec::new();
    let mut hunks: Vec<ConflictHunk> = Vec::new();

    let mut clean_buffer: Vec<String> = Vec::new();
    let mut left_buffer: Vec<String> = Vec::new();
    let mut base_buffer: Option<Vec<String>> = None;
    let mut right_buffer: Vec<String> = Vec::new();

    let mut hunk_start_line: usize = 0;
    let mut left_content_start: usize = 0;
    let mut right_content_start: usize = 0;
    let mut hunk_id_counter: u32 = 0;

    for (line_num, line) in lines.iter().enumerate() {
        let one_indexed = line_num + 1;

        match (detect_marker(line), state) {
            // Start marker in clean state - begin new conflict
            (Some(Marker::Start), ParserState::Clean) => {
                // Flush clean buffer to segments
                if !clean_buffer.is_empty() {
                    segments.push(Segment::Clean(clean_buffer.join("\n")));
                    clean_buffer.clear();
                }
                hunk_start_line = one_indexed;
                left_content_start = one_indexed + 1;
                state = ParserState::InLeft;
            }

            // Start marker while already in conflict - nested conflict error
            (Some(Marker::Start), _) => {
                return Err(ParseError::InvalidMarkers(format!(
                    "nested conflict marker at line {one_indexed}"
                )));
            }

            // Base marker after left - enter diff3 base section
            (Some(Marker::Base), ParserState::InLeft) => {
                base_buffer = Some(Vec::new());
                state = ParserState::InBase;
            }

            // Base marker in wrong state
            (Some(Marker::Base), ParserState::InBase) => {
                return Err(ParseError::InvalidMarkers(format!(
                    "duplicate base marker at line {one_indexed}"
                )));
            }

            (Some(Marker::Base), _) => {
                return Err(ParseError::InvalidMarkers(format!(
                    "unexpected base marker at line {one_indexed}"
                )));
            }

            // Separator after left or base - enter right section
            (Some(Marker::Separator), ParserState::InLeft | ParserState::InBase) => {
                right_content_start = one_indexed + 1;
                state = ParserState::InRight;
            }

            // Separator in wrong state
            (Some(Marker::Separator), ParserState::InRight) => {
                return Err(ParseError::InvalidMarkers(format!(
                    "duplicate separator at line {one_indexed}"
                )));
            }

            (Some(Marker::Separator), ParserState::Clean) => {
                return Err(ParseError::InvalidMarkers(format!(
                    "unexpected separator at line {one_indexed}"
                )));
            }

            // End marker after right - complete the hunk
            (Some(Marker::End), ParserState::InRight) => {
                // Extract context lines
                let context_start = if hunk_start_line > DEFAULT_CONTEXT_LINES {
                    hunk_start_line - DEFAULT_CONTEXT_LINES - 1
                } else {
                    0
                };
                let before: Vec<String> = lines[context_start..hunk_start_line - 1]
                    .iter()
                    .map(|s| (*s).to_string())
                    .collect();

                // Build the hunk
                let hunk = ConflictHunk {
                    id: HunkId(hunk_id_counter),
                    left: HunkContent {
                        text: left_buffer.join("\n"),
                    },
                    right: HunkContent {
                        text: right_buffer.join("\n"),
                    },
                    base: base_buffer
                        .take()
                        .map(|b| HunkContent { text: b.join("\n") }),
                    context: HunkContext {
                        before,
                        after: Vec::new(), // Will be filled after parsing completes
                        start_line_left: left_content_start,
                        start_line_right: right_content_start,
                    },
                    state: HunkState::Unresolved,
                };

                let hunk_index = hunks.len();
                hunks.push(hunk);
                segments.push(Segment::Conflict(hunk_index));

                hunk_id_counter += 1;
                left_buffer.clear();
                right_buffer.clear();
                state = ParserState::Clean;
            }

            // End marker in wrong state
            (Some(Marker::End), _) => {
                return Err(ParseError::InvalidMarkers(format!(
                    "unexpected end marker at line {one_indexed}"
                )));
            }

            // Regular line - add to appropriate buffer
            (None, ParserState::Clean) => {
                clean_buffer.push((*line).to_string());
            }

            (None, ParserState::InLeft) => {
                left_buffer.push((*line).to_string());
            }

            (None, ParserState::InBase) => {
                if let Some(ref mut buf) = base_buffer {
                    buf.push((*line).to_string());
                }
            }

            (None, ParserState::InRight) => {
                right_buffer.push((*line).to_string());
            }
        }
    }

    // Check for unclosed conflict at EOF
    if state != ParserState::Clean {
        return Err(ParseError::InvalidMarkers(format!(
            "unclosed conflict starting at line {hunk_start_line}"
        )));
    }

    // Flush remaining clean content
    if !clean_buffer.is_empty() {
        segments.push(Segment::Clean(clean_buffer.join("\n")));
    }

    // Fill in 'after' context for all hunks
    fill_after_context(&mut hunks, &lines);

    Ok(ParsedConflict { hunks, segments })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_two_way_conflict() {
        let content = r"before
<<<<<<< HEAD
left content
=======
right content
>>>>>>> feature
after";

        let result = parse_conflict_markers(content).unwrap();
        assert_eq!(result.hunks.len(), 1);
        assert_eq!(result.hunks[0].left.text, "left content");
        assert_eq!(result.hunks[0].right.text, "right content");
        assert!(result.hunks[0].base.is_none());
    }

    #[test]
    fn parse_diff3_three_way_conflict() {
        let content = r"before
<<<<<<< HEAD
left content
||||||| merged common ancestors
base content
=======
right content
>>>>>>> feature
after";

        let result = parse_conflict_markers(content).unwrap();
        assert_eq!(result.hunks.len(), 1);
        assert_eq!(result.hunks[0].left.text, "left content");
        assert_eq!(result.hunks[0].right.text, "right content");
        assert!(result.hunks[0].base.is_some());
        assert_eq!(result.hunks[0].base.as_ref().unwrap().text, "base content");
    }

    #[test]
    fn parse_multiple_hunks() {
        let content = r"// header
<<<<<<< HEAD
first left
=======
first right
>>>>>>> feature
middle content
<<<<<<< HEAD
second left
=======
second right
>>>>>>> feature
// footer";

        let result = parse_conflict_markers(content).unwrap();
        assert_eq!(result.hunks.len(), 2);
        assert_eq!(result.hunks[0].left.text, "first left");
        assert_eq!(result.hunks[0].right.text, "first right");
        assert_eq!(result.hunks[1].left.text, "second left");
        assert_eq!(result.hunks[1].right.text, "second right");
    }

    #[test]
    fn parse_no_conflicts_returns_empty_hunks() {
        let content = "just normal content\nno conflicts here";

        let result = parse_conflict_markers(content).unwrap();
        assert!(result.hunks.is_empty());
        assert_eq!(result.segments.len(), 1);
        if let Segment::Clean(text) = &result.segments[0] {
            assert_eq!(text, "just normal content\nno conflicts here");
        } else {
            panic!("Expected Clean segment");
        }
    }

    #[test]
    fn preserves_exact_line_content_no_trimming() {
        let content =
            "<<<<<<< HEAD\n  indented with spaces  \n=======\n\ttabbed content\t\n>>>>>>> feature";

        let result = parse_conflict_markers(content).unwrap();
        assert_eq!(result.hunks[0].left.text, "  indented with spaces  ");
        assert_eq!(result.hunks[0].right.text, "\ttabbed content\t");
    }

    #[test]
    fn preserves_empty_lines_in_content() {
        let content = r"<<<<<<< HEAD
line one

line three
=======
right
>>>>>>> feature";

        let result = parse_conflict_markers(content).unwrap();
        assert_eq!(result.hunks[0].left.text, "line one\n\nline three");
    }

    #[test]
    fn conflict_at_file_start() {
        let content = r"<<<<<<< HEAD
left
=======
right
>>>>>>> feature
after";

        let result = parse_conflict_markers(content).unwrap();
        assert_eq!(result.hunks.len(), 1);
        assert!(result.hunks[0].context.before.is_empty());
    }

    #[test]
    fn conflict_at_file_end() {
        let content = r"before
<<<<<<< HEAD
left
=======
right
>>>>>>> feature";

        let result = parse_conflict_markers(content).unwrap();
        assert_eq!(result.hunks.len(), 1);
        assert!(result.hunks[0].context.after.is_empty());
    }

    #[test]
    fn empty_left_side() {
        let content = r"<<<<<<< HEAD
=======
right content
>>>>>>> feature";

        let result = parse_conflict_markers(content).unwrap();
        assert_eq!(result.hunks[0].left.text, "");
        assert_eq!(result.hunks[0].right.text, "right content");
    }

    #[test]
    fn empty_right_side() {
        let content = r"<<<<<<< HEAD
left content
=======
>>>>>>> feature";

        let result = parse_conflict_markers(content).unwrap();
        assert_eq!(result.hunks[0].left.text, "left content");
        assert_eq!(result.hunks[0].right.text, "");
    }

    #[test]
    fn empty_both_sides() {
        let content = r"<<<<<<< HEAD
=======
>>>>>>> feature";

        let result = parse_conflict_markers(content).unwrap();
        assert_eq!(result.hunks[0].left.text, "");
        assert_eq!(result.hunks[0].right.text, "");
    }

    #[test]
    fn context_lines_captured_correctly() {
        let content = r"line 1
line 2
line 3
line 4
<<<<<<< HEAD
left
=======
right
>>>>>>> feature
line 5
line 6
line 7
line 8";

        let result = parse_conflict_markers(content).unwrap();
        // Should capture 3 lines before (line 2, 3, 4)
        assert_eq!(result.hunks[0].context.before.len(), 3);
        assert_eq!(result.hunks[0].context.before[0], "line 2");
        assert_eq!(result.hunks[0].context.before[1], "line 3");
        assert_eq!(result.hunks[0].context.before[2], "line 4");
        // Should capture 3 lines after (line 5, 6, 7)
        assert_eq!(result.hunks[0].context.after.len(), 3);
        assert_eq!(result.hunks[0].context.after[0], "line 5");
        assert_eq!(result.hunks[0].context.after[1], "line 6");
        assert_eq!(result.hunks[0].context.after[2], "line 7");
    }

    #[test]
    fn line_numbers_are_one_indexed() {
        let content = r"line 1
<<<<<<< HEAD
left content
=======
right content
>>>>>>> feature";

        let result = parse_conflict_markers(content).unwrap();
        // <<<<<<< is on line 2, so left content starts on line 3
        assert_eq!(result.hunks[0].context.start_line_left, 3);
        // ======= is on line 4, so right content starts on line 5
        assert_eq!(result.hunks[0].context.start_line_right, 5);
    }

    #[test]
    fn error_on_nested_start_marker() {
        let content = r"<<<<<<< HEAD
left
<<<<<<< nested
nested left
=======
right
>>>>>>> feature";

        let result = parse_conflict_markers(content);
        assert!(matches!(result, Err(ParseError::InvalidMarkers(msg)) if msg.contains("nested")));
    }

    #[test]
    fn error_on_orphan_separator() {
        let content = "some content\n=======\nmore content";

        let result = parse_conflict_markers(content);
        assert!(
            matches!(result, Err(ParseError::InvalidMarkers(msg)) if msg.contains("unexpected separator"))
        );
    }

    #[test]
    fn error_on_orphan_end_marker() {
        let content = "some content\n>>>>>>> feature\nmore content";

        let result = parse_conflict_markers(content);
        assert!(
            matches!(result, Err(ParseError::InvalidMarkers(msg)) if msg.contains("unexpected end marker"))
        );
    }

    #[test]
    fn error_on_unclosed_conflict() {
        let content = r"<<<<<<< HEAD
left content
=======
right content";

        let result = parse_conflict_markers(content);
        assert!(
            matches!(result, Err(ParseError::InvalidMarkers(msg)) if msg.contains("unclosed conflict"))
        );
    }

    #[test]
    fn error_on_duplicate_base_marker() {
        let content = r"<<<<<<< HEAD
left
||||||| base
first base
||||||| second base
second
=======
right
>>>>>>> feature";

        let result = parse_conflict_markers(content);
        assert!(
            matches!(result, Err(ParseError::InvalidMarkers(msg)) if msg.contains("duplicate base"))
        );
    }

    #[test]
    fn error_on_duplicate_separator() {
        let content = r"<<<<<<< HEAD
left
=======
middle
=======
right
>>>>>>> feature";

        let result = parse_conflict_markers(content);
        assert!(
            matches!(result, Err(ParseError::InvalidMarkers(msg)) if msg.contains("duplicate separator"))
        );
    }

    #[test]
    fn marker_with_label_parsed_correctly() {
        let content = r"<<<<<<< HEAD (some label here)
left
=======
right
>>>>>>> feature-branch-name";

        let result = parse_conflict_markers(content).unwrap();
        assert_eq!(result.hunks.len(), 1);
        assert_eq!(result.hunks[0].left.text, "left");
    }

    #[test]
    fn six_equals_is_not_separator() {
        let content = "======\nnot a separator";

        let result = parse_conflict_markers(content).unwrap();
        assert!(result.hunks.is_empty());
    }

    #[test]
    fn hunk_ids_are_sequential() {
        let content = r"<<<<<<< HEAD
a
=======
b
>>>>>>> feature
<<<<<<< HEAD
c
=======
d
>>>>>>> feature
<<<<<<< HEAD
e
=======
f
>>>>>>> feature";

        let result = parse_conflict_markers(content).unwrap();
        assert_eq!(result.hunks[0].id, HunkId(0));
        assert_eq!(result.hunks[1].id, HunkId(1));
        assert_eq!(result.hunks[2].id, HunkId(2));
    }

    #[test]
    fn segments_preserve_file_structure() {
        let content = r"before
<<<<<<< HEAD
left
=======
right
>>>>>>> feature
middle
<<<<<<< HEAD
left2
=======
right2
>>>>>>> feature
after";

        let result = parse_conflict_markers(content).unwrap();
        assert_eq!(result.segments.len(), 5);
        assert!(matches!(&result.segments[0], Segment::Clean(s) if s == "before"));
        assert!(matches!(&result.segments[1], Segment::Conflict(0)));
        assert!(matches!(&result.segments[2], Segment::Clean(s) if s == "middle"));
        assert!(matches!(&result.segments[3], Segment::Conflict(1)));
        assert!(matches!(&result.segments[4], Segment::Clean(s) if s == "after"));
    }

    #[test]
    fn all_hunks_start_unresolved() {
        let content = r"<<<<<<< HEAD
left
=======
right
>>>>>>> feature";

        let result = parse_conflict_markers(content).unwrap();
        assert_eq!(result.hunks[0].state, HunkState::Unresolved);
    }
}
