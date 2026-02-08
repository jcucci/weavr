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
use crate::diff::{compute_line_diffs, DiffConfig};
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

    let content = match app.session() {
        Some(session) => build_side_document(
            session.segments(),
            session.hunks(),
            side,
            app.current_hunk_index(),
            theme,
            *app.diff_config(),
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

/// Renders the result pane showing the merged output.
pub fn render_result_pane(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme();
    let is_focused = app.focused_pane() == FocusedPane::Result;

    let border_style = if is_focused {
        Style::default().fg(theme.ui.border_focused)
    } else {
        Style::default().fg(theme.ui.border_unfocused)
    };

    let content = match app.session() {
        Some(session) => build_result_document(
            session.segments(),
            session.hunks(),
            app.current_hunk_index(),
            theme,
            app.ai_state(),
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

    let title = Line::from(vec![
        Span::styled(" weavr ", theme.ui.title),
        Span::raw("| "),
        Span::styled(hunk_info, Style::default().fg(theme.base.accent)),
    ]);

    let paragraph = Paragraph::new(title).style(theme.ui.title.bg(theme.base.background));
    frame.render_widget(paragraph, area);
}

/// Duration before status messages auto-clear.
const STATUS_MESSAGE_DURATION: Duration = Duration::from_secs(3);

/// Renders the status bar with context-sensitive help.
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
        FocusedPane::Right => "Right",
        FocusedPane::Result => "Result",
    };

    // Format: "Hunk 2/5 | Left pane | 3 unresolved"
    let status_text = if app.total_hunks() > 0 {
        format!(
            " Hunk {}/{} | {} pane | {} unresolved",
            app.current_hunk_index() + 1,
            app.total_hunks(),
            pane_name,
            unresolved_count
        )
    } else {
        format!(" {pane_name} pane | No conflicts")
    };

    // Add AI indicator when AI is available
    let ai_indicator = if app.ai_available() {
        if app.ai_state().is_loading() {
            format!(" | AI {}", app.ai_state().spinner_char())
        } else if app.ai_state().suggestion.is_some() {
            " | AI [ready]".to_string()
        } else {
            " | AI".to_string()
        }
    } else {
        String::new()
    };

    let full_status = format!("{status_text}{ai_indicator}");
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
    _diff_config: DiffConfig,
) -> Vec<Line<'a>> {
    let mut lines = Vec::new();
    let mut line_number = 1;

    for segment in segments {
        match segment {
            Segment::Clean(text) => {
                for line_text in text.lines() {
                    lines.push(build_line(
                        line_number,
                        line_text,
                        Style::default().fg(theme.base.foreground),
                        false,
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
                    // Apply style based on diff tag
                    let style = match diff_line.tag {
                        ChangeTag::Equal => theme.diff.context,
                        ChangeTag::Delete => theme.diff.removed,
                        ChangeTag::Insert => theme.diff.added,
                    };

                    lines.push(build_line(line_number, &diff_line.text, style, is_current));
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

/// Builds the full document content for the result pane.
#[allow(clippy::too_many_lines)]
fn build_result_document<'a>(
    segments: &[Segment],
    hunks: &[weavr_core::ConflictHunk],
    current_hunk_idx: usize,
    theme: &'a crate::theme::Theme,
    ai_state: &AiState,
) -> Vec<Line<'a>> {
    let mut lines = Vec::new();
    let mut line_number = 1;

    for segment in segments {
        match segment {
            Segment::Clean(text) => {
                for line_text in text.lines() {
                    lines.push(build_line(
                        line_number,
                        line_text,
                        Style::default().fg(theme.base.foreground),
                        false,
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
                } else if is_current && ai_state.has_suggestion_for(hunk.id) {
                    // AI suggestion ghost text
                    let suggestion = ai_state.suggestion.as_ref().unwrap();
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
                        lines.push(build_line(line_number, line_text, ghost_style, true));
                        line_number += 1;
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
}
