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

/// Returns the border style for a pane based on focus state.
fn pane_border_style(current_focus: FocusedPane, pane: FocusedPane) -> Style {
    if current_focus == pane {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

/// Renders the entire UI to the frame.
pub fn draw(frame: &mut Frame, app: &App) {
    let areas = calculate_layout(frame.area());

    // Title bar
    let title = Paragraph::new(" meldr").style(Style::default().fg(Color::Cyan));
    frame.render_widget(title, areas.title_bar);

    // Pane border styles based on focus
    let left_style = pane_border_style(app.focused_pane(), FocusedPane::Left);
    let right_style = pane_border_style(app.focused_pane(), FocusedPane::Right);
    let result_style = pane_border_style(app.focused_pane(), FocusedPane::Result);

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

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    fn create_test_terminal() -> Terminal<TestBackend> {
        let backend = TestBackend::new(80, 24);
        Terminal::new(backend).unwrap()
    }

    #[test]
    fn draw_renders_without_panic() {
        let mut terminal = create_test_terminal();
        let app = App::new();
        terminal.draw(|frame| draw(frame, &app)).unwrap();
    }

    #[test]
    fn draw_shows_title_bar() {
        let mut terminal = create_test_terminal();
        let app = App::new();
        terminal.draw(|frame| draw(frame, &app)).unwrap();
        let buffer = terminal.backend().buffer();
        // Title bar should contain "meldr"
        let title_line: String = (0..buffer.area.width)
            .map(|x| buffer.cell((x, 0)).unwrap().symbol().to_string())
            .collect();
        assert!(title_line.contains("meldr"));
    }

    #[test]
    fn draw_shows_status_bar() {
        let mut terminal = create_test_terminal();
        let app = App::new();
        terminal.draw(|frame| draw(frame, &app)).unwrap();
        let buffer = terminal.backend().buffer();
        let last_line: String = (0..buffer.area.width)
            .map(|x| buffer.cell((x, 23)).unwrap().symbol().to_string())
            .collect();
        assert!(last_line.contains("Press q to quit"));
    }

    #[test]
    fn pane_border_style_returns_focused_style() {
        let style = pane_border_style(FocusedPane::Left, FocusedPane::Left);
        assert_eq!(style.fg, Some(Color::Yellow));
    }

    #[test]
    fn pane_border_style_returns_unfocused_style() {
        let style = pane_border_style(FocusedPane::Left, FocusedPane::Right);
        assert_eq!(style.fg, Some(Color::DarkGray));
    }
}
