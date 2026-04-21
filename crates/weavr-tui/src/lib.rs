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

pub mod ai;
pub mod ast;
pub mod dialog;
pub mod diff;
pub mod editor;
pub mod event;
pub mod help;
pub mod input;
pub mod keybindings;
pub mod navigation;
pub mod resolution;
pub mod theme;
pub mod ui;
pub mod workspace;
use input::{Command, Dialog, InputMode, KeySequence};
use keybindings::KeybindingMap;
use weavr_core::ActionHistory;

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
#[allow(clippy::struct_excessive_bools)]
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
    /// Action history for undo/redo support.
    pub(crate) action_history: ActionHistory,
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
    /// Keybinding map for normal mode.
    pub(crate) keybindings: KeybindingMap,
    /// Cached help sections (built from keybindings).
    pub(crate) help_sections: Vec<help::HelpSection>,
    /// AI integration handle (optional, set by CLI when AI is configured).
    pub(crate) ai_handle: Option<ai::AiHandle>,
    /// AI suggestion state for UI rendering.
    pub(crate) ai_state: ai::AiState,
    /// AST merge strategy (optional, set by CLI when AST mergers are available).
    #[cfg(feature = "ast")]
    pub(crate) ast_strategy: Option<weavr_ast::AstStrategy>,
    /// AST merge suggestion state for UI rendering.
    pub(crate) ast_state: ast::AstState,
    /// Whether the user requested staging (set by `:wa` or staging prompt).
    pub(crate) stage_requested: bool,
    /// Whether to show a staging prompt on `:wq`.
    pub(crate) stage_prompt: bool,
    /// Multi-file workspace (None for single-file mode).
    pub(crate) workspace: Option<workspace::Workspace>,
    /// Whether a partial write was requested (single-file mode).
    pub(crate) partial_write: bool,
    /// Whether the user explicitly requested a write (`:w`, `:wq`, `:wa`, or `:q` when resolved).
    pub(crate) write_requested: bool,
}

impl App {
    /// Creates a new application instance with the default theme.
    #[must_use]
    pub fn new() -> Self {
        let keybindings = KeybindingMap::defaults();
        let help_sections = help::build_help_sections(&keybindings);
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
            action_history: ActionHistory::new(),
            input_mode: InputMode::default(),
            command_buffer: String::new(),
            active_dialog: None,
            editor_pending: None,
            diff_config: diff::DiffConfig::default(),
            keybindings,
            help_sections,
            ai_handle: None,
            ai_state: ai::AiState::default(),
            #[cfg(feature = "ast")]
            ast_strategy: None,
            ast_state: ast::AstState::default(),
            stage_requested: false,
            stage_prompt: false,
            workspace: None,
            partial_write: false,
            write_requested: false,
        }
    }

    /// Creates a new application instance with the specified theme.
    #[must_use]
    pub fn with_theme(theme_name: ThemeName) -> Self {
        let keybindings = KeybindingMap::defaults();
        let help_sections = help::build_help_sections(&keybindings);
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
            action_history: ActionHistory::new(),
            input_mode: InputMode::default(),
            command_buffer: String::new(),
            active_dialog: None,
            editor_pending: None,
            diff_config: diff::DiffConfig::default(),
            keybindings,
            help_sections,
            ai_handle: None,
            ai_state: ai::AiState::default(),
            #[cfg(feature = "ast")]
            ast_strategy: None,
            ast_state: ast::AstState::default(),
            stage_requested: false,
            stage_prompt: false,
            workspace: None,
            partial_write: false,
            write_requested: false,
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

    /// Takes ownership of the session, leaving `None` in its place.
    ///
    /// Use this after the TUI exits to access the session for the
    /// apply/validate/complete lifecycle.
    pub fn take_session(&mut self) -> Option<MergeSession> {
        self.session.take()
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

    /// Redoes the last undone resolution action.
    pub fn redo(&mut self) {
        resolution::redo(self);
    }

    /// Returns whether undo is available.
    #[must_use]
    pub fn can_undo(&self) -> bool {
        self.action_history.can_undo()
    }

    /// Returns whether redo is available.
    #[must_use]
    pub fn can_redo(&self) -> bool {
        self.action_history.can_redo()
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
            Command::WritePartial => self.write_file_partial(),
            Command::WritePartialQuit => self.write_partial_and_quit(),
            Command::WriteAndStage => self.write_and_stage(),
            Command::Quit => self.try_quit(),
            Command::WriteQuit => self.write_and_quit(),
            Command::ForceQuit => self.quit(),
            Command::Help => self.show_help(),
            Command::NextFile => self.next_file(),
            Command::PrevFile => self.prev_file(),
            Command::ShowFileList => self.show_file_list(),
            Command::GoToFile(idx) => self.go_to_file(idx),
            Command::Unknown(s) => {
                if !s.is_empty() {
                    self.set_status_message(&format!("Unknown command: {s}"));
                }
            }
        }
        self.exit_command_mode();
    }

    /// Writes the resolved file (`:w`).
    fn write_file(&mut self) {
        if self.has_unresolved_hunks() {
            let count = self.unresolved_count();
            self.set_status_message(&format!("Cannot save: {count} unresolved hunks"));
        } else if self.is_multi_file() {
            self.mark_current_written();
            self.set_status_message("Saved. Use :n to continue.");
        } else {
            self.write_requested = true;
            self.quit();
        }
    }

    /// Writes the resolved file, requests staging, and quits/advances (`:wa`).
    fn write_and_stage(&mut self) {
        if self.has_unresolved_hunks() {
            let count = self.unresolved_count();
            self.set_status_message(&format!("Cannot save: {count} unresolved hunks"));
        } else if self.is_multi_file() {
            self.stage_current_file();
            self.complete_current_and_advance();
        } else {
            self.write_requested = true;
            self.stage_requested = true;
            self.quit();
        }
    }

    /// Writes and quits/advances, with optional staging prompt (`:wq`).
    fn write_and_quit(&mut self) {
        if self.has_unresolved_hunks() {
            let count = self.unresolved_count();
            self.set_status_message(&format!("Cannot save: {count} unresolved hunks"));
        } else if self.is_multi_file() {
            if self.stage_prompt {
                dialog::show_staging_prompt(self);
            } else {
                self.complete_current_and_advance();
            }
        } else if self.stage_prompt {
            self.write_requested = true;
            dialog::show_staging_prompt(self);
        } else {
            self.write_requested = true;
            self.quit();
        }
    }

    /// Writes the file with unresolved hunks preserved as conflict markers (`:w!`).
    fn write_file_partial(&mut self) {
        if !self.has_unresolved_hunks() {
            // No unresolved hunks — delegate to normal write
            self.write_file();
            return;
        }
        let count = self.unresolved_count();
        if self.is_multi_file() {
            if let Some(ref mut ws) = self.workspace {
                ws.current_mut().written = true;
                ws.current_mut().partial = true;
            }
            self.set_status_message(&format!("Saved with {count} unresolved hunks remaining"));
        } else {
            self.partial_write = true;
            self.quit();
        }
    }

    /// Writes the file with unresolved hunks and quits/advances (`:wq!`).
    fn write_partial_and_quit(&mut self) {
        if !self.has_unresolved_hunks() {
            // No unresolved hunks — delegate to normal write-quit
            self.write_and_quit();
            return;
        }
        if self.is_multi_file() {
            if let Some(ref mut ws) = self.workspace {
                ws.current_mut().partial = true;
            }
            self.complete_current_and_advance();
        } else {
            self.partial_write = true;
            self.quit();
        }
    }

    /// Attempts to quit, showing a warning if there are unresolved hunks.
    fn try_quit(&mut self) {
        if self.is_multi_file() {
            if let Some(ref ws) = self.workspace {
                if ws.all_written() {
                    self.quit();
                    return;
                }
                let unwritten = ws.files().iter().filter(|f| !f.written).count();
                self.set_status_message(&format!(
                    "{unwritten} files unwritten. Use :q! to force quit all."
                ));
            }
        } else if self.has_unresolved_hunks() {
            let count = self.unresolved_count();
            self.set_status_message(&format!("{count} unresolved hunks. Use :q! to force quit"));
        } else {
            self.write_requested = true;
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

    /// Returns whether the user requested staging.
    #[must_use]
    pub fn stage_requested(&self) -> bool {
        self.stage_requested
    }

    /// Returns whether a partial write was requested.
    #[must_use]
    pub fn partial_write_requested(&self) -> bool {
        self.partial_write
    }

    /// Returns whether the user explicitly requested a write.
    #[must_use]
    pub fn write_requested(&self) -> bool {
        self.write_requested
    }

    /// Sets whether to show the staging prompt on `:wq`.
    pub fn set_stage_prompt(&mut self, enabled: bool) {
        self.stage_prompt = enabled;
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

    // --- Multi-file workspace ---

    /// Sets the workspace for multi-file mode and loads the first file.
    pub fn set_workspace(&mut self, ws: workspace::Workspace) {
        self.workspace = Some(ws);
        self.load_file_state();
    }

    /// Returns `true` if the app is in multi-file mode.
    #[must_use]
    pub fn is_multi_file(&self) -> bool {
        self.workspace.is_some()
    }

    /// Returns the total number of files in the workspace, or 1 for single-file.
    #[must_use]
    pub fn file_count(&self) -> usize {
        self.workspace
            .as_ref()
            .map_or(1, workspace::Workspace::file_count)
    }

    /// Returns the current file index (0-based), or 0 for single-file.
    #[must_use]
    pub fn current_file_index(&self) -> usize {
        self.workspace
            .as_ref()
            .map_or(0, workspace::Workspace::current_index)
    }

    /// Returns a reference to the workspace, if in multi-file mode.
    #[must_use]
    pub fn workspace(&self) -> Option<&workspace::Workspace> {
        self.workspace.as_ref()
    }

    /// Takes ownership of the workspace, leaving `None` in its place.
    ///
    /// Saves the current file's state back into the workspace first,
    /// so the caller receives fully up-to-date per-file data.
    pub fn take_workspace(&mut self) -> Option<workspace::Workspace> {
        self.save_current_file_state();
        self.workspace.take()
    }

    /// Saves current App fields back into the workspace's current file state.
    fn save_current_file_state(&mut self) {
        if let Some(ref mut ws) = self.workspace {
            let file_state = ws.current_mut();
            file_state.session = self.session.take().unwrap_or_else(|| {
                // Should not happen, but provide a fallback
                weavr_core::MergeSession::from_conflicted("", std::path::PathBuf::new())
                    .unwrap_or_else(|_| {
                        panic!("failed to create fallback session");
                    })
            });
            file_state.current_hunk_index = self.current_hunk_index;
            file_state.left_right_scroll = self.left_right_scroll;
            file_state.result_scroll = self.result_scroll;
            file_state.action_history =
                std::mem::replace(&mut self.action_history, ActionHistory::new());
            file_state.stage_requested = self.stage_requested;
            file_state.focused_pane = self.focused_pane;
        }
    }

    /// Loads the workspace's current file state into App fields.
    fn load_file_state(&mut self) {
        if let Some(ref mut ws) = self.workspace {
            let file_state = ws.current_mut();
            self.session = Some(std::mem::replace(
                &mut file_state.session,
                // Temporary placeholder; will be swapped back on save
                weavr_core::MergeSession::from_conflicted("", std::path::PathBuf::new())
                    .unwrap_or_else(|_| {
                        panic!("failed to create placeholder session");
                    }),
            ));
            self.current_hunk_index = file_state.current_hunk_index;
            self.left_right_scroll = file_state.left_right_scroll;
            self.result_scroll = file_state.result_scroll;
            self.action_history =
                std::mem::replace(&mut file_state.action_history, ActionHistory::new());
            self.stage_requested = file_state.stage_requested;
            self.focused_pane = file_state.focused_pane;
        }
    }

    /// Switches to a specific file by index. Returns `true` if switched.
    fn switch_to_file(&mut self, index: usize) -> bool {
        self.save_current_file_state();
        let changed = self.workspace.as_mut().is_some_and(|ws| ws.go_to(index));
        if changed {
            self.load_file_state();
            // Clear AI and AST state for the new file
            self.ai_state = ai::AiState::default();
            self.ast_state = ast::AstState::default();
            let file_num = index + 1;
            let file_count = self.file_count();
            if let Some(ref ws) = self.workspace {
                let name = ws.current().display_name().to_string();
                self.set_status_message(&format!("[{file_num}/{file_count}] {name}"));
            }
        }
        changed
    }

    /// Moves to the next file.
    fn next_file(&mut self) {
        if !self.is_multi_file() {
            self.set_status_message("No other files");
            return;
        }
        let next_idx = self.current_file_index() + 1;
        if !self.switch_to_file(next_idx) {
            self.set_status_message("Already at last file");
        }
    }

    /// Moves to the previous file.
    fn prev_file(&mut self) {
        if !self.is_multi_file() {
            self.set_status_message("No other files");
            return;
        }
        let current = self.current_file_index();
        if current == 0 {
            self.set_status_message("Already at first file");
            return;
        }
        self.switch_to_file(current - 1);
    }

    /// Jumps to a specific file (0-indexed).
    fn go_to_file(&mut self, index: usize) {
        if !self.is_multi_file() {
            self.set_status_message("No other files");
            return;
        }
        let count = self.file_count();
        if index >= count {
            self.set_status_message(&format!("Invalid file number (1-{count})"));
            return;
        }
        if !self.switch_to_file(index) {
            self.set_status_message("Already on that file");
        }
    }

    /// Shows the file list dialog.
    fn show_file_list(&mut self) {
        if !self.is_multi_file() {
            self.set_status_message("No other files");
            return;
        }
        dialog::show_file_list(self);
    }

    /// Marks the current file as written in the workspace.
    fn mark_current_written(&mut self) {
        if let Some(ref mut ws) = self.workspace {
            ws.current_mut().written = true;
        }
    }

    /// Sets `stage_requested` on the current file in the workspace.
    fn stage_current_file(&mut self) {
        self.stage_requested = true;
        if let Some(ref mut ws) = self.workspace {
            ws.current_mut().stage_requested = true;
        }
    }

    /// Marks the current file as written, then advances to the next unwritten
    /// file or quits if all are done.
    pub(crate) fn complete_current_and_advance(&mut self) {
        self.mark_current_written();
        if let Some(ref ws) = self.workspace {
            if ws.all_written() {
                self.save_current_file_state();
                self.quit();
                return;
            }
            if let Some(next_idx) = ws.next_unwritten_index() {
                self.switch_to_file(next_idx);
            } else {
                self.save_current_file_state();
                self.quit();
            }
        }
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

    // --- AI Integration ---

    /// Sets the keybinding map for normal mode and rebuilds help sections.
    pub fn set_keybindings(&mut self, map: KeybindingMap) {
        self.help_sections = help::build_help_sections(&map);
        self.keybindings = map;
    }

    /// Returns a reference to the current keybinding map.
    #[must_use]
    pub fn keybindings(&self) -> &KeybindingMap {
        &self.keybindings
    }

    /// Returns a reference to the cached help sections.
    #[must_use]
    pub fn help_sections(&self) -> &[help::HelpSection] {
        &self.help_sections
    }

    /// Sets the AI handle for background suggestion requests.
    pub fn set_ai_handle(&mut self, handle: ai::AiHandle) {
        self.ai_handle = Some(handle);
    }

    /// Returns whether AI features are available.
    #[must_use]
    pub fn ai_available(&self) -> bool {
        self.ai_handle.is_some()
    }

    /// Returns a reference to the AI state.
    #[must_use]
    pub fn ai_state(&self) -> &ai::AiState {
        &self.ai_state
    }

    // --- AST Integration ---

    /// Sets the AST merge strategy.
    #[cfg(feature = "ast")]
    pub fn set_ast_strategy(&mut self, strategy: weavr_ast::AstStrategy) {
        self.ast_strategy = Some(strategy);
    }

    /// Returns whether AST merging is available.
    #[must_use]
    pub fn ast_available(&self) -> bool {
        #[cfg(feature = "ast")]
        {
            self.ast_strategy.is_some()
        }
        #[cfg(not(feature = "ast"))]
        {
            false
        }
    }

    /// Returns a reference to the AST state.
    #[must_use]
    pub fn ast_state(&self) -> &ast::AstState {
        &self.ast_state
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

/// Runs the TUI event loop with the given App.
///
/// This initializes the terminal, runs until `app.should_quit()` is true,
/// then restores the terminal.
///
/// # Errors
///
/// Returns an error if terminal initialization or event handling fails.
pub fn run(app: &mut App) -> std::io::Result<()> {
    let mut terminal = ratatui::init();
    let result = run_event_loop(&mut terminal, app);
    ratatui::restore();
    result
}

/// Main event loop implementation.
fn run_event_loop(terminal: &mut ratatui::DefaultTerminal, app: &mut App) -> std::io::Result<()> {
    while !app.should_quit() {
        // Check for pending editor (external editor integration)
        if let Some(content) = app.take_editor_pending() {
            // Suspend TUI
            ratatui::restore();

            // Run external editor
            let result = run_editor(&content)?;

            // Resume TUI
            *terminal = ratatui::init();

            // Apply result if editor succeeded
            if let Some(new_content) = result {
                app.apply_editor_result(&new_content);
            } else {
                app.set_status_message("Editor cancelled");
            }
            continue;
        }

        // Poll AI background events (non-blocking)
        ai::poll_ai_events(app);

        terminal.draw(|frame| ui::draw(frame, app))?;

        if let Some(evt) = event::poll_event(Duration::from_millis(100))? {
            event::handle_event(app, &evt);
        }
    }

    // Shutdown AI worker on quit
    if let Some(ref ai_handle) = app.ai_handle {
        let _ = ai_handle.send(ai::AiCommand::Shutdown);
    }

    Ok(())
}

/// Runs the external editor with the given content.
///
/// Returns `Some(content)` if the editor exited successfully, `None` otherwise.
fn run_editor(content: &str) -> std::io::Result<Option<String>> {
    use std::io::Write;

    // Prefer VISUAL, then EDITOR, then fall back to vi
    let editor_cmd = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".into());

    // Parse the editor command into program + args using shell-style splitting
    let mut parts = shell_words::split(&editor_cmd)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

    if parts.is_empty() {
        parts.push("vi".into());
    }

    let program = parts.remove(0);

    // Create temp file with content
    let mut tmp = tempfile::NamedTempFile::new()?;
    tmp.write_all(content.as_bytes())?;
    tmp.flush()?;

    // Run editor with any additional arguments
    let status = std::process::Command::new(&program)
        .args(&parts)
        .arg(tmp.path())
        .status()?;

    if status.success() {
        Ok(Some(std::fs::read_to_string(tmp.path())?))
    } else {
        Ok(None) // Editor exited with error, cancel
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

    #[test]
    fn single_file_mode_by_default() {
        let app = App::new();
        assert!(!app.is_multi_file());
        assert_eq!(app.file_count(), 1);
        assert_eq!(app.current_file_index(), 0);
        assert!(app.workspace().is_none());
    }

    #[test]
    fn set_workspace_enables_multi_file() {
        use std::path::PathBuf;
        use workspace::{FileState, Workspace};

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

        assert!(app.is_multi_file());
        assert_eq!(app.file_count(), 2);
        assert_eq!(app.current_file_index(), 0);
        assert!(app.session().is_some());
    }

    #[test]
    fn file_switching_preserves_state() {
        use std::path::PathBuf;
        use workspace::{FileState, Workspace};

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

        // Modify state for first file
        app.scroll_down(5);
        assert_eq!(app.left_right_scroll(), 5);

        // Switch to second file
        assert!(app.switch_to_file(1));
        assert_eq!(app.current_file_index(), 1);
        // Second file starts at scroll 0
        assert_eq!(app.left_right_scroll(), 0);

        // Switch back to first file
        assert!(app.switch_to_file(0));
        assert_eq!(app.current_file_index(), 0);
        // State should be restored
        assert_eq!(app.left_right_scroll(), 5);
    }

    #[test]
    fn next_file_in_single_file_mode_shows_message() {
        let mut app = App::new();
        app.next_file();
        assert!(app.status_message().is_some());
        let msg = &app.status_message().unwrap().0;
        assert!(msg.contains("No other files"));
    }

    #[test]
    fn prev_file_in_single_file_mode_shows_message() {
        let mut app = App::new();
        app.prev_file();
        assert!(app.status_message().is_some());
        let msg = &app.status_message().unwrap().0;
        assert!(msg.contains("No other files"));
    }

    #[test]
    fn show_file_list_in_single_file_shows_message() {
        let mut app = App::new();
        app.show_file_list();
        assert!(app.status_message().is_some());
        let msg = &app.status_message().unwrap().0;
        assert!(msg.contains("No other files"));
    }
}
