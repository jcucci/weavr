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

use crate::help::{default_help_sections, help_line_count};
use crate::input::{AcceptBothOptionsState, HelpState};
use crate::theme::Theme;
use weavr_core::BothOrder;

/// Renders a centered, scrollable help overlay showing keybindings.
pub fn render_help_overlay(frame: &mut Frame, area: Rect, theme: &Theme, state: &HelpState) {
    let dialog_area = centered_rect(60, 70, area);

    // Clear the background
    frame.render_widget(Clear, dialog_area);

    // Build lines from structured help data
    let sections = default_help_sections();
    let mut help_lines: Vec<Line<'_>> = Vec::new();

    for (i, section) in sections.iter().enumerate() {
        if i > 0 {
            help_lines.push(Line::from(""));
        }
        help_lines.push(Line::from(Span::styled(
            format!("=== {} ===", section.title),
            Style::default().add_modifier(Modifier::BOLD),
        )));
        for binding in section.bindings {
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
    let total_lines = help_line_count();
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
