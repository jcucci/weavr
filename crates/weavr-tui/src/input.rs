//! Input handling and command parsing.
//!
//! This module provides types and utilities for managing input modes
//! and parsing vim-style commands.

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
}
