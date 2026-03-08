//! Conflict marker parsing.
//!
//! All types in this module are **stable** and covered by semantic versioning.

use serde::{Deserialize, Serialize};

use crate::{
    ConflictFormat, ConflictHunk, HunkContent, HunkContext, HunkId, HunkState, ParseError,
};

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
        Some(ConflictFormat::JjSnapshot) => parse_jj_snapshot_markers(content),
        Some(ConflictFormat::JjDiff) => parse_jj_diff_markers(content),
        Some(ConflictFormat::Git) | None => parse_git_markers(content),
    }
}

/// Parses Git-style conflict markers.
#[allow(clippy::too_many_lines)]
fn parse_git_markers(content: &str) -> Result<ParsedConflict, ParseError> {
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

    let format = if hunks.is_empty() {
        None
    } else {
        Some(ConflictFormat::Git)
    };

    Ok(ParsedConflict {
        hunks,
        segments,
        format,
    })
}

// --- jj Snapshot Parser ---

/// Detected jj snapshot marker type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JjSnapshotMarker {
    /// <<<<<<< - Start of conflict.
    Start,
    /// +++++++ - Side content.
    Side,
    /// ------- - Base content.
    Base,
    /// >>>>>>> - End of conflict.
    End,
}

/// State machine for jj snapshot parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JjSnapshotState {
    /// Outside any conflict.
    Clean,
    /// After <<<<<<< before first +++++++
    AwaitingFirstSide,
    /// After first +++++++ (collecting side 1 content).
    InSide1,
    /// After ------- (collecting base content).
    InBase,
    /// After second +++++++ (collecting side 2 content).
    InSide2,
}

/// Detects if a line is a jj snapshot marker.
///
/// Markers must have 7+ consecutive characters of the same type,
/// followed by either nothing or a space and label.
fn detect_jj_marker(line: &str) -> Option<JjSnapshotMarker> {
    let bytes = line.as_bytes();
    if bytes.len() < 7 {
        return None;
    }

    let marker_char = bytes[0];
    let marker_type = match marker_char {
        b'<' => JjSnapshotMarker::Start,
        b'+' => JjSnapshotMarker::Side,
        b'-' => JjSnapshotMarker::Base,
        b'>' => JjSnapshotMarker::End,
        _ => return None,
    };

    // Count consecutive marker characters
    let count = bytes.iter().take_while(|&&b| b == marker_char).count();
    if count < 7 {
        return None;
    }

    // Remainder must be empty or start with a space
    if count < bytes.len() && bytes[count] != b' ' {
        return None;
    }

    Some(marker_type)
}

/// Parses jj snapshot-style conflict markers.
#[allow(clippy::too_many_lines)]
fn parse_jj_snapshot_markers(content: &str) -> Result<ParsedConflict, ParseError> {
    let lines: Vec<&str> = content.lines().collect();
    let mut state = JjSnapshotState::Clean;
    let mut segments: Vec<Segment> = Vec::new();
    let mut hunks: Vec<ConflictHunk> = Vec::new();

    let mut clean_buffer: Vec<String> = Vec::new();
    let mut side1_buffer: Vec<String> = Vec::new();
    let mut base_buffer: Option<Vec<String>> = None;
    let mut side2_buffer: Vec<String> = Vec::new();

    let mut hunk_start_line: usize = 0;
    let mut side1_content_start: usize = 0;
    let mut side2_content_start: usize = 0;
    let mut hunk_id_counter: u32 = 0;
    let mut side_count: u32 = 0;

    for (line_num, line) in lines.iter().enumerate() {
        let one_indexed = line_num + 1;

        match (detect_jj_marker(line), state) {
            // Start marker in clean state
            (Some(JjSnapshotMarker::Start), JjSnapshotState::Clean) => {
                if !clean_buffer.is_empty() {
                    segments.push(Segment::Clean(clean_buffer.join("\n")));
                    clean_buffer.clear();
                }
                hunk_start_line = one_indexed;
                side_count = 0;
                state = JjSnapshotState::AwaitingFirstSide;
            }

            // Nested start marker
            (Some(JjSnapshotMarker::Start), _) => {
                return Err(ParseError::InvalidMarkers(format!(
                    "nested conflict marker at line {one_indexed}"
                )));
            }

            // First side marker after start
            (Some(JjSnapshotMarker::Side), JjSnapshotState::AwaitingFirstSide) => {
                side1_content_start = one_indexed + 1;
                side_count = 1;
                state = JjSnapshotState::InSide1;
            }

            // Second side marker (from side 1 or base)
            (Some(JjSnapshotMarker::Side), JjSnapshotState::InSide1 | JjSnapshotState::InBase) => {
                side_count += 1;
                if side_count > 2 {
                    return Err(ParseError::MalformedContent(
                        "multi-sided jj conflicts not yet supported".to_string(),
                    ));
                }
                side2_content_start = one_indexed + 1;
                state = JjSnapshotState::InSide2;
            }

            // Third+ side marker
            (Some(JjSnapshotMarker::Side), JjSnapshotState::InSide2) => {
                return Err(ParseError::MalformedContent(
                    "multi-sided jj conflicts not yet supported".to_string(),
                ));
            }

            // Base marker after side 1
            (Some(JjSnapshotMarker::Base), JjSnapshotState::InSide1) => {
                base_buffer = Some(Vec::new());
                state = JjSnapshotState::InBase;
            }

            // Base marker in wrong state
            (Some(JjSnapshotMarker::Base), _) => {
                return Err(ParseError::InvalidMarkers(format!(
                    "unexpected base marker at line {one_indexed}"
                )));
            }

            // Side marker in wrong state
            (Some(JjSnapshotMarker::Side), JjSnapshotState::Clean) => {
                return Err(ParseError::InvalidMarkers(format!(
                    "unexpected side marker at line {one_indexed}"
                )));
            }

            // End marker after side 2 - complete the hunk
            (Some(JjSnapshotMarker::End), JjSnapshotState::InSide2) => {
                let context_start = if hunk_start_line > DEFAULT_CONTEXT_LINES {
                    hunk_start_line - DEFAULT_CONTEXT_LINES - 1
                } else {
                    0
                };
                let before: Vec<String> = lines[context_start..hunk_start_line - 1]
                    .iter()
                    .map(|s| (*s).to_string())
                    .collect();

                let hunk = ConflictHunk {
                    id: HunkId(hunk_id_counter),
                    left: HunkContent {
                        text: side1_buffer.join("\n"),
                    },
                    right: HunkContent {
                        text: side2_buffer.join("\n"),
                    },
                    base: base_buffer
                        .take()
                        .map(|b| HunkContent { text: b.join("\n") }),
                    context: HunkContext {
                        before,
                        after: Vec::new(),
                        start_line_left: side1_content_start,
                        start_line_right: side2_content_start,
                    },
                    state: HunkState::Unresolved,
                };

                let hunk_index = hunks.len();
                hunks.push(hunk);
                segments.push(Segment::Conflict(hunk_index));

                hunk_id_counter += 1;
                side1_buffer.clear();
                side2_buffer.clear();
                side_count = 0;
                state = JjSnapshotState::Clean;
            }

            // End marker in wrong state
            (Some(JjSnapshotMarker::End), _) => {
                return Err(ParseError::InvalidMarkers(format!(
                    "unexpected end marker at line {one_indexed}"
                )));
            }

            // Regular content lines
            (None, JjSnapshotState::Clean) => {
                clean_buffer.push((*line).to_string());
            }

            (None, JjSnapshotState::AwaitingFirstSide) => {
                // Content between <<<<<<< and first +++++++ is unexpected
                return Err(ParseError::InvalidMarkers(format!(
                    "unexpected content before first side marker at line {one_indexed}"
                )));
            }

            (None, JjSnapshotState::InSide1) => {
                side1_buffer.push((*line).to_string());
            }

            (None, JjSnapshotState::InBase) => {
                if let Some(ref mut buf) = base_buffer {
                    buf.push((*line).to_string());
                }
            }

            (None, JjSnapshotState::InSide2) => {
                side2_buffer.push((*line).to_string());
            }
        }
    }

    // Check for unclosed conflict at EOF
    if state != JjSnapshotState::Clean {
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

    let format = if hunks.is_empty() {
        None
    } else {
        Some(ConflictFormat::JjSnapshot)
    };

    Ok(ParsedConflict {
        hunks,
        segments,
        format,
    })
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

// --- jj Diff Parser ---

/// Detected jj diff marker type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JjDiffMarker {
    /// <<<<<<< - Start of conflict.
    Start,
    /// +++++++ - Snapshot side content.
    SnapshotSide,
    /// %%%%%%% - Diff section.
    DiffSide,
    /// >>>>>>> - End of conflict.
    End,
}

/// State machine for jj diff parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JjDiffState {
    /// Outside any conflict.
    Clean,
    /// After <<<<<<< before first section marker.
    AwaitingFirstSection,
    /// Inside a +++++++ snapshot side.
    InSnapshotSide,
    /// Inside a %%%%%%% diff section. `u8` is the section number (1 or 2).
    InDiffSection(u8),
}

/// Detects if a line is a jj diff marker.
///
/// Markers must have 7+ consecutive characters of the same type,
/// followed by either nothing or a space and label.
fn detect_jj_diff_marker(line: &str) -> Option<JjDiffMarker> {
    let bytes = line.as_bytes();
    if bytes.len() < 7 {
        return None;
    }

    let marker_char = bytes[0];
    let marker_type = match marker_char {
        b'<' => JjDiffMarker::Start,
        b'+' => JjDiffMarker::SnapshotSide,
        b'%' => JjDiffMarker::DiffSide,
        b'>' => JjDiffMarker::End,
        _ => return None,
    };

    let count = bytes.iter().take_while(|&&b| b == marker_char).count();
    if count < 7 {
        return None;
    }

    if count < bytes.len() && bytes[count] != b' ' {
        return None;
    }

    Some(marker_type)
}

/// Reconstructs base and side content from a jj diff section.
///
/// In `%%%%%%%` sections:
/// - ` ` prefix = context (present in both base and side)
/// - `-` prefix = base only
/// - `+` prefix = side only
/// - Empty lines = context (present in both)
fn reconstruct_from_diff(diff_lines: &[String]) -> Result<(String, String), ParseError> {
    let mut base_lines: Vec<&str> = Vec::new();
    let mut side_lines: Vec<&str> = Vec::new();

    for line in diff_lines {
        if line.is_empty() {
            // Empty line = context (in both base and side)
            base_lines.push("");
            side_lines.push("");
        } else {
            match line.as_bytes()[0] {
                b' ' => {
                    let content = &line[1..];
                    base_lines.push(content);
                    side_lines.push(content);
                }
                b'-' => {
                    base_lines.push(&line[1..]);
                }
                b'+' => {
                    side_lines.push(&line[1..]);
                }
                _ => {
                    return Err(ParseError::MalformedContent(format!(
                        "unexpected prefix in diff line: {line}"
                    )));
                }
            }
        }
    }

    Ok((base_lines.join("\n"), side_lines.join("\n")))
}

/// Fills in 'after' context for all hunks using explicit end-marker line positions.
///
/// Unlike `fill_after_context`, this variant takes explicit `>>>>>>>` line positions
/// rather than computing them from `start_line_right + right_line_count`, which
/// doesn't work for diff format since diff lines != reconstructed content lines.
fn fill_after_context_with_ends(hunks: &mut [ConflictHunk], lines: &[&str], end_lines: &[usize]) {
    let hunk_count = hunks.len();

    let hunk_starts: Vec<usize> = hunks
        .iter()
        .map(|h| h.context.start_line_left.saturating_sub(1))
        .collect();

    for hunk_index in 0..hunk_count {
        let end_marker_line = end_lines[hunk_index]; // 1-indexed line of >>>>>>>

        // Collect up to DEFAULT_CONTEXT_LINES after the end marker
        let after_start = end_marker_line; // 0-indexed start = end_marker_line (1-indexed is line after)
        let after_end = (after_start + DEFAULT_CONTEXT_LINES).min(lines.len());

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

/// Parses jj diff-style conflict markers.
///
/// Supports both mixed format (snapshot + diff) and pure diff format (two diffs).
#[allow(clippy::too_many_lines)]
fn parse_jj_diff_markers(content: &str) -> Result<ParsedConflict, ParseError> {
    let lines: Vec<&str> = content.lines().collect();
    let mut state = JjDiffState::Clean;
    let mut segments: Vec<Segment> = Vec::new();
    let mut hunks: Vec<ConflictHunk> = Vec::new();

    let mut clean_buffer: Vec<String> = Vec::new();
    let mut snapshot_buffer: Vec<String> = Vec::new();
    let mut diff1_buffer: Vec<String> = Vec::new();
    let mut diff2_buffer: Vec<String> = Vec::new();

    let mut hunk_start_line: usize = 0;
    let mut section1_content_start: usize = 0;
    let mut section2_content_start: usize = 0;
    let mut hunk_id_counter: u32 = 0;
    let mut section_count: u8 = 0;
    let mut has_snapshot: bool = false;
    let mut end_lines: Vec<usize> = Vec::new();

    for (line_num, line) in lines.iter().enumerate() {
        let one_indexed = line_num + 1;

        match (detect_jj_diff_marker(line), state) {
            // Start marker in clean state
            (Some(JjDiffMarker::Start), JjDiffState::Clean) => {
                if !clean_buffer.is_empty() {
                    segments.push(Segment::Clean(clean_buffer.join("\n")));
                    clean_buffer.clear();
                }
                hunk_start_line = one_indexed;
                section_count = 0;
                has_snapshot = false;
                state = JjDiffState::AwaitingFirstSection;
            }

            // Nested start marker
            (Some(JjDiffMarker::Start), _) => {
                return Err(ParseError::InvalidMarkers(format!(
                    "nested conflict marker at line {one_indexed}"
                )));
            }

            // Snapshot side (+++++++): only valid as first section
            (Some(JjDiffMarker::SnapshotSide), JjDiffState::AwaitingFirstSection) => {
                section1_content_start = one_indexed + 1;
                section_count = 1;
                has_snapshot = true;
                state = JjDiffState::InSnapshotSide;
            }

            // Snapshot side in wrong state
            (
                Some(JjDiffMarker::SnapshotSide),
                JjDiffState::InSnapshotSide | JjDiffState::InDiffSection(_),
            ) => {
                return Err(ParseError::MalformedContent(
                    "unexpected snapshot marker inside diff conflict".to_string(),
                ));
            }

            // Diff section (%%%%%%%): transitions from various states
            (Some(JjDiffMarker::DiffSide), JjDiffState::AwaitingFirstSection) => {
                section1_content_start = one_indexed + 1;
                section_count = 1;
                state = JjDiffState::InDiffSection(1);
            }

            (
                Some(JjDiffMarker::DiffSide),
                JjDiffState::InSnapshotSide | JjDiffState::InDiffSection(1),
            ) => {
                section_count += 1;
                if section_count > 2 {
                    return Err(ParseError::MalformedContent(
                        "multi-sided jj conflicts not yet supported".to_string(),
                    ));
                }
                section2_content_start = one_indexed + 1;
                state = JjDiffState::InDiffSection(2);
            }

            (Some(JjDiffMarker::DiffSide), JjDiffState::InDiffSection(2)) => {
                return Err(ParseError::MalformedContent(
                    "multi-sided jj conflicts not yet supported".to_string(),
                ));
            }

            // Snapshot/Diff side in clean state
            (Some(JjDiffMarker::SnapshotSide | JjDiffMarker::DiffSide), JjDiffState::Clean) => {
                return Err(ParseError::InvalidMarkers(format!(
                    "unexpected section marker at line {one_indexed}"
                )));
            }

            // End marker after second section - complete the hunk
            (Some(JjDiffMarker::End), JjDiffState::InDiffSection(2)) => {
                let context_start = if hunk_start_line > DEFAULT_CONTEXT_LINES {
                    hunk_start_line - DEFAULT_CONTEXT_LINES - 1
                } else {
                    0
                };
                let before: Vec<String> = lines[context_start..hunk_start_line - 1]
                    .iter()
                    .map(|s| (*s).to_string())
                    .collect();

                let (left, right, base) = if has_snapshot {
                    // Mixed format: snapshot side 1, diff side 2
                    let left_text = snapshot_buffer.join("\n");
                    let (base_text, right_text) = reconstruct_from_diff(&diff2_buffer)?;
                    (left_text, right_text, Some(base_text))
                } else {
                    // Pure diff: both sections are diffs from the same base
                    let (base1, left_text) = reconstruct_from_diff(&diff1_buffer)?;
                    let (base2, right_text) = reconstruct_from_diff(&diff2_buffer)?;
                    if base1 != base2 {
                        return Err(ParseError::MalformedContent(
                            "mismatched bases in pure diff conflict".to_string(),
                        ));
                    }
                    (left_text, right_text, Some(base1))
                };

                let hunk = ConflictHunk {
                    id: HunkId(hunk_id_counter),
                    left: HunkContent { text: left },
                    right: HunkContent { text: right },
                    base: base.map(|b| HunkContent { text: b }),
                    context: HunkContext {
                        before,
                        after: Vec::new(),
                        start_line_left: section1_content_start,
                        start_line_right: section2_content_start,
                    },
                    state: HunkState::Unresolved,
                };

                let hunk_index = hunks.len();
                hunks.push(hunk);
                segments.push(Segment::Conflict(hunk_index));
                end_lines.push(one_indexed); // Track end marker position

                hunk_id_counter += 1;
                snapshot_buffer.clear();
                diff1_buffer.clear();
                diff2_buffer.clear();
                section_count = 0;
                has_snapshot = false;
                state = JjDiffState::Clean;
            }

            // End marker with only one section
            (
                Some(JjDiffMarker::End),
                JjDiffState::InSnapshotSide | JjDiffState::InDiffSection(1),
            ) => {
                return Err(ParseError::MalformedContent(
                    "conflict has only one section, expected two".to_string(),
                ));
            }

            // End marker in wrong state
            (Some(JjDiffMarker::End), _) => {
                return Err(ParseError::InvalidMarkers(format!(
                    "unexpected end marker at line {one_indexed}"
                )));
            }

            // Regular content lines
            (None, JjDiffState::Clean) => {
                clean_buffer.push((*line).to_string());
            }

            (None, JjDiffState::AwaitingFirstSection) => {
                return Err(ParseError::InvalidMarkers(format!(
                    "unexpected content before first section marker at line {one_indexed}"
                )));
            }

            (None, JjDiffState::InSnapshotSide) => {
                snapshot_buffer.push((*line).to_string());
            }

            (None, JjDiffState::InDiffSection(1)) => {
                diff1_buffer.push((*line).to_string());
            }

            (None, JjDiffState::InDiffSection(2)) => {
                diff2_buffer.push((*line).to_string());
            }

            // Unreachable but needed for exhaustiveness
            (None | Some(JjDiffMarker::DiffSide), JjDiffState::InDiffSection(_)) => unreachable!(),
        }
    }

    // Check for unclosed conflict at EOF
    if state != JjDiffState::Clean {
        return Err(ParseError::InvalidMarkers(format!(
            "unclosed conflict starting at line {hunk_start_line}"
        )));
    }

    // Flush remaining clean content
    if !clean_buffer.is_empty() {
        segments.push(Segment::Clean(clean_buffer.join("\n")));
    }

    // Fill in 'after' context using explicit end-marker positions
    fill_after_context_with_ends(&mut hunks, &lines, &end_lines);

    let format = if hunks.is_empty() {
        None
    } else {
        Some(ConflictFormat::JjDiff)
    };

    Ok(ParsedConflict {
        hunks,
        segments,
        format,
    })
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

    #[test]
    fn git_format_detected() {
        let content = r"before
<<<<<<< HEAD
left
=======
right
>>>>>>> feature
after";
        let result = parse_conflict_markers(content).unwrap();
        assert_eq!(result.format, Some(ConflictFormat::Git));
    }

    #[test]
    fn no_conflict_format_is_none() {
        let content = "just normal content";
        let result = parse_conflict_markers(content).unwrap();
        assert_eq!(result.format, None);
    }

    // --- jj Snapshot Parser Tests ---

    #[test]
    fn parse_jj_snapshot_basic() {
        let content = r"before
<<<<<<< Conflict 1 of 1
+++++++ Side #1 (Commit ABC)
left content
------- Base
base content
+++++++ Side #2 (Commit DEF)
right content
>>>>>>> Conflict 1 of 1
after";

        let result = parse_conflict_markers(content).unwrap();
        assert_eq!(result.hunks.len(), 1);
        assert_eq!(result.hunks[0].left.text, "left content");
        assert_eq!(result.hunks[0].right.text, "right content");
        assert!(result.hunks[0].base.is_some());
        assert_eq!(result.hunks[0].base.as_ref().unwrap().text, "base content");
        assert_eq!(result.format, Some(ConflictFormat::JjSnapshot));
    }

    #[test]
    fn parse_jj_snapshot_no_base() {
        let content = r"<<<<<<< Conflict 1 of 1
+++++++ Side #1
left content
+++++++ Side #2
right content
>>>>>>> Conflict 1 of 1";

        let result = parse_conflict_markers(content).unwrap();
        assert_eq!(result.hunks.len(), 1);
        assert_eq!(result.hunks[0].left.text, "left content");
        assert_eq!(result.hunks[0].right.text, "right content");
        assert!(result.hunks[0].base.is_none());
    }

    #[test]
    fn parse_jj_snapshot_variable_length_markers() {
        let content = "before\n\
            <<<<<<<<<< Conflict 1 of 1\n\
            ++++++++++ Side #1\n\
            left content\n\
            ---------- Base\n\
            base content\n\
            ++++++++++ Side #2\n\
            right content\n\
            >>>>>>>>>> Conflict 1 of 1\n\
            after";

        let result = parse_conflict_markers(content).unwrap();
        assert_eq!(result.hunks.len(), 1);
        assert_eq!(result.hunks[0].left.text, "left content");
        assert_eq!(result.hunks[0].right.text, "right content");
        assert_eq!(result.hunks[0].base.as_ref().unwrap().text, "base content");
    }

    #[test]
    fn parse_jj_snapshot_with_context() {
        let content = r"line 1
line 2
line 3
line 4
<<<<<<< Conflict 1 of 1
+++++++ Side #1
left
------- Base
base
+++++++ Side #2
right
>>>>>>> Conflict 1 of 1
line 5
line 6
line 7
line 8";

        let result = parse_conflict_markers(content).unwrap();
        assert_eq!(result.hunks[0].context.before.len(), 3);
        assert_eq!(result.hunks[0].context.before[0], "line 2");
        assert_eq!(result.hunks[0].context.before[2], "line 4");
        assert_eq!(result.hunks[0].context.after.len(), 3);
        assert_eq!(result.hunks[0].context.after[0], "line 5");
        assert_eq!(result.hunks[0].context.after[2], "line 7");
    }

    #[test]
    fn parse_jj_snapshot_multiple_hunks() {
        let content = r"before
<<<<<<< Conflict 1 of 2
+++++++ Side #1
first left
------- Base
first base
+++++++ Side #2
first right
>>>>>>> Conflict 1 of 2
middle
<<<<<<< Conflict 2 of 2
+++++++ Side #1
second left
------- Base
second base
+++++++ Side #2
second right
>>>>>>> Conflict 2 of 2
after";

        let result = parse_conflict_markers(content).unwrap();
        assert_eq!(result.hunks.len(), 2);
        assert_eq!(result.hunks[0].left.text, "first left");
        assert_eq!(result.hunks[0].right.text, "first right");
        assert_eq!(result.hunks[1].left.text, "second left");
        assert_eq!(result.hunks[1].right.text, "second right");
        assert_eq!(result.hunks[0].id, HunkId(0));
        assert_eq!(result.hunks[1].id, HunkId(1));
    }

    #[test]
    fn parse_jj_snapshot_empty_sides() {
        let content = r"<<<<<<< Conflict 1 of 1
+++++++ Side #1
+++++++ Side #2
>>>>>>> Conflict 1 of 1";

        let result = parse_conflict_markers(content).unwrap();
        assert_eq!(result.hunks[0].left.text, "");
        assert_eq!(result.hunks[0].right.text, "");
    }

    #[test]
    fn parse_jj_snapshot_preserves_whitespace() {
        let content = "<<<<<<< Conflict 1 of 1\n\
            +++++++ Side #1\n\
            \x20\x20indented content\x20\x20\n\
            ------- Base\n\
            base\n\
            +++++++ Side #2\n\
            \ttabbed content\t\n\
            >>>>>>> Conflict 1 of 1";

        let result = parse_conflict_markers(content).unwrap();
        assert_eq!(result.hunks[0].left.text, "  indented content  ");
        assert_eq!(result.hunks[0].right.text, "\ttabbed content\t");
    }

    #[test]
    fn parse_jj_snapshot_error_nested() {
        let content = r"<<<<<<< Conflict 1 of 1
+++++++ Side #1
<<<<<<< nested
+++++++ Side #1
left
+++++++ Side #2
right
>>>>>>> nested
+++++++ Side #2
right
>>>>>>> Conflict 1 of 1";

        let result = parse_conflict_markers(content);
        assert!(matches!(result, Err(ParseError::InvalidMarkers(msg)) if msg.contains("nested")));
    }

    #[test]
    fn parse_jj_snapshot_error_unclosed() {
        let content = r"<<<<<<< Conflict 1 of 1
+++++++ Side #1
left content
------- Base
base content";

        let result = parse_conflict_markers(content);
        assert!(matches!(result, Err(ParseError::InvalidMarkers(msg)) if msg.contains("unclosed")));
    }

    #[test]
    fn parse_jj_snapshot_error_unexpected_end() {
        let content = r">>>>>>> Conflict 1 of 1";

        let result = parse_conflict_markers(content);
        assert!(
            matches!(result, Err(ParseError::InvalidMarkers(msg)) if msg.contains("unexpected end marker"))
        );
    }

    #[test]
    fn parse_jj_snapshot_error_multi_sided() {
        let content = r"<<<<<<< Conflict 1 of 1
+++++++ Side #1
left
+++++++ Side #2
right
+++++++ Side #3
third
>>>>>>> Conflict 1 of 1";

        let result = parse_conflict_markers(content);
        assert!(
            matches!(result, Err(ParseError::MalformedContent(msg)) if msg.contains("multi-sided"))
        );
    }

    #[test]
    fn parse_jj_snapshot_segments_structure() {
        let content = r"before
<<<<<<< Conflict 1 of 1
+++++++ Side #1
left
+++++++ Side #2
right
>>>>>>> Conflict 1 of 1
after";

        let result = parse_conflict_markers(content).unwrap();
        assert_eq!(result.segments.len(), 3);
        assert!(matches!(&result.segments[0], Segment::Clean(s) if s == "before"));
        assert!(matches!(&result.segments[1], Segment::Conflict(0)));
        assert!(matches!(&result.segments[2], Segment::Clean(s) if s == "after"));
    }

    // --- reconstruct_from_diff tests ---

    #[test]
    fn reconstruct_from_diff_basic() {
        let lines = vec![
            " context".to_string(),
            "-base only".to_string(),
            "+side only".to_string(),
        ];
        let (base, side) = reconstruct_from_diff(&lines).unwrap();
        assert_eq!(base, "context\nbase only");
        assert_eq!(side, "context\nside only");
    }

    #[test]
    fn reconstruct_from_diff_context_only() {
        let lines = vec![" line1".to_string(), " line2".to_string()];
        let (base, side) = reconstruct_from_diff(&lines).unwrap();
        assert_eq!(base, "line1\nline2");
        assert_eq!(side, "line1\nline2");
    }

    #[test]
    fn reconstruct_from_diff_empty() {
        let lines: Vec<String> = vec![];
        let (base, side) = reconstruct_from_diff(&lines).unwrap();
        assert_eq!(base, "");
        assert_eq!(side, "");
    }

    #[test]
    fn reconstruct_from_diff_all_added() {
        let lines = vec!["+new line 1".to_string(), "+new line 2".to_string()];
        let (base, side) = reconstruct_from_diff(&lines).unwrap();
        assert_eq!(base, "");
        assert_eq!(side, "new line 1\nnew line 2");
    }

    #[test]
    fn reconstruct_from_diff_all_removed() {
        let lines = vec!["-old line 1".to_string(), "-old line 2".to_string()];
        let (base, side) = reconstruct_from_diff(&lines).unwrap();
        assert_eq!(base, "old line 1\nold line 2");
        assert_eq!(side, "");
    }

    #[test]
    fn reconstruct_from_diff_empty_lines_are_context() {
        let lines = vec![" first".to_string(), String::new(), " third".to_string()];
        let (base, side) = reconstruct_from_diff(&lines).unwrap();
        assert_eq!(base, "first\n\nthird");
        assert_eq!(side, "first\n\nthird");
    }

    #[test]
    fn reconstruct_from_diff_invalid_prefix() {
        let lines = vec!["xbad line".to_string()];
        let result = reconstruct_from_diff(&lines);
        assert!(
            matches!(result, Err(ParseError::MalformedContent(msg)) if msg.contains("unexpected prefix"))
        );
    }

    // --- jj Diff Parser Tests ---

    #[test]
    fn parse_jj_diff_mixed_basic() {
        let content = concat!(
            "before\n",
            "<<<<<<< Conflict 1 of 1\n",
            "+++++++ Side #1\n",
            "left content\n",
            "%%%%%%% Side #2\n",
            " context line\n",
            "-base only\n",
            "+right only\n",
            ">>>>>>> Conflict 1 of 1\n",
            "after",
        );

        let result = parse_conflict_markers(content).unwrap();
        assert_eq!(result.hunks.len(), 1);
        assert_eq!(result.format, Some(ConflictFormat::JjDiff));

        let hunk = &result.hunks[0];
        assert_eq!(hunk.left.text, "left content");
        assert_eq!(hunk.right.text, "context line\nright only");
        assert!(hunk.base.is_some());
        assert_eq!(hunk.base.as_ref().unwrap().text, "context line\nbase only");
    }

    #[test]
    fn parse_jj_diff_mixed_empty_snapshot() {
        let content = concat!(
            "<<<<<<< Conflict 1 of 1\n",
            "+++++++ Side #1\n",
            "%%%%%%% Side #2\n",
            " context\n",
            "-removed\n",
            ">>>>>>> Conflict 1 of 1",
        );

        let result = parse_conflict_markers(content).unwrap();
        assert_eq!(result.hunks.len(), 1);
        assert_eq!(result.hunks[0].left.text, "");
        assert_eq!(result.hunks[0].right.text, "context");
        assert_eq!(
            result.hunks[0].base.as_ref().unwrap().text,
            "context\nremoved"
        );
    }

    #[test]
    fn parse_jj_diff_pure_basic() {
        let content = concat!(
            "before\n",
            "<<<<<<< Conflict 1 of 1\n",
            "%%%%%%% Side #1\n",
            " context\n",
            "-base only\n",
            "+side 1 only\n",
            "%%%%%%% Side #2\n",
            " context\n",
            "-base only\n",
            "+side 2 only\n",
            ">>>>>>> Conflict 1 of 1\n",
            "after",
        );

        let result = parse_conflict_markers(content).unwrap();
        assert_eq!(result.hunks.len(), 1);
        assert_eq!(result.format, Some(ConflictFormat::JjDiff));

        let hunk = &result.hunks[0];
        assert_eq!(hunk.left.text, "context\nside 1 only");
        assert_eq!(hunk.right.text, "context\nside 2 only");
        assert!(hunk.base.is_some());
        assert_eq!(hunk.base.as_ref().unwrap().text, "context\nbase only");
    }

    #[test]
    fn parse_jj_diff_multi_hunk() {
        let content = concat!(
            "header\n",
            "<<<<<<< Conflict 1 of 2\n",
            "+++++++ Side #1\n",
            "first left\n",
            "%%%%%%% Side #2\n",
            " first context\n",
            "-first base\n",
            "+first right\n",
            ">>>>>>> Conflict 1 of 2\n",
            "middle\n",
            "<<<<<<< Conflict 2 of 2\n",
            "+++++++ Side #1\n",
            "second left\n",
            "%%%%%%% Side #2\n",
            " second context\n",
            "-second base\n",
            "+second right\n",
            ">>>>>>> Conflict 2 of 2\n",
            "footer",
        );

        let result = parse_conflict_markers(content).unwrap();
        assert_eq!(result.hunks.len(), 2);
        assert_eq!(result.hunks[0].left.text, "first left");
        assert_eq!(result.hunks[0].right.text, "first context\nfirst right");
        assert_eq!(result.hunks[1].left.text, "second left");
        assert_eq!(result.hunks[1].right.text, "second context\nsecond right");
        assert_eq!(result.hunks[0].id, HunkId(0));
        assert_eq!(result.hunks[1].id, HunkId(1));
    }

    #[test]
    fn parse_jj_diff_context_capture() {
        let content = concat!(
            "line 1\n",
            "line 2\n",
            "line 3\n",
            "line 4\n",
            "<<<<<<< Conflict 1 of 1\n",
            "+++++++ Side #1\n",
            "left\n",
            "%%%%%%% Side #2\n",
            " ctx\n",
            "-base\n",
            "+right\n",
            ">>>>>>> Conflict 1 of 1\n",
            "line 5\n",
            "line 6\n",
            "line 7\n",
            "line 8",
        );

        let result = parse_conflict_markers(content).unwrap();
        assert_eq!(result.hunks[0].context.before.len(), 3);
        assert_eq!(result.hunks[0].context.before[0], "line 2");
        assert_eq!(result.hunks[0].context.before[2], "line 4");
        assert_eq!(result.hunks[0].context.after.len(), 3);
        assert_eq!(result.hunks[0].context.after[0], "line 5");
        assert_eq!(result.hunks[0].context.after[2], "line 7");
    }

    #[test]
    fn parse_jj_diff_variable_length_markers() {
        let content = concat!(
            "<<<<<<<<<< Conflict 1 of 1\n",
            "++++++++++ Side #1\n",
            "left\n",
            "%%%%%%%%%% Side #2\n",
            " ctx\n",
            "-base\n",
            "+right\n",
            ">>>>>>>>>> Conflict 1 of 1",
        );

        let result = parse_conflict_markers(content).unwrap();
        assert_eq!(result.hunks.len(), 1);
        assert_eq!(result.hunks[0].left.text, "left");
        assert_eq!(result.hunks[0].right.text, "ctx\nright");
    }

    #[test]
    fn parse_jj_diff_error_unclosed() {
        let content = concat!(
            "<<<<<<< Conflict 1 of 1\n",
            "+++++++ Side #1\n",
            "left\n",
            "%%%%%%% Side #2\n",
            " context",
        );

        let result = parse_conflict_markers(content);
        assert!(matches!(result, Err(ParseError::InvalidMarkers(msg)) if msg.contains("unclosed")));
    }

    #[test]
    fn parse_jj_diff_error_single_section() {
        let content = concat!(
            "<<<<<<< Conflict 1 of 1\n",
            "%%%%%%% Side #1\n",
            " context\n",
            ">>>>>>> Conflict 1 of 1",
        );

        let result = parse_conflict_markers(content);
        assert!(
            matches!(result, Err(ParseError::MalformedContent(msg)) if msg.contains("only one section"))
        );
    }

    #[test]
    fn parse_jj_diff_error_nested() {
        let content = concat!(
            "<<<<<<< Conflict 1 of 1\n",
            "+++++++ Side #1\n",
            "<<<<<<< nested\n",
            "+++++++ Side #1\n",
            "left\n",
            "%%%%%%% Side #2\n",
            " ctx\n",
            ">>>>>>> nested\n",
            "%%%%%%% Side #2\n",
            " ctx\n",
            ">>>>>>> Conflict 1 of 1",
        );

        let result = parse_conflict_markers(content);
        assert!(matches!(result, Err(ParseError::InvalidMarkers(msg)) if msg.contains("nested")));
    }

    #[test]
    fn parse_jj_diff_error_mismatched_bases() {
        let content = concat!(
            "<<<<<<< Conflict 1 of 1\n",
            "%%%%%%% Side #1\n",
            " same context\n",
            "-different base 1\n",
            "+side 1\n",
            "%%%%%%% Side #2\n",
            " same context\n",
            "-different base 2\n",
            "+side 2\n",
            ">>>>>>> Conflict 1 of 1",
        );

        let result = parse_conflict_markers(content);
        assert!(
            matches!(result, Err(ParseError::MalformedContent(msg)) if msg.contains("mismatched bases"))
        );
    }

    #[test]
    fn parse_jj_diff_error_multi_sided() {
        let content = concat!(
            "<<<<<<< Conflict 1 of 1\n",
            "+++++++ Side #1\n",
            "left\n",
            "%%%%%%% Side #2\n",
            " ctx\n",
            "%%%%%%% Side #3\n",
            " ctx\n",
            ">>>>>>> Conflict 1 of 1",
        );

        let result = parse_conflict_markers(content);
        assert!(
            matches!(result, Err(ParseError::MalformedContent(msg)) if msg.contains("multi-sided"))
        );
    }

    #[test]
    fn parse_jj_diff_segments_structure() {
        let content = concat!(
            "before\n",
            "<<<<<<< Conflict 1 of 1\n",
            "+++++++ Side #1\n",
            "left\n",
            "%%%%%%% Side #2\n",
            " ctx\n",
            "-base\n",
            "+right\n",
            ">>>>>>> Conflict 1 of 1\n",
            "after",
        );

        let result = parse_conflict_markers(content).unwrap();
        assert_eq!(result.segments.len(), 3);
        assert!(matches!(&result.segments[0], Segment::Clean(s) if s == "before"));
        assert!(matches!(&result.segments[1], Segment::Conflict(0)));
        assert!(matches!(&result.segments[2], Segment::Clean(s) if s == "after"));
    }
}
