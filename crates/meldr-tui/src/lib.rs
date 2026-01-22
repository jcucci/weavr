//! meldr-tui: Terminal User Interface
//!
//! This crate provides the terminal UI for meldr, built on ratatui.
//!
//! Key features:
//! - Three-pane layout (left, right, result)
//! - Keyboard-first navigation
//! - Hunk-based conflict resolution
//! - Theming support
//!
//! The TUI is a thin wrapper around meldr-core. It displays state and
//! captures input but never performs merge logic directly.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use meldr_core::MergeSession;

pub mod event;
pub mod ui;

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
}

impl App {
    /// Creates a new application instance.
    #[must_use]
    pub fn new() -> Self {
        Self {
            session: None,
            should_quit: false,
            focused_pane: FocusedPane::default(),
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

        let input = meldr_core::MergeInput {
            left: meldr_core::FileVersion {
                path: PathBuf::from("test.rs"),
                content: String::from("left"),
            },
            right: meldr_core::FileVersion {
                path: PathBuf::from("test.rs"),
                content: String::from("right"),
            },
            base: None,
        };
        let session = meldr_core::MergeSession::new(input).unwrap();
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
}
