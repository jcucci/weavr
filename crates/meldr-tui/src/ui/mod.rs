//! UI rendering for the TUI.
//!
//! This module handles all rendering logic using ratatui.

mod layout;

pub use layout::{calculate_layout, PaneAreas};

use ratatui::{
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::{App, FocusedPane};

/// Renders the entire UI to the frame.
pub fn draw(frame: &mut Frame, app: &App) {
    let areas = calculate_layout(frame.area());

    // Title bar
    let title = Paragraph::new(" meldr").style(Style::default().fg(Color::Cyan));
    frame.render_widget(title, areas.title_bar);

    // Pane border styles based on focus
    let focused_style = Style::default().fg(Color::Yellow);
    let unfocused_style = Style::default().fg(Color::DarkGray);

    let left_style = if app.focused_pane() == FocusedPane::Left {
        focused_style
    } else {
        unfocused_style
    };
    let right_style = if app.focused_pane() == FocusedPane::Right {
        focused_style
    } else {
        unfocused_style
    };
    let result_style = if app.focused_pane() == FocusedPane::Result {
        focused_style
    } else {
        unfocused_style
    };

    // Left pane
    let left_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(left_style)
        .title(FocusedPane::Left.title());
    frame.render_widget(left_block, areas.left_pane);

    // Right pane
    let right_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(right_style)
        .title(FocusedPane::Right.title());
    frame.render_widget(right_block, areas.right_pane);

    // Result pane
    let result_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(result_style)
        .title(FocusedPane::Result.title());
    frame.render_widget(result_block, areas.result_pane);

    // Status bar
    let status = Paragraph::new(" Press q to quit | Tab to cycle focus")
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(status, areas.status_bar);
}
