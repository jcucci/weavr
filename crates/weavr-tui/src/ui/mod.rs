//! UI rendering for the TUI.
//!
//! This module handles all rendering logic using ratatui.

mod document;
mod layout;
mod overlay;
mod pane;

pub use layout::{calculate_layout, PaneAreas};

use crate::FocusedPane;
use ratatui::Frame;

use crate::input::Dialog;
use crate::App;

/// Which side of the conflict to render.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PaneSide {
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

/// Renders the entire UI to the frame.
pub fn draw(frame: &mut Frame, app: &App) {
    let areas = calculate_layout(frame.area(), app.layout_config(), app.show_base_pane());

    // Title bar with hunk counter
    pane::render_title_bar(frame, areas.title_bar, app);

    // Side panes with full document content
    pane::render_left_pane(frame, areas.left_pane, app);
    if let Some(base_area) = areas.base_pane {
        pane::render_base_pane(frame, base_area, app);
    }
    pane::render_right_pane(frame, areas.right_pane, app);
    pane::render_result_pane(frame, areas.result_pane, app);

    // Status bar with context-sensitive help
    pane::render_status_bar(frame, areas.status_bar, app);

    // Render overlay dialogs on top
    if let Some(dialog) = app.active_dialog() {
        match dialog {
            Dialog::Help(ref state) => {
                overlay::render_help_overlay(
                    frame,
                    frame.area(),
                    app.theme(),
                    state,
                    app.help_sections(),
                );
            }
            Dialog::AcceptBothOptions(state) => {
                overlay::render_accept_both_dialog(frame, frame.area(), app.theme(), state);
            }
            Dialog::AiExplanation(ref text) => {
                overlay::render_ai_explanation_overlay(frame, frame.area(), app.theme(), text);
            }
            Dialog::StagingPrompt => {
                overlay::render_staging_prompt_dialog(frame, frame.area(), app.theme());
            }
            Dialog::FileList(ref state) => {
                if let Some(ref ws) = app.workspace {
                    overlay::render_file_list_overlay(frame, frame.area(), app.theme(), state, ws);
                }
            }
        }
    }
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
        // Status bar shows pane info
        assert!(last_line.contains("pane"));
    }

    #[test]
    fn draw_with_workspace_does_not_panic() {
        use crate::workspace::{FileState, Workspace};
        use std::path::PathBuf;

        let mut terminal = create_test_terminal();
        let content = "<<<<<<< HEAD\nleft\n=======\nright\n>>>>>>> branch\n";
        let session1 =
            weavr_core::MergeSession::from_conflicted(content, PathBuf::from("a.rs")).unwrap();
        let session2 =
            weavr_core::MergeSession::from_conflicted(content, PathBuf::from("b.rs")).unwrap();

        let mut app = App::new();
        let ws = Workspace::new(vec![
            FileState::new(PathBuf::from("a.rs"), session1),
            FileState::new(PathBuf::from("b.rs"), session2),
        ]);
        app.set_workspace(ws);

        terminal.draw(|frame| draw(frame, &app)).unwrap();

        // Title bar should show file position
        let buffer = terminal.backend().buffer();
        let title_line: String = (0..buffer.area.width)
            .map(|x| buffer.cell((x, 0)).unwrap().symbol().to_string())
            .collect();
        assert!(title_line.contains("[1/2]"));
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
