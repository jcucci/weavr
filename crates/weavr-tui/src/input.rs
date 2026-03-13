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
