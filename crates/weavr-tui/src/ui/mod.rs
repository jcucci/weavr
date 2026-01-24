//! UI rendering for the TUI.
//!
//! This module handles all rendering logic using ratatui.

mod layout;
mod pane;

pub use layout::{calculate_layout, PaneAreas};

use ratatui::Frame;

use crate::App;

/// Renders the entire UI to the frame.
pub fn draw(frame: &mut Frame, app: &App) {
    let areas = calculate_layout(frame.area(), app.layout_config());

    // Title bar with hunk counter
    pane::render_title_bar(frame, areas.title_bar, app);

    // Three panes with full document content
    pane::render_left_pane(frame, areas.left_pane, app);
    pane::render_right_pane(frame, areas.right_pane, app);
    pane::render_result_pane(frame, areas.result_pane, app);

    // Status bar with context-sensitive help
    pane::render_status_bar(frame, areas.status_bar, app);
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
        // Title bar should contain "weavr"
        let title_line: String = (0..buffer.area.width)
            .map(|x| buffer.cell((x, 0)).unwrap().symbol().to_string())
            .collect();
        assert!(title_line.contains("weavr"));
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
        // Status bar shows context-sensitive help
        assert!(last_line.contains("quit"));
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
