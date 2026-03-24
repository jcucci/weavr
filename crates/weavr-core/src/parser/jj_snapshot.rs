use crate::{
    ConflictFormat, ConflictHunk, HunkContent, HunkContext, HunkId, HunkState, ParseError,
};

use super::{fill_after_context, ParsedConflict, Segment, DEFAULT_CONTEXT_LINES};

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
pub(super) fn parse_jj_snapshot_markers(content: &str) -> Result<ParsedConflict, ParseError> {
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

                // Capture the verbatim conflict marker text for partial save
                let original_text = lines[hunk_start_line - 1..=line_num].join("\n");

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
                    extra_sides: vec![],
                    extra_bases: vec![],
                    context: HunkContext {
                        before,
                        after: Vec::new(),
                        start_line_left: side1_content_start,
                        start_line_right: side2_content_start,
                    },
                    state: HunkState::Unresolved,
                    original_text: Some(original_text),
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

#[cfg(test)]
mod tests {
    use crate::{parse_conflict_markers, ConflictFormat, HunkId, ParseError, Segment};

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

    #[test]
    fn original_text_captured_jj_snapshot() {
        let content = "before\n<<<<<<< Conflict 1 of 1\n+++++++ Side #1\nleft\n+++++++ Side #2\nright\n>>>>>>> Conflict 1 of 1\nafter";
        let result = parse_conflict_markers(content).unwrap();
        assert_eq!(result.hunks.len(), 1);
        let expected = "<<<<<<< Conflict 1 of 1\n+++++++ Side #1\nleft\n+++++++ Side #2\nright\n>>>>>>> Conflict 1 of 1";
        assert_eq!(result.hunks[0].original_text.as_deref(), Some(expected));
    }
}
