//! Pane content rendering for the three-pane merge view.
//!
//! This module handles rendering the full document with conflicts highlighted
//! in the left, right, and result panes.

use std::time::Duration;

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};
use similar::ChangeTag;
use weavr_core::{HunkState, Segment};

use crate::ai::AiState;
use crate::ast::AstState;
use crate::diff::{compute_line_diffs, compute_word_diffs, DiffConfig, DiffLine};
use crate::highlight::{self, HighlightedDocument};
use crate::input::InputMode;
use crate::{App, FocusedPane};

/// Which side of the conflict to render.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneSide {
    /// Left side (`ours`/`HEAD`).
    Left,
    /// Right side (`theirs`/`MERGE_HEAD`).
    Right,
}

impl PaneSide {
    /// Returns the title for this side.
    fn title(self) -> &'static str {
        match self {
            Self::Left => "Left (Ours)",
            Self::Right => "Right (Theirs)",
        }
    }

    /// Returns the corresponding `FocusedPane`.
    fn focused_pane(self) -> FocusedPane {
        match self {
            Self::Left => FocusedPane::Left,
            Self::Right => FocusedPane::Right,
        }
    }
}

/// Renders the left pane showing the "ours" side of the document.
pub fn render_left_pane(frame: &mut Frame, area: Rect, app: &App) {
    render_side_pane(frame, area, app, PaneSide::Left);
}

/// Renders the right pane showing the "theirs" side of the document.
pub fn render_right_pane(frame: &mut Frame, area: Rect, app: &App) {
    render_side_pane(frame, area, app, PaneSide::Right);
}

/// Renders a side pane (left or right) with full document content.
fn render_side_pane(frame: &mut Frame, area: Rect, app: &App, side: PaneSide) {
    let theme = app.theme();
    let is_focused = app.focused_pane() == side.focused_pane();

    let border_style = if is_focused {
        Style::default().fg(theme.ui.border_focused)
    } else {
        Style::default().fg(theme.ui.border_unfocused)
    };

    let highlight_doc = if app.syntax_highlight() {
        app.highlight_cache().and_then(|cache| match side {
            PaneSide::Left => cache.left.as_ref(),
            PaneSide::Right => cache.right.as_ref(),
        })
    } else {
        None
    };

    let content = match app.session() {
        Some(session) => build_side_document(
            session.segments(),
            session.hunks(),
            side,
            app.current_hunk_index(),
            theme,
            *app.diff_config(),
            highlight_doc,
        ),
        None => vec![Line::from(Span::styled(
            "No file loaded",
            Style::default().fg(theme.base.muted),
        ))],
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title(format!(" {} ", side.title()));

    let paragraph = Paragraph::new(content)
        .block(block)
        .scroll((app.left_right_scroll(), 0));

    frame.render_widget(paragraph, area);
}

/// Renders the base pane showing the common ancestor content for diff3 conflicts.
pub fn render_base_pane(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme();
    let is_focused = app.focused_pane() == FocusedPane::Base;

    let border_style = if is_focused {
        Style::default().fg(theme.ui.border_focused)
    } else {
        Style::default().fg(theme.ui.border_unfocused)
    };

    let highlight_doc = if app.syntax_highlight() {
        app.highlight_cache().and_then(|cache| cache.base.as_ref())
    } else {
        None
    };

    let content = match app.session() {
        Some(session) => build_base_document(
            session.segments(),
            session.hunks(),
            app.current_hunk_index(),
            theme,
            highlight_doc,
        ),
        None => vec![Line::from(Span::styled(
            "No file loaded",
            Style::default().fg(theme.base.muted),
        ))],
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title(" Base (Ancestor) ");

    let paragraph = Paragraph::new(content)
        .block(block)
        .scroll((app.left_right_scroll(), 0));

    frame.render_widget(paragraph, area);
}

/// Renders the result pane showing the merged output.
pub fn render_result_pane(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme();
    let is_focused = app.focused_pane() == FocusedPane::Result;

    let border_style = if is_focused {
        Style::default().fg(theme.ui.border_focused)
    } else {
        Style::default().fg(theme.ui.border_unfocused)
    };

    let highlight_doc = if app.syntax_highlight() {
        app.highlight_cache()
            .and_then(|cache| cache.result.as_ref())
    } else {
        None
    };

    let content = match app.session() {
        Some(session) => build_result_document(
            session.segments(),
            session.hunks(),
            app.current_hunk_index(),
            theme,
            app.ai_state(),
            app.ast_state(),
            highlight_doc,
        ),
        None => vec![Line::from(Span::styled(
            "No file loaded",
            Style::default().fg(theme.base.muted),
        ))],
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title(" Result ");

    let paragraph = Paragraph::new(content)
        .block(block)
        .scroll((app.result_scroll(), 0));

    frame.render_widget(paragraph, area);
}

/// Renders the title bar with file path and hunk counter.
pub fn render_title_bar(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme();

    let hunk_info = if app.total_hunks() > 0 {
        let resolved_count = app.session().map_or(0, |s| {
            s.hunks()
                .iter()
                .filter(|h| matches!(h.state, HunkState::Resolved(_)))
                .count()
        });

        format!(
            "[{}/{}] ({} resolved)",
            app.current_hunk_index() + 1,
            app.total_hunks(),
            resolved_count
        )
    } else {
        "No conflicts".to_string()
    };

    let mut spans = vec![Span::styled(" weavr ", theme.ui.title), Span::raw("| ")];

    // Add file position indicator in multi-file mode
    if let Some(ref ws) = app.workspace {
        let file_path = ws.current().path.to_string_lossy();
        spans.push(Span::styled(
            format!(
                "[{}/{}] {file_path} ",
                app.current_file_index() + 1,
                app.file_count()
            ),
            Style::default().fg(theme.base.foreground),
        ));
        spans.push(Span::raw("| "));
    }

    spans.push(Span::styled(
        hunk_info,
        Style::default().fg(theme.base.accent),
    ));

    let title = Line::from(spans);
    let paragraph = Paragraph::new(title).style(theme.ui.title.bg(theme.base.background));
    frame.render_widget(paragraph, area);
}

/// Duration before status messages auto-clear.
const STATUS_MESSAGE_DURATION: Duration = Duration::from_secs(3);

/// Renders the status bar with context-sensitive help.
#[allow(clippy::too_many_lines)]
pub fn render_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme();

    // Command mode: show the command line
    if app.input_mode() == InputMode::Command {
        let cmd_line = format!(":{}", app.command_buffer());
        let status = Paragraph::new(cmd_line).style(
            Style::default()
                .fg(theme.base.foreground)
                .bg(theme.base.background),
        );
        frame.render_widget(status, area);
        return;
    }

    // Check for status message first (auto-clears after timeout)
    if let Some((msg, timestamp)) = app.status_message() {
        if timestamp.elapsed() < STATUS_MESSAGE_DURATION {
            let status = Paragraph::new(format!(" {msg}")).style(
                Style::default()
                    .fg(theme.base.accent)
                    .bg(theme.base.background),
            );
            frame.render_widget(status, area);
            return;
        }
    }

    // Calculate unresolved count
    let unresolved_count = app.session().map_or(0, |s| {
        s.hunks()
            .iter()
            .filter(|h| matches!(h.state, HunkState::Unresolved))
            .count()
    });

    // Build pane indicator
    let pane_name = match app.focused_pane() {
        FocusedPane::Left => "Left",
        FocusedPane::Base => "Base",
        FocusedPane::Right => "Right",
        FocusedPane::Result => "Result",
    };

    // Build file prefix for multi-file mode
    let file_prefix = if let Some(ref ws) = app.workspace {
        let name = ws.current().display_name();
        format!(
            "[{}/{}] {name} | ",
            app.current_file_index() + 1,
            app.file_count()
        )
    } else {
        String::new()
    };

    // Format: "[2/3] src/lib.rs | Hunk 1/4 | Left pane | 3 unresolved"
    let status_text = if app.total_hunks() > 0 {
        format!(
            " {file_prefix}Hunk {}/{} | {} pane | {} unresolved",
            app.current_hunk_index() + 1,
            app.total_hunks(),
            pane_name,
            unresolved_count
        )
    } else {
        format!(" {file_prefix}{pane_name} pane | No conflicts")
    };

    // Add AI indicator when AI is available
    let ai_indicator = if app.ai_available() {
        if app.ai_state().is_loading() {
            format!(" | AI {}", app.ai_state().spinner_char())
        } else if app
            .current_hunk()
            .is_some_and(|h| app.ai_state().has_suggestion_for(h.id))
        {
            " | AI [ready]".to_string()
        } else {
            " | AI".to_string()
        }
    } else {
        String::new()
    };

    // Add AST indicator when AST merging is available
    let ast_indicator = if app.ast_available() {
        if app
            .current_hunk()
            .is_some_and(|h| app.ast_state().has_suggestion_for(h.id))
        {
            " | AST [ready]".to_string()
        } else {
            " | AST".to_string()
        }
    } else {
        String::new()
    };

    // Build undo/redo indicator
    let undo_redo_indicator = {
        let can_undo = app.can_undo();
        let can_redo = app.can_redo();
        match (can_undo, can_redo) {
            (false, false) => String::new(),
            (true, false) => format!(" | undo({})", app.action_history.undo_count()),
            (false, true) => format!(" | redo({})", app.action_history.redo_count()),
            (true, true) => format!(
                " | undo({}) redo({})",
                app.action_history.undo_count(),
                app.action_history.redo_count()
            ),
        }
    };

    let diff3_indicator = if app.show_base_pane() { " | diff3" } else { "" };

    let full_status =
        format!("{status_text}{undo_redo_indicator}{diff3_indicator}{ai_indicator}{ast_indicator}");
    let status = Paragraph::new(full_status).style(theme.ui.status.bg(theme.base.background));
    frame.render_widget(status, area);
}

/// Builds the full document content for a side pane (left or right).
fn build_side_document<'a>(
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
fn build_base_document<'a>(
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
fn build_result_document<'a>(
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
                return build_highlighted_line(line_number, spans_data, is_highlighted);
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
fn build_word_diff_line(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::ThemeName;
    use ratatui::{backend::TestBackend, Terminal};

    fn create_test_terminal() -> Terminal<TestBackend> {
        let backend = TestBackend::new(80, 24);
        Terminal::new(backend).unwrap()
    }

    #[test]
    fn render_left_pane_without_session() {
        let mut terminal = create_test_terminal();
        let app = App::new();
        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 40, 10);
                render_left_pane(frame, area, &app);
            })
            .unwrap();
    }

    #[test]
    fn render_right_pane_without_session() {
        let mut terminal = create_test_terminal();
        let app = App::new();
        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 40, 10);
                render_right_pane(frame, area, &app);
            })
            .unwrap();
    }

    #[test]
    fn render_result_pane_without_session() {
        let mut terminal = create_test_terminal();
        let app = App::new();
        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 80, 10);
                render_result_pane(frame, area, &app);
            })
            .unwrap();
    }

    #[test]
    fn render_title_bar_shows_no_conflicts() {
        let mut terminal = create_test_terminal();
        let app = App::new();
        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 80, 1);
                render_title_bar(frame, area, &app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let title_line: String = (0..buffer.area.width)
            .map(|x| buffer.cell((x, 0)).unwrap().symbol().to_string())
            .collect();
        assert!(title_line.contains("weavr"));
        assert!(title_line.contains("No conflicts"));
    }

    #[test]
    fn render_status_bar_shows_pane_and_conflicts() {
        let mut terminal = create_test_terminal();
        let app = App::new();
        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 80, 1);
                render_status_bar(frame, area, &app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let status_line: String = (0..buffer.area.width)
            .map(|x| buffer.cell((x, 0)).unwrap().symbol().to_string())
            .collect();
        // New format: "Left pane | No conflicts"
        assert!(status_line.contains("Left pane"));
        assert!(status_line.contains("No conflicts"));
    }

    #[test]
    fn pane_side_titles() {
        assert_eq!(PaneSide::Left.title(), "Left (Ours)");
        assert_eq!(PaneSide::Right.title(), "Right (Theirs)");
    }

    #[test]
    fn pane_side_focused_pane() {
        assert_eq!(PaneSide::Left.focused_pane(), FocusedPane::Left);
        assert_eq!(PaneSide::Right.focused_pane(), FocusedPane::Right);
    }

    #[test]
    fn renders_with_different_themes() {
        let mut terminal = create_test_terminal();

        for theme_name in [ThemeName::Dark, ThemeName::Light, ThemeName::Dracula] {
            let app = App::with_theme(theme_name);
            terminal
                .draw(|frame| {
                    let area = Rect::new(0, 0, 40, 10);
                    render_left_pane(frame, area, &app);
                })
                .unwrap();
        }
    }

    #[test]
    fn word_diff_line_produces_multiple_spans() {
        use crate::diff::DiffLine;
        use crate::theme::Theme;

        let theme = Theme::from(ThemeName::Dark);
        let diff_line = DiffLine::with_counterpart(
            "hello world",
            ChangeTag::Delete,
            "hello universe".to_string(),
        );

        let line = build_word_diff_line(1, &diff_line, PaneSide::Left, &theme, false);

        // Should have: line number span + word spans (more than 2 total)
        assert!(
            line.spans.len() > 2,
            "Expected multiple spans for word diff, got {}",
            line.spans.len()
        );
    }
}
