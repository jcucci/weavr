//! Overlay and dialog rendering.
//!
//! This module provides centered overlay dialogs for help, confirmations,
//! and other modal interactions.

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::help::{self, HelpSection};
use crate::input::{AcceptBothOptionsState, FileListMode, FileListState, HelpState};
use crate::theme::Theme;
use crate::workspace::{FileStatus, Workspace};
use weavr_core::BothOrder;

/// Renders a centered, scrollable help overlay showing keybindings.
pub fn render_help_overlay(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    state: &HelpState,
    sections: &[HelpSection],
) {
    let dialog_area = centered_rect(60, 70, area);

    // Clear the background
    frame.render_widget(Clear, dialog_area);

    // Build lines from structured help data
    let mut help_lines: Vec<Line<'_>> = Vec::new();

    for (i, section) in sections.iter().enumerate() {
        if i > 0 {
            help_lines.push(Line::from(""));
        }
        help_lines.push(Line::from(Span::styled(
            format!("=== {} ===", section.title),
            Style::default().add_modifier(Modifier::BOLD),
        )));
        for binding in &section.bindings {
            help_lines.push(Line::from(format!(
                "  {:<10}{}",
                binding.key, binding.description
            )));
        }
    }

    help_lines.push(Line::from(""));
    help_lines.push(Line::from(Span::styled(
        "Press ?, q, or Esc to close · j/k to scroll",
        Style::default().fg(theme.base.muted),
    )));

    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.ui.border_focused))
        .style(Style::default().bg(theme.base.background));

    // Clamp scroll so the user can't scroll past the content.
    // Inner height = dialog height minus 2 for top/bottom borders.
    let visible_height = dialog_area.height.saturating_sub(2) as usize;
    let total_lines = help::help_line_count(sections);
    let max_scroll = total_lines.saturating_sub(visible_height);
    let clamped_scroll = state.scroll.min(max_scroll);

    let paragraph = Paragraph::new(help_lines)
        .block(block)
        .scroll((u16::try_from(clamped_scroll).unwrap_or(u16::MAX), 0))
        .style(Style::default().fg(theme.base.foreground));

    frame.render_widget(paragraph, dialog_area);
}

/// Renders the `AcceptBoth` options dialog.
pub fn render_accept_both_dialog(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    state: &AcceptBothOptionsState,
) {
    let dialog_area = centered_rect(50, 40, area);

    // Clear the background
    frame.render_widget(Clear, dialog_area);

    let order_left = if state.order == BothOrder::LeftThenRight {
        "[L]eft first"
    } else {
        " Left first "
    };
    let order_right = if state.order == BothOrder::RightThenLeft {
        "[R]ight first"
    } else {
        " Right first "
    };
    let dedupe_check = if state.deduplicate { "[x]" } else { "[ ]" };

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  Order: "),
            Span::styled(
                order_left,
                if state.order == BothOrder::LeftThenRight {
                    Style::default()
                        .fg(theme.ui.border_focused)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.base.muted)
                },
            ),
            Span::raw("  "),
            Span::styled(
                order_right,
                if state.order == BothOrder::RightThenLeft {
                    Style::default()
                        .fg(theme.ui.border_focused)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.base.muted)
                },
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  Deduplicate: "),
            Span::styled(
                dedupe_check,
                if state.deduplicate {
                    theme.diff.added
                } else {
                    Style::default().fg(theme.base.muted)
                },
            ),
            Span::raw(" enabled"),
        ]),
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(
            "  [L]/[R] toggle order   [Space] toggle dedupe",
            Style::default().fg(theme.base.muted),
        )),
        Line::from(Span::styled(
            "  [Enter] confirm        [Esc] cancel",
            Style::default().fg(theme.base.muted),
        )),
    ];

    let block = Block::default()
        .title(" Accept Both Options ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.ui.border_focused))
        .style(Style::default().bg(theme.base.background));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .style(Style::default().fg(theme.base.foreground));

    frame.render_widget(paragraph, dialog_area);
}

/// Renders an AI explanation overlay.
pub fn render_ai_explanation_overlay(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    explanation: &str,
) {
    let dialog_area = centered_rect(70, 60, area);

    // Clear the background
    frame.render_widget(Clear, dialog_area);

    let mut lines = vec![
        Line::from(Span::styled(
            "=== AI Conflict Explanation ===",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    for paragraph in explanation.lines() {
        lines.push(Line::from(paragraph.to_string()));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Press Esc, q, or ? to close",
        Style::default().fg(theme.base.muted),
    )));

    let block = Block::default()
        .title(" AI Explanation ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.ui.border_focused))
        .style(Style::default().bg(theme.base.background));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(theme.base.foreground));

    frame.render_widget(paragraph, dialog_area);
}

/// Renders the staging prompt dialog.
pub fn render_staging_prompt_dialog(frame: &mut Frame, area: Rect, theme: &Theme) {
    let dialog_area = centered_rect(35, 25, area);

    // Clear the background
    frame.render_widget(Clear, dialog_area);

    let lines = vec![
        Line::from(""),
        Line::from("  Stage resolved file?"),
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "[y]",
                Style::default()
                    .fg(theme.ui.border_focused)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Yes   "),
            Span::styled(
                "[n]",
                Style::default()
                    .fg(theme.ui.border_focused)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" No"),
        ]),
    ];

    let block = Block::default()
        .title(" Stage ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.ui.border_focused))
        .style(Style::default().bg(theme.base.background));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .style(Style::default().fg(theme.base.foreground));

    frame.render_widget(paragraph, dialog_area);
}

/// Builds the search bar line for the file list dialog.
fn file_list_search_line(state: &FileListState, theme: &Theme) -> Line<'static> {
    match state.mode {
        FileListMode::Navigate if state.search_query.is_empty() => Line::from(Span::styled(
            "  / to search",
            Style::default().fg(theme.base.muted),
        )),
        FileListMode::Navigate => Line::from(vec![
            Span::styled("  /", Style::default().fg(theme.base.muted)),
            Span::styled(
                state.search_query.clone(),
                Style::default().fg(theme.base.foreground),
            ),
        ]),
        FileListMode::Search => Line::from(vec![
            Span::styled("  /", Style::default().fg(theme.base.accent)),
            Span::styled(
                state.search_query.clone(),
                Style::default().fg(theme.base.foreground),
            ),
            Span::styled("_", Style::default().fg(theme.base.accent)),
        ]),
    }
}

/// Builds lines for the file entries in the file list dialog.
fn file_list_entry_lines<'a>(
    state: &FileListState,
    workspace: &Workspace,
    theme: &'a Theme,
) -> Vec<Line<'a>> {
    let files = workspace.files();
    let current_idx = workspace.current_index();

    if state.filtered_indices.is_empty() {
        return vec![Line::from(Span::styled(
            "  (no matching files)",
            Style::default().fg(theme.base.muted),
        ))];
    }

    state
        .filtered_indices
        .iter()
        .enumerate()
        .map(|(display_row, &ws_idx)| {
            let file = &files[ws_idx];
            let is_selected = display_row == state.selected_index;
            let is_current = ws_idx == current_idx;

            let icon = match (is_current, file.status(), file.written) {
                (true, _, _) => ">",
                (_, FileStatus::NotStarted, _) => " ",
                (_, FileStatus::InProgress, _) => "~",
                (_, FileStatus::FullyResolved, true) => "+",
                (_, FileStatus::FullyResolved, false) => "*",
            };

            let (resolved, total) = file.resolution_counts();
            let display_path = file.path.to_string_lossy();
            let text = format!(
                "  {icon} {:2}. {display_path}  ({resolved}/{total} resolved)",
                ws_idx + 1
            );

            let style = if is_selected {
                Style::default()
                    .fg(theme.base.background)
                    .bg(theme.base.accent)
                    .add_modifier(Modifier::BOLD)
            } else if file.written {
                Style::default().fg(theme.base.muted)
            } else if is_current {
                Style::default()
                    .fg(theme.base.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.base.foreground)
            };

            Line::from(Span::styled(text, style))
        })
        .collect()
}

/// Renders the file list overlay for multi-file navigation.
pub fn render_file_list_overlay(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    state: &FileListState,
    workspace: &Workspace,
) {
    let dialog_area = centered_rect(50, 60, area);
    frame.render_widget(Clear, dialog_area);

    let total_files = workspace.files().len();
    let mut lines: Vec<Line<'_>> = Vec::new();

    lines.push(file_list_search_line(state, theme));
    lines.push(Line::from(Span::styled(
        format!(
            "  [sort: {}] [filter: {}]",
            state.sort.label(),
            state.filter.label()
        ),
        Style::default().fg(theme.base.muted),
    )));
    lines.push(Line::from(""));
    lines.extend(file_list_entry_lines(state, workspace, theme));
    lines.push(Line::from(""));
    let footer = match state.mode {
        FileListMode::Navigate => {
            "  [j/k] Navigate  [/] Search  [s] Sort  [f] Filter  [Enter] Open  [q] Back"
        }
        FileListMode::Search => "  Type to filter  [Esc] Stop search  [Up/Down] Navigate",
    };
    lines.push(Line::from(Span::styled(
        footer,
        Style::default().fg(theme.base.muted),
    )));

    let title = if state.filtered_indices.len() < total_files {
        format!(" Files ({}/{}) ", state.filtered_indices.len(), total_files)
    } else {
        " Files ".to_string()
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.ui.border_focused))
        .style(Style::default().bg(theme.base.background));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .style(Style::default().fg(theme.base.foreground));

    frame.render_widget(paragraph, dialog_area);
}

/// Creates a centered rectangle with the given percentage of the parent area.
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(area);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(vertical[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centered_rect_produces_smaller_area() {
        let parent = Rect::new(0, 0, 100, 50);
        let centered = centered_rect(50, 50, parent);

        assert!(centered.width < parent.width);
        assert!(centered.height < parent.height);
        assert!(centered.x > 0);
        assert!(centered.y > 0);
    }
}
