//! weavr-tui: Terminal User Interface
//!
//! This crate provides the terminal UI for weavr, built on ratatui.
//!
//! Key features:
//! - Three-pane layout (left, right, result)
//! - Keyboard-first navigation
//! - Hunk-based conflict resolution
//! - Theming support
//!
//! The TUI is a thin wrapper around weavr-core. It displays state and
//! captures input but never performs merge logic directly.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use weavr_core::{AcceptBothOptions, ConflictHunk, HunkState, MergeSession, Resolution};

pub mod event;
pub mod theme;
pub mod ui;

/// Configuration for the three-pane layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutConfig {
    /// Percentage of height for top row (left/right panes). Default: 60
    pub top_ratio_percent: u16,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            top_ratio_percent: 60,
        }
    }
}

use theme::{Theme, ThemeName};

/// Which pane currently has focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusedPane {
    /// Left pane (ours).
    #[default]
    Left,
    /// Right pane (theirs).
    Right,
    /// Result pane (merged output).
    Result,
}

/// Application state for the TUI.
pub struct App {
    /// The active merge session.
    session: Option<MergeSession>,
    /// Whether the application should quit.
    should_quit: bool,
    /// Which pane has focus.
    focused_pane: FocusedPane,
    /// The active theme.
    theme: Theme,
    /// Current hunk index (0-based).
    current_hunk_index: usize,
    /// Synchronized scroll offset for left/right panes.
    left_right_scroll: u16,
    /// Independent scroll offset for result pane.
    result_scroll: u16,
    /// Layout configuration.
    layout_config: LayoutConfig,
}

impl App {
    /// Creates a new application instance with the default theme.
    #[must_use]
    pub fn new() -> Self {
        Self {
            session: None,
            should_quit: false,
            focused_pane: FocusedPane::default(),
            theme: Theme::from(ThemeName::default()),
            current_hunk_index: 0,
            left_right_scroll: 0,
            result_scroll: 0,
            layout_config: LayoutConfig::default(),
        }
    }

    /// Creates a new application instance with the specified theme.
    #[must_use]
    pub fn with_theme(theme_name: ThemeName) -> Self {
        Self {
            session: None,
            should_quit: false,
            focused_pane: FocusedPane::default(),
            theme: Theme::from(theme_name),
            current_hunk_index: 0,
            left_right_scroll: 0,
            result_scroll: 0,
            layout_config: LayoutConfig::default(),
        }
    }

    /// Sets the merge session to display.
    pub fn set_session(&mut self, session: MergeSession) {
        self.session = Some(session);
    }

    /// Returns a reference to the current session, if any.
    #[must_use]
    pub fn session(&self) -> Option<&MergeSession> {
        self.session.as_ref()
    }

    /// Returns whether the application should quit.
    #[must_use]
    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    /// Signals the application to quit.
    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    /// Returns the currently focused pane.
    #[must_use]
    pub fn focused_pane(&self) -> FocusedPane {
        self.focused_pane
    }

    /// Cycles focus to the next pane (Left -> Right -> Result -> Left).
    pub fn cycle_focus(&mut self) {
        self.focused_pane = match self.focused_pane {
            FocusedPane::Left => FocusedPane::Right,
            FocusedPane::Right => FocusedPane::Result,
            FocusedPane::Result => FocusedPane::Left,
        };
    }

    /// Cycles focus to the previous pane (Left -> Result -> Right -> Left).
    pub fn cycle_focus_back(&mut self) {
        self.focused_pane = match self.focused_pane {
            FocusedPane::Left => FocusedPane::Result,
            FocusedPane::Right => FocusedPane::Left,
            FocusedPane::Result => FocusedPane::Right,
        };
    }

    /// Returns a reference to the current theme.
    #[must_use]
    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    /// Sets the theme by name.
    pub fn set_theme(&mut self, name: ThemeName) {
        self.theme = Theme::from(name);
    }

    /// Returns a reference to the current hunk, if any.
    #[must_use]
    pub fn current_hunk(&self) -> Option<&ConflictHunk> {
        self.session
            .as_ref()
            .and_then(|s| s.hunks().get(self.current_hunk_index))
    }

    /// Returns the current hunk index (0-based).
    #[must_use]
    pub fn current_hunk_index(&self) -> usize {
        self.current_hunk_index
    }

    /// Returns the total number of hunks.
    #[must_use]
    pub fn total_hunks(&self) -> usize {
        self.session.as_ref().map_or(0, |s| s.hunks().len())
    }

    /// Moves to the next hunk.
    pub fn next_hunk(&mut self) {
        let total = self.total_hunks();
        if total > 0 && self.current_hunk_index < total - 1 {
            self.current_hunk_index += 1;
            self.reset_scroll();
        }
    }

    /// Moves to the previous hunk.
    pub fn prev_hunk(&mut self) {
        if self.current_hunk_index > 0 {
            self.current_hunk_index -= 1;
            self.reset_scroll();
        }
    }

    /// Moves to a specific hunk by index.
    pub fn go_to_hunk(&mut self, index: usize) {
        let total = self.total_hunks();
        if total > 0 && index < total {
            self.current_hunk_index = index;
            self.reset_scroll();
        }
    }

    /// Moves to the next unresolved hunk, wrapping around if necessary.
    pub fn next_unresolved_hunk(&mut self) {
        if let Some(session) = &self.session {
            let hunks = session.hunks();
            let total = hunks.len();
            if total == 0 {
                return;
            }

            // Search forward from current position
            for i in 1..=total {
                let idx = (self.current_hunk_index + i) % total;
                if matches!(hunks[idx].state, HunkState::Unresolved) {
                    self.current_hunk_index = idx;
                    self.reset_scroll();
                    return;
                }
            }
        }
    }

    /// Resolves the current hunk by accepting the left (ours) content.
    pub fn resolve_left(&mut self) {
        let resolution_data = self.session.as_ref().and_then(|session| {
            session
                .hunks()
                .get(self.current_hunk_index)
                .map(|hunk| (hunk.id, Resolution::accept_left(hunk)))
        });

        if let (Some(session), Some((hunk_id, resolution))) =
            (self.session.as_mut(), resolution_data)
        {
            let _ = session.set_resolution(hunk_id, resolution);
        }
    }

    /// Resolves the current hunk by accepting the right (theirs) content.
    pub fn resolve_right(&mut self) {
        let resolution_data = self.session.as_ref().and_then(|session| {
            session
                .hunks()
                .get(self.current_hunk_index)
                .map(|hunk| (hunk.id, Resolution::accept_right(hunk)))
        });

        if let (Some(session), Some((hunk_id, resolution))) =
            (self.session.as_mut(), resolution_data)
        {
            let _ = session.set_resolution(hunk_id, resolution);
        }
    }

    /// Resolves the current hunk by accepting both sides (left then right).
    pub fn resolve_both(&mut self) {
        let resolution_data = self.session.as_ref().and_then(|session| {
            session.hunks().get(self.current_hunk_index).map(|hunk| {
                let options = AcceptBothOptions::default();
                (hunk.id, Resolution::accept_both(hunk, &options))
            })
        });

        if let (Some(session), Some((hunk_id, resolution))) =
            (self.session.as_mut(), resolution_data)
        {
            let _ = session.set_resolution(hunk_id, resolution);
        }
    }

    /// Scrolls up by the specified number of lines.
    pub fn scroll_up(&mut self, lines: u16) {
        match self.focused_pane {
            FocusedPane::Left | FocusedPane::Right => {
                self.left_right_scroll = self.left_right_scroll.saturating_sub(lines);
            }
            FocusedPane::Result => {
                self.result_scroll = self.result_scroll.saturating_sub(lines);
            }
        }
    }

    /// Scrolls down by the specified number of lines.
    pub fn scroll_down(&mut self, lines: u16) {
        match self.focused_pane {
            FocusedPane::Left | FocusedPane::Right => {
                self.left_right_scroll = self.left_right_scroll.saturating_add(lines);
            }
            FocusedPane::Result => {
                self.result_scroll = self.result_scroll.saturating_add(lines);
            }
        }
    }

    /// Returns the scroll offset for left/right panes.
    #[must_use]
    pub fn left_right_scroll(&self) -> u16 {
        self.left_right_scroll
    }

    /// Returns the scroll offset for the result pane.
    #[must_use]
    pub fn result_scroll(&self) -> u16 {
        self.result_scroll
    }

    /// Returns a reference to the layout configuration.
    #[must_use]
    pub fn layout_config(&self) -> &LayoutConfig {
        &self.layout_config
    }

    /// Resets scroll positions when changing hunks.
    fn reset_scroll(&mut self) {
        self.left_right_scroll = 0;
        self.result_scroll = 0;
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl FocusedPane {
    /// Returns the display title for this pane.
    #[must_use]
    pub fn title(self) -> &'static str {
        match self {
            Self::Left => "Left (Ours)",
            Self::Right => "Right (Theirs)",
            Self::Result => "Result",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_creation() {
        let app = App::new();
        assert!(!app.should_quit());
        assert!(app.session().is_none());
    }

    #[test]
    fn app_default() {
        let app = App::default();
        assert!(!app.should_quit());
    }

    #[test]
    fn app_quit() {
        let mut app = App::new();
        assert!(!app.should_quit());
        app.quit();
        assert!(app.should_quit());
    }

    #[test]
    fn app_set_session() {
        use std::path::PathBuf;

        let mut app = App::new();
        assert!(app.session().is_none());

        let input = weavr_core::MergeInput {
            left: weavr_core::FileVersion {
                path: PathBuf::from("test.rs"),
                content: String::from("left"),
            },
            right: weavr_core::FileVersion {
                path: PathBuf::from("test.rs"),
                content: String::from("right"),
            },
            base: None,
        };
        let session = weavr_core::MergeSession::new(input).unwrap();
        app.set_session(session);

        assert!(app.session().is_some());
    }

    #[test]
    fn focused_pane_default_is_left() {
        let app = App::new();
        assert_eq!(app.focused_pane(), FocusedPane::Left);
    }

    #[test]
    fn cycle_focus_forward() {
        let mut app = App::new();
        assert_eq!(app.focused_pane(), FocusedPane::Left);

        app.cycle_focus();
        assert_eq!(app.focused_pane(), FocusedPane::Right);

        app.cycle_focus();
        assert_eq!(app.focused_pane(), FocusedPane::Result);

        app.cycle_focus();
        assert_eq!(app.focused_pane(), FocusedPane::Left);
    }

    #[test]
    fn cycle_focus_backward() {
        let mut app = App::new();
        assert_eq!(app.focused_pane(), FocusedPane::Left);

        app.cycle_focus_back();
        assert_eq!(app.focused_pane(), FocusedPane::Result);

        app.cycle_focus_back();
        assert_eq!(app.focused_pane(), FocusedPane::Right);

        app.cycle_focus_back();
        assert_eq!(app.focused_pane(), FocusedPane::Left);
    }

    #[test]
    fn focused_pane_titles() {
        assert_eq!(FocusedPane::Left.title(), "Left (Ours)");
        assert_eq!(FocusedPane::Right.title(), "Right (Theirs)");
        assert_eq!(FocusedPane::Result.title(), "Result");
    }

    #[test]
    fn app_with_theme() {
        let app = App::with_theme(ThemeName::Light);
        // Verify theme is set by checking a known color
        assert_eq!(
            app.theme().base.background,
            ratatui::style::Color::Rgb(250, 250, 250)
        );
    }

    #[test]
    fn app_set_theme() {
        let mut app = App::new();
        app.set_theme(ThemeName::Dracula);
        // Dracula background is Rgb(40, 42, 54)
        assert_eq!(
            app.theme().base.background,
            ratatui::style::Color::Rgb(40, 42, 54)
        );
    }

    #[test]
    fn layout_config_default() {
        let config = LayoutConfig::default();
        assert_eq!(config.top_ratio_percent, 60);
    }

    #[test]
    fn app_initial_hunk_state() {
        let app = App::new();
        assert_eq!(app.current_hunk_index(), 0);
        assert_eq!(app.total_hunks(), 0);
        assert!(app.current_hunk().is_none());
    }

    #[test]
    fn app_hunk_navigation_without_session() {
        let mut app = App::new();
        // Should not panic with no session
        app.next_hunk();
        app.prev_hunk();
        app.go_to_hunk(5);
        app.next_unresolved_hunk();
        assert_eq!(app.current_hunk_index(), 0);
    }

    #[test]
    fn app_scroll_state() {
        let mut app = App::new();
        assert_eq!(app.left_right_scroll(), 0);
        assert_eq!(app.result_scroll(), 0);

        // Left pane focused by default, scroll affects left_right
        app.scroll_down(5);
        assert_eq!(app.left_right_scroll(), 5);
        assert_eq!(app.result_scroll(), 0);

        app.scroll_up(2);
        assert_eq!(app.left_right_scroll(), 3);

        // Switch to result pane
        app.cycle_focus();
        app.cycle_focus(); // Now on Result
        app.scroll_down(10);
        assert_eq!(app.left_right_scroll(), 3);
        assert_eq!(app.result_scroll(), 10);
    }

    #[test]
    fn app_scroll_saturates() {
        let mut app = App::new();
        // Scroll up from 0 should stay at 0
        app.scroll_up(100);
        assert_eq!(app.left_right_scroll(), 0);
    }
}
