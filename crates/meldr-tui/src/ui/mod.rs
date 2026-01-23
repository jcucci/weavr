//! UI rendering for the TUI.
//!
//! This module handles all rendering logic using ratatui.

mod layout;

pub use layout::{calculate_layout, PaneAreas};

use ratatui::{
    style::Style,
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::{App, FocusedPane};

/// Returns the border style for a pane based on focus state.
fn pane_border_style(app: &App, pane: FocusedPane) -> Style {
    if app.focused_pane() == pane {
        Style::default().fg(app.theme().ui.border_focused)
    } else {
        Style::default().fg(app.theme().ui.border_unfocused)
    }
}

/// Renders the entire UI to the frame.
pub fn draw(frame: &mut Frame, app: &App) {
    let areas = calculate_layout(frame.area());
    let theme = app.theme();

    // Title bar - combine theme style with base background
    let title_style = theme.ui.title.bg(theme.base.background);
    let title = Paragraph::new(" meldr").style(title_style);
    frame.render_widget(title, areas.title_bar);

    // Pane border styles based on focus
    let left_style = pane_border_style(app, FocusedPane::Left);
    let right_style = pane_border_style(app, FocusedPane::Right);
    let result_style = pane_border_style(app, FocusedPane::Result);

    // Left pane
    let left_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(left_style)
        .title(FocusedPane::Left.title())
        .title_style(left_style);
    frame.render_widget(left_block, areas.left_pane);

    // Right pane
    let right_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(right_style)
        .title(FocusedPane::Right.title())
        .title_style(right_style);
    frame.render_widget(right_block, areas.right_pane);

    // Result pane
    let result_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(result_style)
        .title(FocusedPane::Result.title())
        .title_style(result_style);
    frame.render_widget(result_block, areas.result_pane);

    // Status bar - combine theme style with base background
    let status_style = theme.ui.status.bg(theme.base.background);
    let status = Paragraph::new(" Press q to quit | Tab to cycle focus").style(status_style);
    frame.render_widget(status, areas.status_bar);
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
        let app = App::new();
        let style = pane_border_style(&app, FocusedPane::Left);
        assert_eq!(style.fg, Some(app.theme().ui.border_focused));
    }

    #[test]
    fn pane_border_style_returns_unfocused_style() {
        let app = App::new();
        let style = pane_border_style(&app, FocusedPane::Right);
        assert_eq!(style.fg, Some(app.theme().ui.border_unfocused));
    }

    #[test]
    fn draw_with_different_themes() {
        let mut terminal = create_test_terminal();

        // Test with dark theme (default)
        let app_dark = App::new();
        terminal.draw(|frame| draw(frame, &app_dark)).unwrap();

        // Test with light theme
        let app_light = App::with_theme(ThemeName::Light);
        terminal.draw(|frame| draw(frame, &app_light)).unwrap();

        // Test with Catppuccin Mocha
        let app_mocha = App::with_theme(ThemeName::CatppuccinMocha);
        terminal.draw(|frame| draw(frame, &app_mocha)).unwrap();
    }
}
