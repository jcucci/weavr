//! Pane content rendering for the three-pane merge view.
//!
//! This module handles rendering the full document with conflicts highlighted
//! in the left, right, and result panes.

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};
use weavr_core::{HunkState, Segment};

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

/// Renders the status bar with context-sensitive help.
pub fn render_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme();

    let help_text = match app.focused_pane() {
        FocusedPane::Left | FocusedPane::Right => {
            " j/k: nav | l/r/b: resolve | n: next | Tab: pane | q: quit"
        }
        FocusedPane::Result => " l/r/b: resolve | Tab: pane | q: quit",
    };

    let status = Paragraph::new(help_text).style(theme.ui.status.bg(theme.base.background));
    frame.render_widget(status, area);
}

/// Builds the full document content for a side pane (left or right).
fn build_side_document<'a>(
    segments: &[Segment],
    hunks: &[weavr_core::ConflictHunk],
    side: PaneSide,
    current_hunk_idx: usize,
    theme: &'a crate::theme::Theme,
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

                let conflict_text = match side {
                    PaneSide::Left => &hunk.left.text,
                    PaneSide::Right => &hunk.right.text,
                };

                let style = match side {
                    PaneSide::Left => theme.conflict.left,
                    PaneSide::Right => theme.conflict.right,
                };

                // Add marker for conflict start
                if is_current {
                    lines.push(Line::from(Span::styled(
                        format!("──── Conflict {} ────", hunk_idx + 1),
                        style.add_modifier(Modifier::BOLD),
                    )));
                }

                for line_text in conflict_text.lines() {
                    lines.push(build_line(line_number, line_text, style, is_current));
                    line_number += 1;
                }

                if is_current {
                    lines.push(Line::from(Span::styled(
                        "────────────────────",
                        style.add_modifier(Modifier::BOLD),
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
fn build_result_document<'a>(
    segments: &[Segment],
    hunks: &[weavr_core::ConflictHunk],
    current_hunk_idx: usize,
    theme: &'a crate::theme::Theme,
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
                        "  Select: [l]eft  [r]ight  [b]oth",
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
    fn render_status_bar_shows_help() {
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
        assert!(status_line.contains("j/k"));
        assert!(status_line.contains("quit"));
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
