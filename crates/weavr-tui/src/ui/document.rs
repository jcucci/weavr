//! Document building for the three-pane merge view.
//!
//! This module constructs `Line<'_>` sequences from merge segments and hunks,
//! handling conflict markers, syntax highlighting, and word-level diff display.

use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};
use similar::ChangeTag;
use weavr_core::{HunkState, Segment};

use crate::ai::AiState;
use crate::ast::AstState;
use crate::diff::{compute_line_diffs, compute_word_diffs, DiffConfig, DiffLine};
use crate::highlight::{self, HighlightedDocument};

use super::pane::PaneSide;

/// Builds the full document content for a side pane (left or right).
pub(super) fn build_side_document<'a>(
    segments: &[Segment],
    hunks: &[weavr_core::ConflictHunk],
    side: PaneSide,
    current_hunk_idx: usize,
    theme: &'a crate::theme::Theme,
    diff_config: DiffConfig,
    highlight_doc: Option<&HighlightedDocument>,
) -> Vec<Line<'a>> {
    let mut lines = Vec::new();
    let mut line_number = 1;

    for segment in segments {
        match segment {
            Segment::Clean(text) => {
                for line_text in text.lines() {
                    lines.push(build_highlighted_or_plain_line(
                        line_number,
                        line_text,
                        Style::default().fg(theme.base.foreground),
                        false,
                        highlight_doc,
                    ));
                    line_number += 1;
                }
            }
            Segment::Conflict(hunk_idx) => {
                let hunk = &hunks[*hunk_idx];
                let is_current = *hunk_idx == current_hunk_idx;

                // Compute diff between left and right sides
                let diffs = compute_line_diffs(&hunk.left.text, &hunk.right.text);

                // Select the appropriate diff lines for this side
                let diff_lines = match side {
                    PaneSide::Left => &diffs.left_lines,
                    PaneSide::Right => &diffs.right_lines,
                };

                // Base style for the side (used for conflict markers)
                let side_style = match side {
                    PaneSide::Left => theme.conflict.left,
                    PaneSide::Right => theme.conflict.right,
                };

                // Add marker for conflict start
                if is_current {
                    lines.push(Line::from(Span::styled(
                        format!("──── Conflict {} ────", hunk_idx + 1),
                        side_style.add_modifier(Modifier::BOLD),
                    )));
                }

                for diff_line in diff_lines {
                    if diff_config.word_diff && diff_line.counterpart.is_some() {
                        lines.push(build_word_diff_line(
                            line_number,
                            diff_line,
                            side,
                            theme,
                            is_current,
                        ));
                    } else {
                        // Apply style based on diff tag
                        let style = match diff_line.tag {
                            ChangeTag::Equal => theme.diff.context,
                            ChangeTag::Delete => theme.diff.removed,
                            ChangeTag::Insert => theme.diff.added,
                        };

                        lines.push(build_line(line_number, &diff_line.text, style, is_current));
                    }
                    line_number += 1;
                }

                if is_current {
                    lines.push(Line::from(Span::styled(
                        "────────────────────",
                        side_style.add_modifier(Modifier::BOLD),
                    )));
                }
            }
        }
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "(empty file)",
            Style::default().fg(theme.base.muted),
        )));
    }

    lines
}

/// Builds the full document content for the base (ancestor) pane.
pub(super) fn build_base_document<'a>(
    segments: &[Segment],
    hunks: &[weavr_core::ConflictHunk],
    current_hunk_idx: usize,
    theme: &'a crate::theme::Theme,
    highlight_doc: Option<&HighlightedDocument>,
) -> Vec<Line<'a>> {
    let mut lines = Vec::new();
    let mut line_number = 1;

    for segment in segments {
        match segment {
            Segment::Clean(text) => {
                for line_text in text.lines() {
                    lines.push(build_highlighted_or_plain_line(
                        line_number,
                        line_text,
                        Style::default().fg(theme.base.foreground),
                        false,
                        highlight_doc,
                    ));
                    line_number += 1;
                }
            }
            Segment::Conflict(hunk_idx) => {
                let hunk = &hunks[*hunk_idx];
                let is_current = *hunk_idx == current_hunk_idx;
                let base_style = theme.conflict.base;

                if is_current {
                    lines.push(Line::from(Span::styled(
                        format!("──── Conflict {} ────", hunk_idx + 1),
                        base_style.add_modifier(Modifier::BOLD),
                    )));
                }

                if let Some(base_content) = &hunk.base {
                    for line_text in base_content.text.lines() {
                        lines.push(build_line(line_number, line_text, base_style, is_current));
                        line_number += 1;
                    }
                } else {
                    lines.push(Line::from(Span::styled(
                        "(no base — 2-way conflict)".to_string(),
                        Style::default()
                            .fg(theme.base.muted)
                            .add_modifier(Modifier::ITALIC),
                    )));
                }

                if is_current {
                    lines.push(Line::from(Span::styled(
                        "────────────────────",
                        base_style.add_modifier(Modifier::BOLD),
                    )));
                }
            }
        }
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "(empty file)",
            Style::default().fg(theme.base.muted),
        )));
    }

    lines
}

/// Builds the full document content for the result pane.
#[allow(clippy::too_many_lines)]
pub(super) fn build_result_document<'a>(
    segments: &[Segment],
    hunks: &[weavr_core::ConflictHunk],
    current_hunk_idx: usize,
    theme: &'a crate::theme::Theme,
    ai_state: &AiState,
    ast_state: &AstState,
    highlight_doc: Option<&HighlightedDocument>,
) -> Vec<Line<'a>> {
    let mut lines = Vec::new();
    let mut line_number = 1;

    for segment in segments {
        match segment {
            Segment::Clean(text) => {
                for line_text in text.lines() {
                    lines.push(build_highlighted_or_plain_line(
                        line_number,
                        line_text,
                        Style::default().fg(theme.base.foreground),
                        false,
                        highlight_doc,
                    ));
                    line_number += 1;
                }
            }
            Segment::Conflict(hunk_idx) => {
                let hunk = &hunks[*hunk_idx];
                let is_current = *hunk_idx == current_hunk_idx;

                if let HunkState::Resolved(resolution) = &hunk.state {
                    // Show resolved content
                    let style = theme.conflict.resolved;
                    let hunk_num = hunk_idx + 1;
                    if is_current {
                        lines.push(Line::from(Span::styled(
                            format!("──── Resolved {hunk_num} ────"),
                            style.add_modifier(Modifier::BOLD),
                        )));
                    }
                    for line_text in resolution.content.lines() {
                        lines.push(build_line(line_number, line_text, style, is_current));
                        line_number += 1;
                    }
                    if is_current {
                        lines.push(Line::from(Span::styled(
                            "────────────────────",
                            style.add_modifier(Modifier::BOLD),
                        )));
                    }
                } else if is_current && ast_state.has_suggestion_for(hunk.id) {
                    // AST merge suggestion ghost text
                    let suggestion = ast_state.suggestion_for(hunk.id).unwrap();
                    let ghost_style = Style::default()
                        .fg(theme.base.muted)
                        .add_modifier(Modifier::ITALIC);
                    let header_style = ghost_style.add_modifier(Modifier::BOLD);

                    let conf_str = suggestion
                        .confidence
                        .map(|c| format!(" ({c}%)"))
                        .unwrap_or_default();
                    let desc_str = if suggestion.description.is_empty() {
                        String::new()
                    } else {
                        format!(": {}", suggestion.description)
                    };
                    lines.push(Line::from(Span::styled(
                        format!("──── AST Merge{conf_str}{desc_str} ────"),
                        header_style,
                    )));
                    for line_text in suggestion.resolution.content.lines() {
                        lines.push(Line::from(vec![
                            Span::styled(
                                "   ~ ".to_string(),
                                Style::default().add_modifier(Modifier::DIM),
                            ),
                            Span::styled(line_text.to_string(), ghost_style),
                        ]));
                    }
                    lines.push(Line::from(Span::styled(
                        "  [Enter] Accept  [Esc] Dismiss",
                        Style::default().fg(theme.base.muted),
                    )));
                    lines.push(Line::from(Span::styled(
                        "────────────────────",
                        header_style,
                    )));
                } else if is_current && ai_state.has_suggestion_for(hunk.id) {
                    // AI suggestion ghost text
                    let suggestion = ai_state.suggestion_for(hunk.id).unwrap();
                    let ghost_style = Style::default()
                        .fg(theme.base.muted)
                        .add_modifier(Modifier::ITALIC);
                    let header_style = ghost_style.add_modifier(Modifier::BOLD);

                    let conf_str = suggestion
                        .confidence
                        .map(|c| format!(" ({c}%)"))
                        .unwrap_or_default();
                    lines.push(Line::from(Span::styled(
                        format!("──── AI Suggestion{conf_str} ────"),
                        header_style,
                    )));
                    for line_text in suggestion.resolution.content.lines() {
                        // Render ghost lines without consuming line numbers
                        // so subsequent real content retains correct numbering.
                        lines.push(Line::from(vec![
                            Span::styled(
                                "   ~ ".to_string(),
                                Style::default().add_modifier(Modifier::DIM),
                            ),
                            Span::styled(line_text.to_string(), ghost_style),
                        ]));
                    }
                    lines.push(Line::from(Span::styled(
                        "  [Enter] Accept  [Esc] Dismiss  [?] Explain",
                        Style::default().fg(theme.base.muted),
                    )));
                    lines.push(Line::from(Span::styled(
                        "────────────────────",
                        header_style,
                    )));
                } else if is_current && ai_state.pending_hunk == Some(hunk.id) {
                    // Loading spinner
                    let style = theme.conflict.unresolved;
                    let spinner = ai_state.spinner_char();
                    lines.push(Line::from(Span::styled(
                        format!("──── {spinner} AI thinking... ────"),
                        style.add_modifier(Modifier::BOLD),
                    )));
                    lines.push(Line::from(Span::styled(
                        "  Select: [o]urs  [t]heirs  [b]oth",
                        Style::default().fg(theme.base.muted),
                    )));
                    lines.push(Line::from(Span::styled(
                        "────────────────────",
                        style.add_modifier(Modifier::BOLD),
                    )));
                } else {
                    // Unresolved: show placeholder
                    let style = theme.conflict.unresolved;
                    let hunk_num = hunk_idx + 1;
                    let marker = if is_current {
                        format!("──── UNRESOLVED {hunk_num} [?] ────")
                    } else {
                        format!("──── unresolved {hunk_num} ────")
                    };
                    lines.push(Line::from(Span::styled(
                        marker,
                        style.add_modifier(Modifier::BOLD),
                    )));
                    lines.push(Line::from(Span::styled(
                        "  Select: [o]urs  [t]heirs  [b]oth",
                        Style::default().fg(theme.base.muted),
                    )));
                    if is_current {
                        lines.push(Line::from(Span::styled(
                            "────────────────────",
                            style.add_modifier(Modifier::BOLD),
                        )));
                    }
                }
            }
        }
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "(empty file)",
            Style::default().fg(theme.base.muted),
        )));
    }

    lines
}

/// Builds a single line with line number and content.
fn build_line(line_number: usize, text: &str, style: Style, highlight: bool) -> Line<'static> {
    let line_num_style = if highlight {
        Style::default()
            .fg(ratatui::style::Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().add_modifier(Modifier::DIM)
    };

    Line::from(vec![
        Span::styled(format!("{line_number:4} "), line_num_style),
        Span::styled(text.to_string(), style),
    ])
}

/// Builds a line using syntax highlighting if available, otherwise plain.
///
/// `line_number` is 1-based; the highlighted document is indexed 0-based.
/// Falls back to plain rendering if the cached span text doesn't match the
/// source `text`, guarding against stale cache misalignment.
fn build_highlighted_or_plain_line(
    line_number: usize,
    text: &str,
    fallback_style: Style,
    is_highlighted: bool,
    highlight_doc: Option<&HighlightedDocument>,
) -> Line<'static> {
    if let Some(doc) = highlight_doc {
        if let Some(spans_data) = doc.get_line_spans(line_number - 1) {
            if !spans_data.is_empty() {
                // Validate cached span text matches source to avoid rendering
                // wrong content from a stale or misaligned cache.
                let cached_text: String = spans_data.iter().map(|(_, s)| s.as_str()).collect();
                if cached_text == text {
                    return build_highlighted_line(line_number, spans_data, is_highlighted);
                }
            }
        }
    }
    build_line(line_number, text, fallback_style, is_highlighted)
}

/// Builds a line from syntax-highlighted spans.
fn build_highlighted_line(
    line_number: usize,
    hl_spans: &[(syntect::highlighting::Style, String)],
    is_highlighted: bool,
) -> Line<'static> {
    let line_num_style = if is_highlighted {
        Style::default()
            .fg(ratatui::style::Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().add_modifier(Modifier::DIM)
    };

    let mut spans = vec![Span::styled(format!("{line_number:4} "), line_num_style)];
    for (style, text) in hl_spans {
        spans.push(Span::styled(
            text.clone(),
            highlight::syntect_style_to_ratatui(*style),
        ));
    }
    Line::from(spans)
}

/// Builds a line with word-level diff highlighting.
///
/// Uses the counterpart text to compute word-level diffs and renders
/// each word segment with appropriate styling: base diff style for
/// unchanged words, `theme.diff.modified` for changed words.
pub(super) fn build_word_diff_line(
    line_number: usize,
    diff_line: &DiffLine,
    side: PaneSide,
    theme: &crate::theme::Theme,
    highlight: bool,
) -> Line<'static> {
    let line_num_style = if highlight {
        Style::default()
            .fg(ratatui::style::Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().add_modifier(Modifier::DIM)
    };

    let counterpart = diff_line.counterpart.as_deref().unwrap_or("");

    // Compute word diffs: for left side, our text is "old" and counterpart is "new";
    // for right side, counterpart is "old" and our text is "new".
    let word_changes = match side {
        PaneSide::Left => compute_word_diffs(&diff_line.text, counterpart),
        PaneSide::Right => compute_word_diffs(counterpart, &diff_line.text),
    };

    let base_style = match diff_line.tag {
        ChangeTag::Delete => theme.diff.removed,
        ChangeTag::Insert => theme.diff.added,
        ChangeTag::Equal => theme.diff.context,
    };
    let modified_style = theme.diff.modified;

    let mut spans = vec![Span::styled(format!("{line_number:4} "), line_num_style)];

    // Which tag represents "our" changed words?
    let own_change_tag = match side {
        PaneSide::Left => ChangeTag::Delete,
        PaneSide::Right => ChangeTag::Insert,
    };

    for word in word_changes {
        let style = if word.tag == ChangeTag::Equal {
            base_style
        } else if word.tag == own_change_tag {
            modified_style
        } else {
            // Skip words that belong to the other side
            continue;
        };
        spans.push(Span::styled(word.text, style));
    }

    Line::from(spans)
}
