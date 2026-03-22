//! Input handling and command parsing.
//!
//! This module provides types and utilities for managing input modes,
//! parsing vim-style commands, and tracking multi-key sequences.

use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyModifiers};

/// The current input mode of the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputMode {
    /// Normal mode - standard keybindings active.
    #[default]
    Normal,
    /// Command mode - typing a vim-style command (e.g., `:w`).
    Command,
    /// Dialog mode - a modal dialog is open.
    Dialog,
    /// Edit mode - inline text editing in the result pane.
    Edit,
}

/// Sub-mode within edit mode (vim-style normal vs insert).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditSubMode {
    /// Normal sub-mode: vim navigation keys active.
    #[default]
    Normal,
    /// Insert sub-mode: typing inserts text at cursor.
    Insert,
}

/// State for the inline text editor in the result pane.
#[derive(Debug, Clone)]
pub struct EditState {
    /// Text buffer split by newlines.
    pub lines: Vec<String>,
    /// 0-based cursor row.
    pub cursor_row: usize,
    /// 0-based cursor column (byte offset within line).
    pub cursor_col: usize,
    /// Vertical scroll offset for long content.
    pub scroll_offset: usize,
    /// Current sub-mode (normal vs insert).
    pub sub_mode: EditSubMode,
    /// Pending key for multi-key sequences (e.g., `dd`).
    pub pending_key: Option<char>,
}

impl EditState {
    /// Creates a new edit state from content string.
    #[must_use]
    pub fn new(content: &str) -> Self {
        let lines: Vec<String> = if content.is_empty() {
            vec![String::new()]
        } else {
            content.lines().map(String::from).collect()
        };
        Self {
            lines,
            cursor_row: 0,
            cursor_col: 0,
            scroll_offset: 0,
            sub_mode: EditSubMode::Insert,
            pending_key: None,
        }
    }

    /// Joins the lines back into a single string.
    #[must_use]
    pub fn content(&self) -> String {
        self.lines.join("\n")
    }

    /// Inserts a character at the cursor position.
    pub fn insert_char(&mut self, c: char) {
        let line = &mut self.lines[self.cursor_row];
        if self.cursor_col >= line.len() {
            line.push(c);
            self.cursor_col = line.len();
        } else {
            line.insert(self.cursor_col, c);
            self.cursor_col += c.len_utf8();
        }
    }

    /// Deletes the character under the cursor.
    pub fn delete_char(&mut self) {
        let line = &self.lines[self.cursor_row];
        if self.cursor_col < line.len() {
            let c = self.lines[self.cursor_row].as_bytes()[self.cursor_col];
            let char_len = utf8_char_len(c);
            self.lines[self.cursor_row].drain(self.cursor_col..self.cursor_col + char_len);
        } else if self.cursor_row + 1 < self.lines.len() {
            // Join with next line
            let next = self.lines.remove(self.cursor_row + 1);
            self.lines[self.cursor_row].push_str(&next);
        }
    }

    /// Deletes the character before the cursor (backspace).
    pub fn backspace(&mut self) {
        if self.cursor_col > 0 {
            // Find the start of the previous character
            let line = &self.lines[self.cursor_row];
            let prev_boundary = prev_char_boundary(line, self.cursor_col);
            self.lines[self.cursor_row].drain(prev_boundary..self.cursor_col);
            self.cursor_col = prev_boundary;
        } else if self.cursor_row > 0 {
            // Join with previous line
            let current = self.lines.remove(self.cursor_row);
            self.cursor_row -= 1;
            self.cursor_col = self.lines[self.cursor_row].len();
            self.lines[self.cursor_row].push_str(&current);
        }
    }

    /// Inserts a newline at the cursor position.
    pub fn newline(&mut self) {
        let rest = self.lines[self.cursor_row].split_off(self.cursor_col);
        self.cursor_row += 1;
        self.lines.insert(self.cursor_row, rest);
        self.cursor_col = 0;
    }

    /// Moves cursor up one row.
    pub fn move_up(&mut self) {
        if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.clamp_cursor_col();
        }
    }

    /// Moves cursor down one row.
    pub fn move_down(&mut self) {
        if self.cursor_row + 1 < self.lines.len() {
            self.cursor_row += 1;
            self.clamp_cursor_col();
        }
    }

    /// Moves cursor left one character.
    pub fn move_left(&mut self) {
        if self.cursor_col > 0 {
            let line = &self.lines[self.cursor_row];
            self.cursor_col = prev_char_boundary(line, self.cursor_col);
        }
    }

    /// Moves cursor right one character.
    pub fn move_right(&mut self) {
        let line = &self.lines[self.cursor_row];
        if self.cursor_col < line.len() {
            let c = line.as_bytes()[self.cursor_col];
            self.cursor_col += utf8_char_len(c);
        }
    }

    /// Moves cursor to the start of the current line.
    pub fn move_to_line_start(&mut self) {
        self.cursor_col = 0;
    }

    /// Moves cursor to the end of the current line.
    pub fn move_to_line_end(&mut self) {
        self.cursor_col = self.lines[self.cursor_row].len();
    }

    /// Moves cursor forward one word.
    pub fn move_word_forward(&mut self) {
        let line = &self.lines[self.cursor_row];
        let bytes = line.as_bytes();
        let len = bytes.len();
        if self.cursor_col >= len {
            // Move to next line if possible
            if self.cursor_row + 1 < self.lines.len() {
                self.cursor_row += 1;
                self.cursor_col = 0;
            }
            return;
        }
        // Skip non-whitespace
        let mut pos = self.cursor_col;
        while pos < len && !bytes[pos].is_ascii_whitespace() {
            pos += utf8_char_len(bytes[pos]);
        }
        // Skip whitespace
        while pos < len && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        self.cursor_col = pos;
    }

    /// Moves cursor backward one word.
    pub fn move_word_back(&mut self) {
        if self.cursor_col == 0 {
            if self.cursor_row > 0 {
                self.cursor_row -= 1;
                self.cursor_col = self.lines[self.cursor_row].len();
            }
            return;
        }
        let line = &self.lines[self.cursor_row];
        let bytes = line.as_bytes();
        let mut pos = self.cursor_col;
        // Skip whitespace backwards
        while pos > 0 && bytes[pos - 1].is_ascii_whitespace() {
            pos -= 1;
        }
        // Skip non-whitespace backwards
        while pos > 0 && !bytes[pos - 1].is_ascii_whitespace() {
            pos -= 1;
        }
        self.cursor_col = pos;
    }

    /// Deletes the current line.
    pub fn delete_line(&mut self) {
        if self.lines.len() > 1 {
            self.lines.remove(self.cursor_row);
            if self.cursor_row >= self.lines.len() {
                self.cursor_row = self.lines.len() - 1;
            }
        } else {
            self.lines[0].clear();
        }
        self.clamp_cursor_col();
    }

    /// Opens a new line below the cursor and enters insert mode.
    pub fn open_line_below(&mut self) {
        self.cursor_row += 1;
        self.lines.insert(self.cursor_row, String::new());
        self.cursor_col = 0;
        self.sub_mode = EditSubMode::Insert;
    }

    /// Opens a new line above the cursor and enters insert mode.
    pub fn open_line_above(&mut self) {
        self.lines.insert(self.cursor_row, String::new());
        self.cursor_col = 0;
        self.sub_mode = EditSubMode::Insert;
    }

    /// Enters insert mode at the cursor position.
    pub fn enter_insert(&mut self) {
        self.sub_mode = EditSubMode::Insert;
    }

    /// Enters insert mode after the cursor position.
    pub fn enter_insert_after(&mut self) {
        let line = &self.lines[self.cursor_row];
        if self.cursor_col < line.len() {
            let c = line.as_bytes()[self.cursor_col];
            self.cursor_col += utf8_char_len(c);
        }
        self.sub_mode = EditSubMode::Insert;
    }

    /// Enters insert mode at the end of the current line.
    pub fn enter_insert_end(&mut self) {
        self.cursor_col = self.lines[self.cursor_row].len();
        self.sub_mode = EditSubMode::Insert;
    }

    /// Switches to normal sub-mode.
    pub fn enter_normal(&mut self) {
        self.sub_mode = EditSubMode::Normal;
        // In vim, cursor moves back one position when entering normal mode
        if self.cursor_col > 0 {
            let line = &self.lines[self.cursor_row];
            self.cursor_col = prev_char_boundary(line, self.cursor_col);
        }
    }

    /// Clamps cursor column to the current line length.
    fn clamp_cursor_col(&mut self) {
        let max = self.lines[self.cursor_row].len();
        if self.cursor_col > max {
            self.cursor_col = max;
        }
    }

    /// Ensures the cursor is visible within the given viewport height.
    pub fn ensure_cursor_visible(&mut self, viewport_height: usize) {
        if viewport_height == 0 {
            return;
        }
        if self.cursor_row < self.scroll_offset {
            self.scroll_offset = self.cursor_row;
        } else if self.cursor_row >= self.scroll_offset + viewport_height {
            self.scroll_offset = self.cursor_row - viewport_height + 1;
        }
    }
}

/// Returns the byte length of a UTF-8 character from its first byte.
fn utf8_char_len(first_byte: u8) -> usize {
    match first_byte {
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xFF => 4,
        _ => 1,
    }
}

/// Finds the previous character boundary in a string.
fn prev_char_boundary(s: &str, pos: usize) -> usize {
    let mut p = pos.saturating_sub(1);
    while p > 0 && !s.is_char_boundary(p) {
        p -= 1;
    }
    p
}

use weavr_core::BothOrder;

/// The type of dialog currently open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dialog {
    /// Help overlay showing keybindings (with scroll state).
    Help(HelpState),
    /// `AcceptBoth` options configuration dialog.
    AcceptBothOptions(AcceptBothOptionsState),
    /// AI explanation overlay.
    AiExplanation(String),
    /// Staging prompt shown on `:wq` when `stage_prompt` is enabled.
    StagingPrompt,
    /// File list dialog for multi-file navigation.
    FileList(FileListState),
}

/// State for the file list dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileListState {
    /// Currently selected index in the file list.
    pub selected_index: usize,
}

/// State for the scrollable help dialog.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HelpState {
    /// Vertical scroll offset (in lines).
    pub scroll: usize,
}

/// State for the `AcceptBoth` options dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptBothOptionsState {
    /// Order of content combination.
    pub order: BothOrder,
    /// Remove duplicate lines.
    pub deduplicate: bool,
    /// Currently focused field (0 = order, 1 = deduplicate).
    pub focused_field: usize,
}

impl Default for AcceptBothOptionsState {
    fn default() -> Self {
        Self {
            order: BothOrder::LeftThenRight,
            deduplicate: false,
            focused_field: 0,
        }
    }
}

/// A parsed vim-style command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Write/save the file (`:w`).
    Write,
    /// Write and stage the file (`:wa`).
    WriteAndStage,
    /// Quit the application (`:q`).
    Quit,
    /// Write and quit (`:wq` or `:x`).
    WriteQuit,
    /// Write with unresolved hunks, preserving conflict markers (`:w!`).
    WritePartial,
    /// Write with unresolved hunks and quit (`:wq!`).
    WritePartialQuit,
    /// Force quit without saving (`:q!`).
    ForceQuit,
    /// Show help (`:help`).
    Help,
    /// Go to next file (`:n`, `:next`).
    NextFile,
    /// Go to previous file (`:prev`).
    PrevFile,
    /// Show file list (`:files`).
    ShowFileList,
    /// Jump to a specific file by 1-indexed number (`:file N`), stored 0-indexed.
    GoToFile(usize),
    /// Unknown or invalid command.
    Unknown(String),
}

impl Command {
    /// Parses a command string into a Command variant.
    ///
    /// The input should not include the leading `:`.
    #[must_use]
    pub fn parse(input: &str) -> Self {
        let trimmed = input.trim();
        match trimmed {
            "w" => Self::Write,
            "wa" => Self::WriteAndStage,
            "q" => Self::Quit,
            "wq" | "x" => Self::WriteQuit,
            "w!" => Self::WritePartial,
            "wq!" => Self::WritePartialQuit,
            "q!" => Self::ForceQuit,
            "help" => Self::Help,
            "n" | "next" => Self::NextFile,
            "prev" => Self::PrevFile,
            "files" => Self::ShowFileList,
            other => {
                // Check for `:file N` pattern
                if let Some(num_str) = other
                    .strip_prefix("file ")
                    .or_else(|| other.strip_prefix("file\t"))
                {
                    if let Ok(n) = num_str.trim().parse::<usize>() {
                        if n >= 1 {
                            return Self::GoToFile(n - 1); // Convert 1-indexed to 0-indexed
                        }
                    }
                    return Self::Unknown(other.to_string());
                }
                Self::Unknown(other.to_string())
            }
        }
    }

    /// Returns a description of the command for error messages.
    #[must_use]
    pub fn description(&self) -> &str {
        match self {
            Self::Write => "write",
            Self::WritePartial => "write (partial)",
            Self::WritePartialQuit => "write (partial) and quit",
            Self::WriteAndStage => "write and stage",
            Self::Quit => "quit",
            Self::WriteQuit => "write and quit",
            Self::ForceQuit => "force quit",
            Self::Help => "help",
            Self::NextFile => "next file",
            Self::PrevFile => "previous file",
            Self::ShowFileList => "file list",
            Self::GoToFile(_) => "go to file",
            Self::Unknown(_) => "unknown command",
        }
    }
}

/// Tracks pending keys for multi-key sequence detection (e.g., 'gg').
#[derive(Debug, Clone, Default)]
pub struct KeySequence {
    pending: Vec<(KeyCode, KeyModifiers, Instant)>,
}

impl KeySequence {
    /// Creates a new empty key sequence tracker.
    #[must_use]
    pub fn new() -> Self {
        Self {
            pending: Vec::new(),
        }
    }

    /// Sets a pending key for sequence detection (single key, no modifiers).
    pub fn set(&mut self, key: KeyCode) {
        self.pending.clear();
        self.pending.push((key, KeyModifiers::NONE, Instant::now()));
    }

    /// Checks if a pending key matches and is within the timeout.
    /// Returns true if there's a matching pending key that hasn't expired.
    /// Clears the pending key if it has expired.
    pub fn check(&mut self, expected: KeyCode, timeout: Duration) -> bool {
        if self.pending.len() == 1 {
            let (key, _, timestamp) = self.pending[0];
            if timestamp.elapsed() > timeout {
                self.pending.clear();
                return false;
            }
            return key == expected;
        }
        false
    }

    /// Pushes a key onto the pending buffer.
    pub fn push(&mut self, code: KeyCode, mods: KeyModifiers) {
        self.pending.push((code, mods, Instant::now()));
    }

    /// Returns the pending keys as `(KeyCode, KeyModifiers)` pairs,
    /// checking that no key has timed out. If any key has expired,
    /// clears the buffer and returns an empty vec.
    pub fn pending_keys(&mut self, timeout: Duration) -> Vec<(KeyCode, KeyModifiers)> {
        if let Some((_, _, timestamp)) = self.pending.first() {
            if timestamp.elapsed() > timeout {
                self.pending.clear();
                return Vec::new();
            }
        }
        self.pending
            .iter()
            .map(|(code, mods, _)| (*code, *mods))
            .collect()
    }

    /// Returns true if there are pending keys.
    #[must_use]
    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    /// Drains and returns all pending keys as `(KeyCode, KeyModifiers)` pairs.
    pub fn drain(&mut self) -> Vec<(KeyCode, KeyModifiers)> {
        self.pending
            .drain(..)
            .map(|(code, mods, _)| (code, mods))
            .collect()
    }

    /// Clears any pending key sequence.
    pub fn clear(&mut self) {
        self.pending.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_write() {
        assert_eq!(Command::parse("w"), Command::Write);
        assert_eq!(Command::parse("  w  "), Command::Write);
    }

    #[test]
    fn parse_write_and_stage() {
        assert_eq!(Command::parse("wa"), Command::WriteAndStage);
        assert_eq!(Command::parse("  wa  "), Command::WriteAndStage);
    }

    #[test]
    fn parse_quit() {
        assert_eq!(Command::parse("q"), Command::Quit);
    }

    #[test]
    fn parse_write_quit() {
        assert_eq!(Command::parse("wq"), Command::WriteQuit);
        assert_eq!(Command::parse("x"), Command::WriteQuit);
    }

    #[test]
    fn parse_w_bang() {
        assert_eq!(Command::parse("w!"), Command::WritePartial);
        assert_eq!(Command::parse("  w!  "), Command::WritePartial);
    }

    #[test]
    fn parse_wq_bang() {
        assert_eq!(Command::parse("wq!"), Command::WritePartialQuit);
        assert_eq!(Command::parse("  wq!  "), Command::WritePartialQuit);
    }

    #[test]
    fn parse_force_quit() {
        assert_eq!(Command::parse("q!"), Command::ForceQuit);
    }

    #[test]
    fn parse_help() {
        assert_eq!(Command::parse("help"), Command::Help);
        assert_eq!(Command::parse("  help  "), Command::Help);
    }

    #[test]
    fn parse_next_file() {
        assert_eq!(Command::parse("n"), Command::NextFile);
        assert_eq!(Command::parse("next"), Command::NextFile);
        assert_eq!(Command::parse("  n  "), Command::NextFile);
    }

    #[test]
    fn parse_prev_file() {
        assert_eq!(Command::parse("prev"), Command::PrevFile);
        assert_eq!(Command::parse("  prev  "), Command::PrevFile);
    }

    #[test]
    fn parse_show_file_list() {
        assert_eq!(Command::parse("files"), Command::ShowFileList);
        assert_eq!(Command::parse("  files  "), Command::ShowFileList);
    }

    #[test]
    fn parse_go_to_file() {
        // 1-indexed input -> 0-indexed storage
        assert_eq!(Command::parse("file 1"), Command::GoToFile(0));
        assert_eq!(Command::parse("file 3"), Command::GoToFile(2));
        assert_eq!(Command::parse("  file 2  "), Command::GoToFile(1));
    }

    #[test]
    fn parse_go_to_file_zero_is_invalid() {
        assert_eq!(
            Command::parse("file 0"),
            Command::Unknown("file 0".to_string())
        );
    }

    #[test]
    fn parse_go_to_file_non_numeric_is_unknown() {
        assert_eq!(
            Command::parse("file abc"),
            Command::Unknown("file abc".to_string())
        );
    }

    #[test]
    fn parse_unknown() {
        assert_eq!(Command::parse("foo"), Command::Unknown("foo".to_string()));
        assert_eq!(Command::parse(""), Command::Unknown(String::new()));
    }

    #[test]
    fn input_mode_default_is_normal() {
        assert_eq!(InputMode::default(), InputMode::Normal);
    }

    #[test]
    fn key_sequence_new_is_empty() {
        let seq = KeySequence::new();
        assert!(seq.pending.is_empty());
    }

    #[test]
    fn key_sequence_set_and_check() {
        let mut seq = KeySequence::new();
        let timeout = Duration::from_millis(500);

        // Initially no pending key
        assert!(!seq.check(KeyCode::Char('g'), timeout));

        // Set a pending key
        seq.set(KeyCode::Char('g'));

        // Check matching key returns true
        assert!(seq.check(KeyCode::Char('g'), timeout));

        // Check non-matching key returns false
        assert!(!seq.check(KeyCode::Char('x'), timeout));
    }

    #[test]
    fn key_sequence_clear() {
        let mut seq = KeySequence::new();
        let timeout = Duration::from_millis(500);

        seq.set(KeyCode::Char('g'));
        assert!(seq.check(KeyCode::Char('g'), timeout));

        seq.clear();
        assert!(!seq.check(KeyCode::Char('g'), timeout));
    }

    #[test]
    fn key_sequence_default() {
        let seq = KeySequence::default();
        assert!(seq.pending.is_empty());
    }

    #[test]
    fn key_sequence_push_and_drain() {
        let mut seq = KeySequence::new();
        seq.push(KeyCode::Char('g'), KeyModifiers::NONE);
        assert!(seq.has_pending());
        let keys = seq.drain();
        assert_eq!(keys.len(), 1);
        assert!(!seq.has_pending());
    }

    #[test]
    fn key_sequence_pending_keys_respects_timeout() {
        let mut seq = KeySequence::new();
        seq.push(KeyCode::Char('g'), KeyModifiers::NONE);
        // With a long timeout, keys should be available
        let keys = seq.pending_keys(Duration::from_secs(10));
        assert_eq!(keys.len(), 1);
    }
}
