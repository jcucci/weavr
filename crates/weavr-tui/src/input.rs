//! Input handling and command parsing.
//!
//! This module provides types and utilities for managing input modes,
//! parsing vim-style commands, and tracking multi-key sequences.

use std::time::{Duration, Instant};

use crossterm::event::KeyCode;

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
    /// Help overlay showing keybindings.
    Help,
    /// `AcceptBoth` options configuration dialog.
    AcceptBothOptions(AcceptBothOptionsState),
    /// AI explanation overlay.
    AiExplanation(String),
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
    /// Quit the application (`:q`).
    Quit,
    /// Write and quit (`:wq` or `:x`).
    WriteQuit,
    /// Force quit without saving (`:q!`).
    ForceQuit,
    /// Unknown or invalid command.
    Unknown(String),
}

impl Command {
    /// Parses a command string into a Command variant.
    ///
    /// The input should not include the leading `:`.
    #[must_use]
    pub fn parse(input: &str) -> Self {
        match input.trim() {
            "w" => Self::Write,
            "q" => Self::Quit,
            "wq" | "x" => Self::WriteQuit,
            "q!" => Self::ForceQuit,
            other => Self::Unknown(other.to_string()),
        }
    }

    /// Returns a description of the command for error messages.
    #[must_use]
    pub fn description(&self) -> &str {
        match self {
            Self::Write => "write",
            Self::Quit => "quit",
            Self::WriteQuit => "write and quit",
            Self::ForceQuit => "force quit",
            Self::Unknown(_) => "unknown command",
        }
    }
}

/// Tracks pending keys for multi-key sequence detection (e.g., 'gg').
#[derive(Debug, Clone, Default)]
pub struct KeySequence {
    pending: Option<(KeyCode, Instant)>,
}

impl KeySequence {
    /// Creates a new empty key sequence tracker.
    #[must_use]
    pub fn new() -> Self {
        Self { pending: None }
    }

    /// Sets a pending key for sequence detection.
    pub fn set(&mut self, key: KeyCode) {
        self.pending = Some((key, Instant::now()));
    }

    /// Checks if a pending key matches and is within the timeout.
    /// Returns true if there's a matching pending key that hasn't expired.
    /// Clears the pending key if it has expired.
    pub fn check(&mut self, expected: KeyCode, timeout: Duration) -> bool {
        if let Some((key, timestamp)) = self.pending {
            if timestamp.elapsed() > timeout {
                self.pending = None;
                return false;
            }
            return key == expected;
        }
        false
    }

    /// Clears any pending key sequence.
    pub fn clear(&mut self) {
        self.pending = None;
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
    fn parse_quit() {
        assert_eq!(Command::parse("q"), Command::Quit);
    }

    #[test]
    fn parse_write_quit() {
        assert_eq!(Command::parse("wq"), Command::WriteQuit);
        assert_eq!(Command::parse("x"), Command::WriteQuit);
    }

    #[test]
    fn parse_force_quit() {
        assert_eq!(Command::parse("q!"), Command::ForceQuit);
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
        assert!(seq.pending.is_none());
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
        assert!(seq.pending.is_none());
    }
}
