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

use std::time::{Duration, Instant};

use weavr_core::{ConflictHunk, MergeSession};

/// Timeout for multi-key sequences like 'gg'.
const KEY_SEQUENCE_TIMEOUT: Duration = Duration::from_millis(500);

pub mod dialog;
pub mod diff;
pub mod editor;
pub mod event;
pub mod input;
pub mod navigation;
pub mod resolution;
pub mod theme;
pub mod ui;
pub mod undo;

use input::{Command, Dialog, InputMode, KeySequence};
use undo::UndoStack;

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
    pub(crate) session: Option<MergeSession>,
    /// Whether the application should quit.
    pub(crate) should_quit: bool,
    /// Which pane has focus.
    pub(crate) focused_pane: FocusedPane,
    /// The active theme.
    pub(crate) theme: Theme,
    /// Current hunk index (0-based).
    pub(crate) current_hunk_index: usize,
    /// Synchronized scroll offset for left/right panes.
    pub(crate) left_right_scroll: u16,
    /// Independent scroll offset for result pane.
    pub(crate) result_scroll: u16,
    /// Layout configuration.
    pub(crate) layout_config: LayoutConfig,
    /// Tracker for multi-key sequences (e.g., 'gg').
    pub(crate) key_sequence: KeySequence,
    /// Status message to display (with timestamp for auto-clear).
    pub(crate) status_message: Option<(String, Instant)>,
    /// Undo stack for resolution changes.
    pub(crate) undo_stack: UndoStack,
    /// Current input mode.
    pub(crate) input_mode: InputMode,
    /// Command buffer for command mode.
    pub(crate) command_buffer: String,
    /// Currently active dialog, if any.
    pub(crate) active_dialog: Option<Dialog>,
    /// Content pending for external editor (Phase 7).
    pub(crate) editor_pending: Option<String>,
    /// Configuration for diff highlighting.
    pub(crate) diff_config: diff::DiffConfig,
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
            key_sequence: KeySequence::new(),
            status_message: None,
            undo_stack: UndoStack::new(),
            input_mode: InputMode::default(),
            command_buffer: String::new(),
            active_dialog: None,
            editor_pending: None,
            diff_config: diff::DiffConfig::default(),
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
            key_sequence: KeySequence::new(),
            status_message: None,
            undo_stack: UndoStack::new(),
            input_mode: InputMode::default(),
            command_buffer: String::new(),
            active_dialog: None,
            editor_pending: None,
            diff_config: diff::DiffConfig::default(),
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
        navigation::cycle_focus(self);
    }

    /// Cycles focus to the previous pane (Left -> Result -> Right -> Left).
    pub fn cycle_focus_back(&mut self) {
        navigation::cycle_focus_back(self);
    }

    /// Sets focus directly to the result pane.
    pub fn focus_result(&mut self) {
        navigation::focus_result(self);
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
        navigation::next_hunk(self);
    }

    /// Moves to the previous hunk.
    pub fn prev_hunk(&mut self) {
        navigation::prev_hunk(self);
    }

    /// Moves to a specific hunk by index.
    pub fn go_to_hunk(&mut self, index: usize) {
        navigation::go_to_hunk(self, index);
    }

    /// Moves to the next unresolved hunk, wrapping around if necessary.
    pub fn next_unresolved_hunk(&mut self) {
        navigation::next_unresolved_hunk(self);
    }

    /// Moves to the previous unresolved hunk, wrapping around if necessary.
    pub fn prev_unresolved_hunk(&mut self) {
        navigation::prev_unresolved_hunk(self);
    }

    /// Resolves the current hunk by accepting the left (ours) content.
    pub fn resolve_left(&mut self) {
        resolution::resolve_left(self);
    }

    /// Resolves the current hunk by accepting the right (theirs) content.
    pub fn resolve_right(&mut self) {
        resolution::resolve_right(self);
    }

    /// Resolves the current hunk by accepting both sides (left then right).
    pub fn resolve_both(&mut self) {
        resolution::resolve_both(self);
    }

    /// Clears the resolution for the current hunk, returning it to unresolved state.
    pub fn clear_current_resolution(&mut self) {
        resolution::clear_current_resolution(self);
    }

    /// Undoes the last resolution action.
    pub fn undo(&mut self) {
        resolution::undo(self);
    }

    /// Scrolls up by the specified number of lines.
    pub fn scroll_up(&mut self, lines: u16) {
        navigation::scroll_up(self, lines);
    }

    /// Scrolls down by the specified number of lines.
    pub fn scroll_down(&mut self, lines: u16) {
        navigation::scroll_down(self, lines);
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

    /// Returns a reference to the diff configuration.
    #[must_use]
    pub fn diff_config(&self) -> &diff::DiffConfig {
        &self.diff_config
    }

    /// Toggles word-level diff highlighting on/off.
    pub fn toggle_word_diff(&mut self) {
        self.diff_config.word_diff = !self.diff_config.word_diff;
        let status = if self.diff_config.word_diff {
            "Word diff enabled"
        } else {
            "Word diff disabled"
        };
        self.set_status_message(status);
    }

    /// Sets a status message to display in the status bar.
    ///
    /// The message will auto-clear after a few seconds.
    pub fn set_status_message(&mut self, msg: &str) {
        self.status_message = Some((msg.to_string(), Instant::now()));
    }

    /// Returns the current status message and its timestamp, if any.
    #[must_use]
    pub fn status_message(&self) -> Option<&(String, Instant)> {
        self.status_message.as_ref()
    }

    /// Returns the current input mode.
    #[must_use]
    pub fn input_mode(&self) -> InputMode {
        self.input_mode
    }

    /// Enters command mode (for `:` commands).
    pub fn enter_command_mode(&mut self) {
        self.input_mode = InputMode::Command;
        self.command_buffer.clear();
    }

    /// Exits command mode and returns to normal mode.
    pub fn exit_command_mode(&mut self) {
        self.input_mode = InputMode::Normal;
        self.command_buffer.clear();
    }

    /// Returns the current command buffer contents.
    #[must_use]
    pub fn command_buffer(&self) -> &str {
        &self.command_buffer
    }

    /// Appends a character to the command buffer.
    pub fn append_to_command(&mut self, c: char) {
        self.command_buffer.push(c);
    }

    /// Removes the last character from the command buffer.
    pub fn backspace_command(&mut self) {
        self.command_buffer.pop();
    }

    /// Executes the current command buffer.
    pub fn execute_command(&mut self) {
        let cmd = Command::parse(&self.command_buffer);
        match cmd {
            Command::Write => self.write_file(),
            Command::Quit => self.try_quit(),
            Command::WriteQuit => {
                // TODO: Implement :wq when file writing is implemented
                self.set_status_message(":wq not yet implemented - use :q! to force quit");
            }
            Command::ForceQuit => self.quit(),
            Command::Unknown(s) => {
                if !s.is_empty() {
                    self.set_status_message(&format!("Unknown command: {s}"));
                }
            }
        }
        self.exit_command_mode();
    }

    /// Writes the resolved file. Currently a placeholder.
    fn write_file(&mut self) {
        if self.has_unresolved_hunks() {
            let count = self.unresolved_count();
            self.set_status_message(&format!("Cannot save: {count} unresolved hunks"));
        } else {
            // TODO: Implement actual file writing in Phase 7
            self.set_status_message("File saved (not yet implemented)");
        }
    }

    /// Attempts to quit, showing a warning if there are unresolved hunks.
    fn try_quit(&mut self) {
        if self.has_unresolved_hunks() {
            let count = self.unresolved_count();
            self.set_status_message(&format!("{count} unresolved hunks. Use :q! to force quit"));
        } else {
            self.quit();
        }
    }

    /// Returns true if there are unresolved hunks.
    fn has_unresolved_hunks(&self) -> bool {
        self.unresolved_count() > 0
    }

    /// Returns the number of unresolved hunks.
    fn unresolved_count(&self) -> usize {
        self.session
            .as_ref()
            .map_or(0, |s| s.unresolved_hunks().len())
    }

    /// Shows the help dialog.
    pub fn show_help(&mut self) {
        dialog::show_help(self);
    }

    /// Closes any open dialog and returns to normal mode.
    pub fn close_dialog(&mut self) {
        dialog::close_dialog(self);
    }

    /// Returns the currently active dialog, if any.
    #[must_use]
    pub fn active_dialog(&self) -> Option<&Dialog> {
        self.active_dialog.as_ref()
    }

    /// Shows the `AcceptBoth` options dialog.
    pub fn show_accept_both_dialog(&mut self) {
        dialog::show_accept_both_dialog(self);
    }

    /// Toggles the order in the `AcceptBoth` options dialog.
    pub fn toggle_accept_both_order(&mut self) {
        dialog::toggle_accept_both_order(self);
    }

    /// Toggles the deduplicate option in the `AcceptBoth` options dialog.
    pub fn toggle_accept_both_dedupe(&mut self) {
        dialog::toggle_accept_both_dedupe(self);
    }

    /// Confirms the `AcceptBoth` options and applies the resolution.
    pub fn confirm_accept_both(&mut self) {
        dialog::confirm_accept_both(self);
    }

    // --- Phase 7: Editor Integration ---

    /// Prepares content for external editor and sets pending state.
    /// Returns true if editor should be launched.
    pub fn prepare_editor(&mut self) -> bool {
        editor::prepare_editor(self)
    }

    /// Takes the pending editor content, clearing the pending state.
    pub fn take_editor_pending(&mut self) -> Option<String> {
        editor::take_editor_pending(self)
    }

    /// Applies content returned from the external editor as a manual resolution.
    pub fn apply_editor_result(&mut self, content: &str) {
        editor::apply_editor_result(self, content);
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
    use weavr_core::BothOrder;

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
        app.prev_unresolved_hunk();
        assert_eq!(app.current_hunk_index(), 0);
    }

    #[test]
    fn focus_result_sets_pane() {
        let mut app = App::new();
        assert_eq!(app.focused_pane(), FocusedPane::Left);

        app.focus_result();
        assert_eq!(app.focused_pane(), FocusedPane::Result);
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

    #[test]
    fn show_accept_both_dialog_opens_dialog() {
        let mut app = App::new();
        assert!(app.active_dialog().is_none());
        assert_eq!(app.input_mode(), InputMode::Normal);

        app.show_accept_both_dialog();

        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::AcceptBothOptions(_))
        ));
        assert_eq!(app.input_mode(), InputMode::Dialog);
    }

    #[test]
    fn toggle_accept_both_order_changes_order() {
        let mut app = App::new();
        app.show_accept_both_dialog();

        // Default is LeftThenRight
        if let Some(Dialog::AcceptBothOptions(state)) = app.active_dialog() {
            assert_eq!(state.order, BothOrder::LeftThenRight);
        }

        app.toggle_accept_both_order();

        if let Some(Dialog::AcceptBothOptions(state)) = app.active_dialog() {
            assert_eq!(state.order, BothOrder::RightThenLeft);
        }

        app.toggle_accept_both_order();

        if let Some(Dialog::AcceptBothOptions(state)) = app.active_dialog() {
            assert_eq!(state.order, BothOrder::LeftThenRight);
        }
    }

    #[test]
    fn toggle_accept_both_dedupe_changes_dedupe() {
        let mut app = App::new();
        app.show_accept_both_dialog();

        // Default is false
        if let Some(Dialog::AcceptBothOptions(state)) = app.active_dialog() {
            assert!(!state.deduplicate);
        }

        app.toggle_accept_both_dedupe();

        if let Some(Dialog::AcceptBothOptions(state)) = app.active_dialog() {
            assert!(state.deduplicate);
        }
    }

    #[test]
    fn close_dialog_from_accept_both() {
        let mut app = App::new();
        app.show_accept_both_dialog();

        assert!(app.active_dialog().is_some());
        app.close_dialog();
        assert!(app.active_dialog().is_none());
        assert_eq!(app.input_mode(), InputMode::Normal);
    }

    #[test]
    fn prepare_editor_without_session_returns_false() {
        let mut app = App::new();
        assert!(!app.prepare_editor());
        assert!(app.take_editor_pending().is_none());
    }

    #[test]
    fn take_editor_pending_clears_pending() {
        let mut app = App::new();
        // Manually set pending for testing
        app.editor_pending = Some("test content".to_string());

        let content = app.take_editor_pending();
        assert_eq!(content, Some("test content".to_string()));

        // Second call returns None
        assert!(app.take_editor_pending().is_none());
    }
}
