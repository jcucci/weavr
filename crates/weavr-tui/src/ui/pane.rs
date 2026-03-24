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
use weavr_core::HunkState;

use crate::input::InputMode;
use crate::{App, FocusedPane};

use super::document::{build_base_document, build_result_document, build_side_document};

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
#[allow(clippy::too_many_lines)]
pub fn render_result_pane(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme();
    let is_focused = app.focused_pane() == FocusedPane::Result;

    // Edit mode: render the edit buffer instead of normal content
    if let Some(edit_state) = app.edit_state() {
        let border_style = Style::default()
            .fg(theme.base.accent)
            .add_modifier(Modifier::BOLD);

        let content: Vec<Line<'_>> = edit_state
            .lines
            .iter()
            .enumerate()
            .map(|(i, line_text)| {
                let line_num = i + 1;
                let is_cursor_line = i == edit_state.cursor_row;
                let line_num_style = if is_cursor_line {
                    Style::default()
                        .fg(ratatui::style::Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().add_modifier(Modifier::DIM)
                };

                Line::from(vec![
                    Span::styled(format!("{line_num:4} "), line_num_style),
                    Span::styled(
                        line_text.as_str(),
                        Style::default().fg(theme.base.foreground),
                    ),
                ])
            })
            .collect();

        let title = match edit_state.sub_mode {
            crate::input::EditSubMode::Insert => " Result [INSERT] ",
            crate::input::EditSubMode::Normal => " Result [EDIT] ",
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(border_style)
            .title(title);

        // Compute scroll from cursor position and viewport height
        // (borders consume 2 rows)
        let viewport_height = area.height.saturating_sub(2) as usize;
        let scroll_offset = edit_state.scroll_offset;
        let scroll_y = if viewport_height > 0 {
            if edit_state.cursor_row < scroll_offset {
                edit_state.cursor_row
            } else if edit_state.cursor_row >= scroll_offset + viewport_height {
                edit_state.cursor_row - viewport_height + 1
            } else {
                scroll_offset
            }
        } else {
            0
        };
        #[allow(clippy::cast_possible_truncation)]
        let scroll_y_u16 = scroll_y as u16;
        let paragraph = Paragraph::new(content)
            .block(block)
            .scroll((scroll_y_u16, 0));

        frame.render_widget(paragraph, area);

        // Set cursor position within the pane
        // Account for border (1) and line number prefix (5 chars: "   1 ")
        #[allow(clippy::cast_possible_truncation)]
        let cursor_x = area.x + 1 + 5 + edit_state.cursor_col as u16;
        #[allow(clippy::cast_possible_truncation)]
        let cursor_y = area.y + 1 + (edit_state.cursor_row as u16).saturating_sub(scroll_y_u16);

        if cursor_x < area.x + area.width && cursor_y < area.y + area.height {
            frame.set_cursor_position((cursor_x, cursor_y));
        }

        return;
    }

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

    // Edit mode: show submode-aware indicator
    if app.input_mode() == InputMode::Edit {
        let edit_status = if let Some(edit_state) = app.edit_state() {
            match edit_state.sub_mode {
                crate::input::EditSubMode::Insert => {
                    " -- INSERT -- (Esc: normal | Ctrl+C: discard)".to_string()
                }
                crate::input::EditSubMode::Normal => {
                    " -- EDIT -- (Esc/q: apply | i: insert | Ctrl+C: discard)".to_string()
                }
            }
        } else {
            String::new()
        };
        let status = Paragraph::new(edit_status).style(
            Style::default()
                .fg(theme.base.accent)
                .bg(theme.base.background),
        );
        frame.render_widget(status, area);
        return;
    }

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
        use super::super::document::build_word_diff_line;
        use crate::diff::DiffLine;
        use crate::theme::Theme;
        use similar::ChangeTag;

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
