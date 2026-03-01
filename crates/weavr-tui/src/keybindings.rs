//! Custom keybinding support.
//!
//! This module provides configurable keybinding mappings for normal mode.
//! Users can override default bindings via the `[keybindings]` section
//! in their TOML config. Dialog mode and command mode keybindings remain
//! hardcoded.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::str::FromStr;

use crossterm::event::{KeyCode, KeyModifiers};

// ---------------------------------------------------------------------------
// Action enum
// ---------------------------------------------------------------------------

/// A bindable action that can be triggered by a keybinding in normal mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    // Application
    /// Quit the application.
    Quit,
    /// Enter command mode (`:` prefix).
    EnterCommandMode,
    /// Show the help overlay.
    ShowHelp,

    // Navigation
    /// Move to the next hunk.
    NextHunk,
    /// Move to the previous hunk.
    PrevHunk,
    /// Move to the next unresolved hunk.
    NextUnresolved,
    /// Move to the previous unresolved hunk.
    PrevUnresolved,
    /// Jump to the first hunk.
    FirstHunk,
    /// Jump to the last hunk.
    LastHunk,
    /// Cycle focus to the next pane.
    CycleFocus,
    /// Cycle focus to the previous pane.
    CycleFocusBack,
    /// Focus the result pane (or accept AI suggestion if present).
    FocusResult,

    // Scrolling
    /// Scroll down half a page.
    ScrollHalfDown,
    /// Scroll up half a page.
    ScrollHalfUp,
    /// Scroll down a full page.
    ScrollPageDown,
    /// Scroll up a full page.
    ScrollPageUp,

    // Resolution
    /// Accept the left (ours) side.
    ResolveLeft,
    /// Accept the right (theirs) side.
    ResolveRight,
    /// Accept both sides with default options.
    ResolveBoth,
    /// Open the accept-both options dialog.
    ResolveBothOptions,
    /// Clear the resolution for the current hunk.
    ClearResolution,
    /// Undo the last action.
    Undo,
    /// Redo the last undone action.
    Redo,
    /// Open the current hunk in an external editor.
    EditInEditor,

    // Display
    /// Toggle word-level diff highlighting.
    ToggleWordDiff,

    // AI
    /// Request an AI suggestion for the current hunk.
    AiSuggest,
    /// Request AI suggestions for all unresolved hunks.
    AiSuggestAll,
    /// Show AI explanation (when suggestion present) or help.
    AiExplainOrHelp,
    /// Dismiss the current AI suggestion.
    DismissAiSuggestion,
}

/// All `Action` variants in declaration order.
const ALL_ACTIONS: &[Action] = &[
    Action::Quit,
    Action::EnterCommandMode,
    Action::ShowHelp,
    Action::NextHunk,
    Action::PrevHunk,
    Action::NextUnresolved,
    Action::PrevUnresolved,
    Action::FirstHunk,
    Action::LastHunk,
    Action::CycleFocus,
    Action::CycleFocusBack,
    Action::FocusResult,
    Action::ScrollHalfDown,
    Action::ScrollHalfUp,
    Action::ScrollPageDown,
    Action::ScrollPageUp,
    Action::ResolveLeft,
    Action::ResolveRight,
    Action::ResolveBoth,
    Action::ResolveBothOptions,
    Action::ClearResolution,
    Action::Undo,
    Action::Redo,
    Action::EditInEditor,
    Action::ToggleWordDiff,
    Action::AiSuggest,
    Action::AiSuggestAll,
    Action::AiExplainOrHelp,
    Action::DismissAiSuggestion,
];

impl Action {
    /// Returns a slice of all `Action` variants.
    #[must_use]
    pub fn all() -> &'static [Action] {
        ALL_ACTIONS
    }
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Action {
    /// Returns the `snake_case` config key for this action.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Quit => "quit",
            Self::EnterCommandMode => "enter_command_mode",
            Self::ShowHelp => "show_help",
            Self::NextHunk => "next_hunk",
            Self::PrevHunk => "prev_hunk",
            Self::NextUnresolved => "next_unresolved",
            Self::PrevUnresolved => "prev_unresolved",
            Self::FirstHunk => "first_hunk",
            Self::LastHunk => "last_hunk",
            Self::CycleFocus => "cycle_focus",
            Self::CycleFocusBack => "cycle_focus_back",
            Self::FocusResult => "focus_result",
            Self::ScrollHalfDown => "scroll_half_down",
            Self::ScrollHalfUp => "scroll_half_up",
            Self::ScrollPageDown => "scroll_page_down",
            Self::ScrollPageUp => "scroll_page_up",
            Self::ResolveLeft => "resolve_left",
            Self::ResolveRight => "resolve_right",
            Self::ResolveBoth => "resolve_both",
            Self::ResolveBothOptions => "resolve_both_options",
            Self::ClearResolution => "clear_resolution",
            Self::Undo => "undo",
            Self::Redo => "redo",
            Self::EditInEditor => "edit_in_editor",
            Self::ToggleWordDiff => "toggle_word_diff",
            Self::AiSuggest => "ai_suggest",
            Self::AiSuggestAll => "ai_suggest_all",
            Self::AiExplainOrHelp => "ai_explain_or_help",
            Self::DismissAiSuggestion => "dismiss_ai_suggestion",
        }
    }
}

impl FromStr for Action {
    type Err = KeyNotationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        for action in ALL_ACTIONS {
            if action.as_str() == s {
                return Ok(*action);
            }
        }
        Err(KeyNotationError::UnknownAction(s.to_string()))
    }
}

// ---------------------------------------------------------------------------
// Key input types
// ---------------------------------------------------------------------------

/// A key input: either a single key press or a multi-key sequence.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum KeyInput {
    /// A single key press with optional modifiers.
    Single(KeyCode, KeyModifiers),
    /// A sequence of key presses (e.g., `gg`).
    Sequence(Vec<(KeyCode, KeyModifiers)>),
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Error type for key notation parsing.
#[derive(Debug, thiserror::Error)]
pub enum KeyNotationError {
    /// An unknown special key name.
    #[error("unknown key: '{0}'")]
    UnknownKey(String),

    /// Empty input string.
    #[error("empty key notation")]
    EmptyInput,

    /// Unknown action name in config.
    #[error("unknown action: '{0}'")]
    UnknownAction(String),
}

// ---------------------------------------------------------------------------
// Key notation parsing
// ---------------------------------------------------------------------------

/// Parses a key notation string into a `KeyInput`.
///
/// Supported notations:
/// - Single character: `"j"`, `"G"`, `"?"`, `":"`
/// - Ctrl modifier: `"C-d"`, `"C-r"`
/// - Alt modifier: `"M-x"`, `"A-x"`
/// - Special keys: `"<Space>"`, `"<Enter>"`, `"<Esc>"`, `"<Tab>"`, `"<S-Tab>"`
/// - Function keys: `"<F1>"` through `"<F12>"`
/// - Navigation: `"<Up>"`, `"<Down>"`, `"<Left>"`, `"<Right>"`
/// - Page keys: `"<PageUp>"`, `"<PageDown>"`, `"<Home>"`, `"<End>"`
/// - Sequences: `"gg"` (multiple unmodified characters)
///
/// # Errors
///
/// Returns `KeyNotationError` if the string cannot be parsed as a valid key
/// notation.
pub fn parse_key_notation(s: &str) -> Result<KeyInput, KeyNotationError> {
    if s.is_empty() {
        return Err(KeyNotationError::EmptyInput);
    }

    // Angle-bracket notation: <F1>, <Space>, <C-d>, <S-Tab>, etc.
    if s.starts_with('<') && s.ends_with('>') {
        let inner = &s[1..s.len() - 1];
        return parse_angle_bracket(inner);
    }

    // Modifier prefix notation: C-d, M-x, A-x
    if s.len() == 3 && s.as_bytes()[1] == b'-' {
        let modifier_char = s.as_bytes()[0];
        let key_char = s.as_bytes()[2] as char;
        match modifier_char {
            b'C' => {
                return Ok(KeyInput::Single(
                    KeyCode::Char(key_char),
                    KeyModifiers::CONTROL,
                ))
            }
            b'M' | b'A' => return Ok(KeyInput::Single(KeyCode::Char(key_char), KeyModifiers::ALT)),
            _ => {}
        }
    }

    // Single character
    let chars: Vec<char> = s.chars().collect();
    if chars.len() == 1 {
        return Ok(KeyInput::Single(
            KeyCode::Char(chars[0]),
            KeyModifiers::NONE,
        ));
    }

    // Multi-character sequence (e.g., "gg", "gj")
    let keys: Vec<(KeyCode, KeyModifiers)> = chars
        .iter()
        .map(|&c| (KeyCode::Char(c), KeyModifiers::NONE))
        .collect();
    Ok(KeyInput::Sequence(keys))
}

/// Parses the inside of an angle-bracket notation (without the `<` and `>`).
fn parse_angle_bracket(inner: &str) -> Result<KeyInput, KeyNotationError> {
    // Check for modifier prefixes: C-, S-, M-, A-
    if let Some(rest) = inner.strip_prefix("C-") {
        let (code, extra_mods) = parse_key_name(rest)?;
        return Ok(KeyInput::Single(code, KeyModifiers::CONTROL | extra_mods));
    }
    if let Some(rest) = inner.strip_prefix("M-") {
        let (code, extra_mods) = parse_key_name(rest)?;
        return Ok(KeyInput::Single(code, KeyModifiers::ALT | extra_mods));
    }
    if let Some(rest) = inner.strip_prefix("A-") {
        let (code, extra_mods) = parse_key_name(rest)?;
        return Ok(KeyInput::Single(code, KeyModifiers::ALT | extra_mods));
    }
    if let Some(rest) = inner.strip_prefix("S-") {
        let (code, extra_mods) = parse_key_name(rest)?;
        // Normalize: Shift+Tab is BackTab with no extra SHIFT, since BackTab
        // inherently represents Shift+Tab.
        if code == KeyCode::Tab {
            return Ok(KeyInput::Single(KeyCode::BackTab, extra_mods));
        }
        return Ok(KeyInput::Single(code, KeyModifiers::SHIFT | extra_mods));
    }

    // No modifier prefix — parse as a special key name
    let (code, mods) = parse_key_name(inner)?;
    Ok(KeyInput::Single(code, mods))
}

/// Parses a key name (inside angle brackets, after modifier prefix removal).
fn parse_key_name(name: &str) -> Result<(KeyCode, KeyModifiers), KeyNotationError> {
    // Single char after modifier: <C-d> → inner "d"
    let chars: Vec<char> = name.chars().collect();
    if chars.len() == 1 {
        return Ok((KeyCode::Char(chars[0]), KeyModifiers::NONE));
    }

    // Named keys (case-insensitive match)
    let lower = name.to_lowercase();
    match lower.as_str() {
        "space" => Ok((KeyCode::Char(' '), KeyModifiers::NONE)),
        "enter" | "return" | "cr" => Ok((KeyCode::Enter, KeyModifiers::NONE)),
        "esc" | "escape" => Ok((KeyCode::Esc, KeyModifiers::NONE)),
        "tab" => Ok((KeyCode::Tab, KeyModifiers::NONE)),
        "backtab" | "s-tab" => Ok((KeyCode::BackTab, KeyModifiers::NONE)),
        "backspace" | "bs" => Ok((KeyCode::Backspace, KeyModifiers::NONE)),
        "delete" | "del" => Ok((KeyCode::Delete, KeyModifiers::NONE)),
        "insert" | "ins" => Ok((KeyCode::Insert, KeyModifiers::NONE)),
        "up" => Ok((KeyCode::Up, KeyModifiers::NONE)),
        "down" => Ok((KeyCode::Down, KeyModifiers::NONE)),
        "left" => Ok((KeyCode::Left, KeyModifiers::NONE)),
        "right" => Ok((KeyCode::Right, KeyModifiers::NONE)),
        "home" => Ok((KeyCode::Home, KeyModifiers::NONE)),
        "end" => Ok((KeyCode::End, KeyModifiers::NONE)),
        "pageup" | "pgup" => Ok((KeyCode::PageUp, KeyModifiers::NONE)),
        "pagedown" | "pgdn" => Ok((KeyCode::PageDown, KeyModifiers::NONE)),
        _ if lower.starts_with('f') => {
            let num_str = &lower[1..];
            if let Ok(n) = num_str.parse::<u8>() {
                if (1..=12).contains(&n) {
                    return Ok((KeyCode::F(n), KeyModifiers::NONE));
                }
            }
            Err(KeyNotationError::UnknownKey(name.to_string()))
        }
        _ => Err(KeyNotationError::UnknownKey(name.to_string())),
    }
}

// ---------------------------------------------------------------------------
// Key notation display (reverse)
// ---------------------------------------------------------------------------

/// Converts a `KeyInput` back to its human-readable notation.
#[must_use]
pub fn display_key_notation(input: &KeyInput) -> String {
    match input {
        KeyInput::Single(code, mods) => display_single_key(*code, *mods),
        KeyInput::Sequence(keys) => {
            let parts: Vec<String> = keys
                .iter()
                .map(|(code, mods)| display_single_key(*code, *mods))
                .collect();
            parts.join("")
        }
    }
}

/// Formats a single key press as a human-readable string.
fn display_single_key(code: KeyCode, mods: KeyModifiers) -> String {
    // BackTab already implies Shift; strip redundant SHIFT to avoid "Shift+S-Tab".
    let effective_mods = match code {
        KeyCode::BackTab => mods.difference(KeyModifiers::SHIFT),
        _ => mods,
    };

    let mut prefix = String::new();
    if effective_mods.contains(KeyModifiers::CONTROL) {
        prefix.push_str("Ctrl+");
    }
    if effective_mods.contains(KeyModifiers::ALT) {
        prefix.push_str("Alt+");
    }
    if effective_mods.contains(KeyModifiers::SHIFT) {
        prefix.push_str("Shift+");
    }

    let key_name = match code {
        KeyCode::Char(' ') => "Space".to_string(),
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Enter => "Enter".to_string(),
        KeyCode::Esc => "Esc".to_string(),
        KeyCode::Tab => "Tab".to_string(),
        KeyCode::BackTab => "S-Tab".to_string(),
        KeyCode::Backspace => "Backspace".to_string(),
        KeyCode::Delete => "Delete".to_string(),
        KeyCode::Insert => "Insert".to_string(),
        KeyCode::Up => "Up".to_string(),
        KeyCode::Down => "Down".to_string(),
        KeyCode::Left => "Left".to_string(),
        KeyCode::Right => "Right".to_string(),
        KeyCode::Home => "Home".to_string(),
        KeyCode::End => "End".to_string(),
        KeyCode::PageUp => "PgUp".to_string(),
        KeyCode::PageDown => "PgDn".to_string(),
        KeyCode::F(n) => format!("F{n}"),
        _ => format!("{code:?}"),
    };

    format!("{prefix}{key_name}")
}

// ---------------------------------------------------------------------------
// KeybindingMap
// ---------------------------------------------------------------------------

/// Runtime keybinding lookup map.
///
/// Supports both single-key bindings (O(1) lookup) and multi-key sequences.
pub struct KeybindingMap {
    /// Single-key bindings.
    singles: HashMap<(KeyCode, KeyModifiers), Action>,
    /// Multi-key sequence bindings.
    sequences: Vec<(Vec<(KeyCode, KeyModifiers)>, Action)>,
    /// Keys that are prefixes of multi-key sequences.
    sequence_prefixes: HashSet<(KeyCode, KeyModifiers)>,
    /// Reverse lookup: action → bound keys (for help display).
    reverse: HashMap<Action, Vec<KeyInput>>,
}

impl KeybindingMap {
    /// Creates an empty keybinding map.
    fn new() -> Self {
        Self {
            singles: HashMap::new(),
            sequences: Vec::new(),
            sequence_prefixes: HashSet::new(),
            reverse: HashMap::new(),
        }
    }

    /// Binds a key input to an action.
    pub fn bind(&mut self, action: Action, input: KeyInput) {
        match &input {
            KeyInput::Single(code, mods) => {
                self.singles.insert((*code, *mods), action);
            }
            KeyInput::Sequence(keys) => {
                // Add all proper prefixes to the prefix set
                for prefix_key in &keys[..keys.len() - 1] {
                    self.sequence_prefixes.insert(*prefix_key);
                }
                self.sequences.push((keys.clone(), action));
            }
        }
        self.reverse.entry(action).or_default().push(input);
    }

    /// Removes all bindings for a given action.
    pub fn unbind_action(&mut self, action: Action) {
        self.singles.retain(|_, a| *a != action);
        self.sequences.retain(|(_, a)| *a != action);
        self.reverse.remove(&action);
        self.rebuild_sequence_prefixes();
    }

    /// Looks up a single key press.
    #[must_use]
    pub fn lookup_single(&self, code: KeyCode, mods: KeyModifiers) -> Option<Action> {
        self.singles.get(&(code, mods)).copied()
    }

    /// Looks up a multi-key sequence.
    #[must_use]
    pub fn lookup_sequence(&self, keys: &[(KeyCode, KeyModifiers)]) -> Option<Action> {
        for (seq, action) in &self.sequences {
            if seq == keys {
                return Some(*action);
            }
        }
        None
    }

    /// Returns true if this key could be the start of a multi-key sequence.
    #[must_use]
    pub fn is_sequence_prefix(&self, code: KeyCode, mods: KeyModifiers) -> bool {
        self.sequence_prefixes.contains(&(code, mods))
    }

    /// Returns the display string for all keys bound to an action.
    ///
    /// Multiple bindings are joined with `/` (e.g., `"j/Down"`).
    #[must_use]
    pub fn display_keys_for(&self, action: Action) -> String {
        match self.reverse.get(&action) {
            Some(inputs) => inputs
                .iter()
                .map(display_key_notation)
                .collect::<Vec<_>>()
                .join("/"),
            None => String::new(),
        }
    }

    /// Returns the display string for just the first (primary) key bound
    /// to an action. Used for compact paired display (e.g., "j/k").
    #[must_use]
    pub fn display_primary_key_for(&self, action: Action) -> String {
        match self.reverse.get(&action) {
            Some(inputs) if !inputs.is_empty() => display_key_notation(&inputs[0]),
            _ => String::new(),
        }
    }

    /// Rebuilds the sequence prefix set from current sequences.
    fn rebuild_sequence_prefixes(&mut self) {
        self.sequence_prefixes.clear();
        for (keys, _) in &self.sequences {
            for prefix_key in &keys[..keys.len() - 1] {
                self.sequence_prefixes.insert(*prefix_key);
            }
        }
    }

    /// Creates the default keybinding map matching the original hardcoded bindings.
    #[must_use]
    pub fn defaults() -> Self {
        let mut map = Self::new();

        // Application
        map.bind(Action::Quit, single_char('q'));
        map.bind(Action::EnterCommandMode, single_char(':'));
        map.bind(Action::ShowHelp, single_key(KeyCode::F(1)));

        // Navigation
        map.bind(Action::NextHunk, single_char('j'));
        map.bind(Action::NextHunk, single_key(KeyCode::Down));
        map.bind(Action::PrevHunk, single_char('k'));
        map.bind(Action::PrevHunk, single_key(KeyCode::Up));
        map.bind(Action::NextUnresolved, single_char('n'));
        map.bind(Action::PrevUnresolved, single_char('N'));
        map.bind(
            Action::FirstHunk,
            KeyInput::Sequence(vec![
                (KeyCode::Char('g'), KeyModifiers::NONE),
                (KeyCode::Char('g'), KeyModifiers::NONE),
            ]),
        );
        map.bind(Action::LastHunk, single_char('G'));
        map.bind(Action::CycleFocus, single_key(KeyCode::Tab));
        map.bind(Action::CycleFocusBack, single_key(KeyCode::BackTab));
        map.bind(Action::FocusResult, single_key(KeyCode::Enter));

        // Scrolling
        map.bind(
            Action::ScrollHalfDown,
            KeyInput::Single(KeyCode::Char('d'), KeyModifiers::CONTROL),
        );
        map.bind(
            Action::ScrollHalfUp,
            KeyInput::Single(KeyCode::Char('u'), KeyModifiers::CONTROL),
        );
        map.bind(Action::ScrollPageDown, single_key(KeyCode::PageDown));
        map.bind(Action::ScrollPageUp, single_key(KeyCode::PageUp));

        // Resolution
        map.bind(Action::ResolveLeft, single_char('o'));
        map.bind(Action::ResolveRight, single_char('t'));
        map.bind(Action::ResolveBoth, single_char('b'));
        map.bind(Action::ResolveBothOptions, single_char('B'));
        map.bind(Action::ClearResolution, single_char('x'));
        map.bind(Action::Undo, single_char('u'));
        map.bind(
            Action::Redo,
            KeyInput::Single(KeyCode::Char('r'), KeyModifiers::CONTROL),
        );
        map.bind(Action::EditInEditor, single_char('e'));

        // Display
        map.bind(Action::ToggleWordDiff, single_char('w'));

        // AI
        map.bind(Action::AiSuggest, single_char('s'));
        map.bind(Action::AiSuggestAll, single_char('S'));
        map.bind(Action::AiExplainOrHelp, single_char('?'));
        map.bind(Action::DismissAiSuggestion, single_key(KeyCode::Esc));

        map
    }
}

impl fmt::Debug for KeybindingMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KeybindingMap")
            .field("singles_count", &self.singles.len())
            .field("sequences_count", &self.sequences.len())
            .finish_non_exhaustive()
    }
}

/// Helper: creates a `KeyInput::Single` for an unmodified character.
fn single_char(c: char) -> KeyInput {
    KeyInput::Single(KeyCode::Char(c), KeyModifiers::NONE)
}

/// Helper: creates a `KeyInput::Single` for a special key with no modifiers.
fn single_key(code: KeyCode) -> KeyInput {
    KeyInput::Single(code, KeyModifiers::NONE)
}

// ---------------------------------------------------------------------------
// Config builder
// ---------------------------------------------------------------------------

/// Builds a `KeybindingMap` from user overrides applied on top of defaults.
///
/// Each entry maps an action name (`snake_case`) to a list of key notation
/// strings. For example: `"next_hunk" → ["j", "<Down>"]`.
///
/// Returns the resolved map and a list of warning messages (e.g., duplicate
/// bindings or prefix conflicts).
///
/// # Errors
///
/// Returns `KeyNotationError` if a key notation string cannot be parsed or
/// an action name is unrecognized.
pub fn build_from_config(
    overrides: &BTreeMap<String, Vec<String>>,
) -> Result<(KeybindingMap, Vec<String>), KeyNotationError> {
    let mut map = KeybindingMap::defaults();
    let mut warnings = Vec::new();

    for (action_name, key_strs) in overrides {
        let action = Action::from_str(action_name)?;

        // Remove existing bindings for this action before applying overrides
        map.unbind_action(action);

        for key_str in key_strs {
            let input = parse_key_notation(key_str)?;
            map.bind(action, input);
        }
    }

    // Detect conflicts: check for keys bound to multiple actions
    detect_conflicts(&map, &mut warnings);

    Ok((map, warnings))
}

/// Detects duplicate key bindings and prefix conflicts, adding warnings.
fn detect_conflicts(map: &KeybindingMap, warnings: &mut Vec<String>) {
    // Check for single keys that are also sequence prefixes
    for (&(code, mods), &action) in &map.singles {
        if map.sequence_prefixes.contains(&(code, mods)) {
            let key_display = display_single_key(code, mods);
            warnings.push(format!(
                "key '{key_display}' is bound to '{action}' but also starts a key sequence; \
                 single press will be delayed",
            ));
        }
    }

    // Check for the same key bound to multiple actions via the reverse map.
    // Build key → Vec<Action> and warn when a key maps to more than one action.
    let mut key_to_actions: HashMap<(KeyCode, KeyModifiers), Vec<Action>> = HashMap::new();
    for (&action, inputs) in &map.reverse {
        for input in inputs {
            if let KeyInput::Single(code, mods) = input {
                key_to_actions
                    .entry((*code, *mods))
                    .or_default()
                    .push(action);
            }
        }
    }
    for ((code, mods), actions) in &key_to_actions {
        if actions.len() > 1 {
            let key_display = display_single_key(*code, *mods);
            let action_names: Vec<_> = actions.iter().map(ToString::to_string).collect();
            warnings.push(format!(
                "key '{key_display}' is bound to multiple actions: {}; last binding wins",
                action_names.join(", "),
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Key notation parsing -------------------------------------------------

    #[test]
    fn parse_single_char() {
        assert_eq!(
            parse_key_notation("j").unwrap(),
            KeyInput::Single(KeyCode::Char('j'), KeyModifiers::NONE)
        );
    }

    #[test]
    fn parse_uppercase_char() {
        assert_eq!(
            parse_key_notation("G").unwrap(),
            KeyInput::Single(KeyCode::Char('G'), KeyModifiers::NONE)
        );
    }

    #[test]
    fn parse_special_char() {
        assert_eq!(
            parse_key_notation("?").unwrap(),
            KeyInput::Single(KeyCode::Char('?'), KeyModifiers::NONE)
        );
    }

    #[test]
    fn parse_ctrl_modifier() {
        assert_eq!(
            parse_key_notation("C-d").unwrap(),
            KeyInput::Single(KeyCode::Char('d'), KeyModifiers::CONTROL)
        );
    }

    #[test]
    fn parse_alt_modifier() {
        assert_eq!(
            parse_key_notation("M-x").unwrap(),
            KeyInput::Single(KeyCode::Char('x'), KeyModifiers::ALT)
        );
        assert_eq!(
            parse_key_notation("A-x").unwrap(),
            KeyInput::Single(KeyCode::Char('x'), KeyModifiers::ALT)
        );
    }

    #[test]
    fn parse_angle_bracket_ctrl() {
        assert_eq!(
            parse_key_notation("<C-d>").unwrap(),
            KeyInput::Single(KeyCode::Char('d'), KeyModifiers::CONTROL)
        );
    }

    #[test]
    fn parse_function_key() {
        assert_eq!(
            parse_key_notation("<F1>").unwrap(),
            KeyInput::Single(KeyCode::F(1), KeyModifiers::NONE)
        );
        assert_eq!(
            parse_key_notation("<F12>").unwrap(),
            KeyInput::Single(KeyCode::F(12), KeyModifiers::NONE)
        );
    }

    #[test]
    fn parse_special_keys() {
        assert_eq!(
            parse_key_notation("<Space>").unwrap(),
            KeyInput::Single(KeyCode::Char(' '), KeyModifiers::NONE)
        );
        assert_eq!(
            parse_key_notation("<Enter>").unwrap(),
            KeyInput::Single(KeyCode::Enter, KeyModifiers::NONE)
        );
        assert_eq!(
            parse_key_notation("<Esc>").unwrap(),
            KeyInput::Single(KeyCode::Esc, KeyModifiers::NONE)
        );
        assert_eq!(
            parse_key_notation("<Tab>").unwrap(),
            KeyInput::Single(KeyCode::Tab, KeyModifiers::NONE)
        );
        // <S-Tab> normalizes to BackTab (BackTab inherently means Shift+Tab)
        assert_eq!(
            parse_key_notation("<S-Tab>").unwrap(),
            KeyInput::Single(KeyCode::BackTab, KeyModifiers::NONE)
        );
    }

    #[test]
    fn parse_navigation_keys() {
        assert_eq!(
            parse_key_notation("<Up>").unwrap(),
            KeyInput::Single(KeyCode::Up, KeyModifiers::NONE)
        );
        assert_eq!(
            parse_key_notation("<Down>").unwrap(),
            KeyInput::Single(KeyCode::Down, KeyModifiers::NONE)
        );
        assert_eq!(
            parse_key_notation("<PageUp>").unwrap(),
            KeyInput::Single(KeyCode::PageUp, KeyModifiers::NONE)
        );
        assert_eq!(
            parse_key_notation("<PageDown>").unwrap(),
            KeyInput::Single(KeyCode::PageDown, KeyModifiers::NONE)
        );
    }

    #[test]
    fn parse_sequence() {
        assert_eq!(
            parse_key_notation("gg").unwrap(),
            KeyInput::Sequence(vec![
                (KeyCode::Char('g'), KeyModifiers::NONE),
                (KeyCode::Char('g'), KeyModifiers::NONE),
            ])
        );
    }

    #[test]
    fn parse_empty_is_error() {
        assert!(parse_key_notation("").is_err());
    }

    #[test]
    fn parse_unknown_key_is_error() {
        assert!(parse_key_notation("<FooBar>").is_err());
    }

    // -- Display notation round-trip ------------------------------------------

    #[test]
    fn display_single_char_roundtrip() {
        let input = KeyInput::Single(KeyCode::Char('j'), KeyModifiers::NONE);
        assert_eq!(display_key_notation(&input), "j");
    }

    #[test]
    fn display_ctrl_key() {
        let input = KeyInput::Single(KeyCode::Char('d'), KeyModifiers::CONTROL);
        assert_eq!(display_key_notation(&input), "Ctrl+d");
    }

    #[test]
    fn display_function_key() {
        let input = KeyInput::Single(KeyCode::F(1), KeyModifiers::NONE);
        assert_eq!(display_key_notation(&input), "F1");
    }

    #[test]
    fn display_sequence() {
        let input = KeyInput::Sequence(vec![
            (KeyCode::Char('g'), KeyModifiers::NONE),
            (KeyCode::Char('g'), KeyModifiers::NONE),
        ]);
        assert_eq!(display_key_notation(&input), "gg");
    }

    #[test]
    fn display_special_keys() {
        assert_eq!(
            display_key_notation(&KeyInput::Single(KeyCode::Char(' '), KeyModifiers::NONE)),
            "Space"
        );
        assert_eq!(
            display_key_notation(&KeyInput::Single(KeyCode::Enter, KeyModifiers::NONE)),
            "Enter"
        );
        assert_eq!(
            display_key_notation(&KeyInput::Single(KeyCode::PageDown, KeyModifiers::NONE)),
            "PgDn"
        );
    }

    // -- KeybindingMap --------------------------------------------------------

    #[test]
    fn defaults_has_all_expected_bindings() {
        let map = KeybindingMap::defaults();

        // Spot-check common bindings
        assert_eq!(
            map.lookup_single(KeyCode::Char('q'), KeyModifiers::NONE),
            Some(Action::Quit)
        );
        assert_eq!(
            map.lookup_single(KeyCode::Char('j'), KeyModifiers::NONE),
            Some(Action::NextHunk)
        );
        assert_eq!(
            map.lookup_single(KeyCode::Down, KeyModifiers::NONE),
            Some(Action::NextHunk)
        );
        assert_eq!(
            map.lookup_single(KeyCode::Char('o'), KeyModifiers::NONE),
            Some(Action::ResolveLeft)
        );
        assert_eq!(
            map.lookup_single(KeyCode::Char('d'), KeyModifiers::CONTROL),
            Some(Action::ScrollHalfDown)
        );
        assert_eq!(
            map.lookup_single(KeyCode::F(1), KeyModifiers::NONE),
            Some(Action::ShowHelp)
        );
    }

    #[test]
    fn defaults_has_gg_sequence() {
        let map = KeybindingMap::defaults();
        let gg = vec![
            (KeyCode::Char('g'), KeyModifiers::NONE),
            (KeyCode::Char('g'), KeyModifiers::NONE),
        ];
        assert_eq!(map.lookup_sequence(&gg), Some(Action::FirstHunk));
    }

    #[test]
    fn g_is_sequence_prefix() {
        let map = KeybindingMap::defaults();
        assert!(map.is_sequence_prefix(KeyCode::Char('g'), KeyModifiers::NONE));
        assert!(!map.is_sequence_prefix(KeyCode::Char('j'), KeyModifiers::NONE));
    }

    #[test]
    fn display_keys_for_action() {
        let map = KeybindingMap::defaults();
        let display = map.display_keys_for(Action::NextHunk);
        assert!(display.contains('j'));
        assert!(display.contains("Down"));
    }

    #[test]
    fn unbind_action_removes_all_bindings() {
        let mut map = KeybindingMap::defaults();
        map.unbind_action(Action::NextHunk);
        assert_eq!(
            map.lookup_single(KeyCode::Char('j'), KeyModifiers::NONE),
            None
        );
        assert_eq!(map.lookup_single(KeyCode::Down, KeyModifiers::NONE), None);
        assert!(map.display_keys_for(Action::NextHunk).is_empty());
    }

    // -- Action parsing -------------------------------------------------------

    #[test]
    fn action_from_str_valid() {
        assert_eq!(Action::from_str("quit").unwrap(), Action::Quit);
        assert_eq!(Action::from_str("next_hunk").unwrap(), Action::NextHunk);
        assert_eq!(Action::from_str("redo").unwrap(), Action::Redo);
    }

    #[test]
    fn action_from_str_unknown() {
        assert!(Action::from_str("nonexistent").is_err());
    }

    #[test]
    fn action_roundtrip() {
        for action in Action::all() {
            let s = action.as_str();
            let parsed = Action::from_str(s).unwrap();
            assert_eq!(*action, parsed);
        }
    }

    // -- Config builder -------------------------------------------------------

    #[test]
    fn build_from_empty_config_returns_defaults() {
        let overrides = BTreeMap::new();
        let (map, _) = build_from_config(&overrides).unwrap();
        assert_eq!(
            map.lookup_single(KeyCode::Char('j'), KeyModifiers::NONE),
            Some(Action::NextHunk)
        );
    }

    #[test]
    fn build_from_config_overrides_action() {
        let mut overrides = BTreeMap::new();
        overrides.insert("next_hunk".to_string(), vec!["n".to_string()]);
        let (map, _) = build_from_config(&overrides).unwrap();

        // 'n' should now be NextHunk, not NextUnresolved
        assert_eq!(
            map.lookup_single(KeyCode::Char('n'), KeyModifiers::NONE),
            Some(Action::NextHunk)
        );
        // 'j' and Down should no longer be bound to NextHunk (overrides replace all)
        assert_ne!(
            map.lookup_single(KeyCode::Char('j'), KeyModifiers::NONE),
            Some(Action::NextHunk)
        );
    }

    #[test]
    fn build_from_config_multiple_keys() {
        let mut overrides = BTreeMap::new();
        overrides.insert(
            "next_hunk".to_string(),
            vec!["n".to_string(), "<Down>".to_string()],
        );
        let (map, _) = build_from_config(&overrides).unwrap();

        assert_eq!(
            map.lookup_single(KeyCode::Char('n'), KeyModifiers::NONE),
            Some(Action::NextHunk)
        );
        assert_eq!(
            map.lookup_single(KeyCode::Down, KeyModifiers::NONE),
            Some(Action::NextHunk)
        );
    }

    #[test]
    fn build_from_config_unknown_action_is_error() {
        let mut overrides = BTreeMap::new();
        overrides.insert("nonexistent".to_string(), vec!["x".to_string()]);
        assert!(build_from_config(&overrides).is_err());
    }

    #[test]
    fn build_from_config_bad_key_notation_is_error() {
        let mut overrides = BTreeMap::new();
        overrides.insert("quit".to_string(), vec!["<Nonexistent>".to_string()]);
        assert!(build_from_config(&overrides).is_err());
    }

    #[test]
    fn w_key_bound_to_toggle_word_diff() {
        let map = KeybindingMap::defaults();
        assert_eq!(
            map.lookup_single(KeyCode::Char('w'), KeyModifiers::NONE),
            Some(Action::ToggleWordDiff)
        );
    }

    #[test]
    fn prefix_conflict_produces_warning() {
        let mut overrides = BTreeMap::new();
        // Bind 'g' as a single key to an action — conflicts with 'gg' sequence
        overrides.insert("last_hunk".to_string(), vec!["g".to_string()]);
        let (_, warnings) = build_from_config(&overrides).unwrap();
        assert!(
            warnings.iter().any(|w| w.contains("'g'")),
            "Expected warning about 'g' prefix conflict, got: {warnings:?}"
        );
    }
}
