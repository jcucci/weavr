use crate::{
    ConflictFormat, ConflictHunk, HunkContent, HunkContext, HunkId, HunkState, ParseError,
};

use super::{ParsedConflict, Segment, DEFAULT_CONTEXT_LINES};

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

/// Fills in 'after' context for all hunks using explicit marker line positions.
///
/// Unlike `fill_after_context`, this variant takes explicit `<<<<<<<` and `>>>>>>>`
/// line positions rather than computing them from content start lines, which
/// doesn't work for diff format since diff lines != reconstructed content lines.
fn fill_after_context_with_ends(
    hunks: &mut [ConflictHunk],
    lines: &[&str],
    start_lines: &[usize],
    end_lines: &[usize],
) {
    let hunk_count = hunks.len();

    for hunk_index in 0..hunk_count {
        let end_marker_line = end_lines[hunk_index]; // 1-indexed line of >>>>>>>

        // Collect up to DEFAULT_CONTEXT_LINES after the end marker
        let after_start = end_marker_line; // 0-indexed start = end_marker_line (1-indexed is line after)
        let after_end = (after_start + DEFAULT_CONTEXT_LINES).min(lines.len());

        // Use the actual <<<<<<< line of the next conflict for boundary checking
        let next_conflict_start = if hunk_index + 1 < hunk_count {
            start_lines[hunk_index + 1] - 1 // Convert 1-indexed to 0-indexed
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
pub(super) fn parse_jj_diff_markers(content: &str) -> Result<ParsedConflict, ParseError> {
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
    let mut start_lines: Vec<usize> = Vec::new();
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

                // Capture the verbatim conflict marker text for partial save
                let original_text = lines[hunk_start_line - 1..=line_num].join("\n");

                let hunk = ConflictHunk {
                    id: HunkId(hunk_id_counter),
                    left: HunkContent { text: left },
                    right: HunkContent { text: right },
                    base: base.map(|b| HunkContent { text: b }),
                    extra_sides: vec![],
                    extra_bases: vec![],
                    context: HunkContext {
                        before,
                        after: Vec::new(),
                        start_line_left: section1_content_start,
                        start_line_right: section2_content_start,
                    },
                    state: HunkState::Unresolved,
                    original_text: Some(original_text),
                };

                let hunk_index = hunks.len();
                hunks.push(hunk);
                segments.push(Segment::Conflict(hunk_index));
                start_lines.push(hunk_start_line); // Track <<<<<<< position
                end_lines.push(one_indexed); // Track >>>>>>> position

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
    fill_after_context_with_ends(&mut hunks, &lines, &start_lines, &end_lines);

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
    use super::reconstruct_from_diff;
    use crate::{parse_conflict_markers, ConflictFormat, HunkId, ParseError, Segment};

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

    #[test]
    fn original_text_captured_jj_diff() {
        let content = concat!(
            "before\n",
            "<<<<<<< Conflict 1 of 1\n",
            "+++++++ Side #1\n",
            "left\n",
            "%%%%%%% Side #2\n",
            " base\n",
            "-base\n",
            "+right\n",
            ">>>>>>> Conflict 1 of 1\n",
            "after",
        );
        let result = parse_conflict_markers(content).unwrap();
        assert_eq!(result.hunks.len(), 1);
        let expected = concat!(
            "<<<<<<< Conflict 1 of 1\n",
            "+++++++ Side #1\n",
            "left\n",
            "%%%%%%% Side #2\n",
            " base\n",
            "-base\n",
            "+right\n",
            ">>>>>>> Conflict 1 of 1",
        );
        assert_eq!(result.hunks[0].original_text.as_deref(), Some(expected));
    }
}
