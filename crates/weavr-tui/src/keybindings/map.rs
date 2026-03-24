use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::str::FromStr;

use crossterm::event::{KeyCode, KeyModifiers};

use super::action::Action;
use super::notation::{
    display_key_notation, display_single_key, parse_key_notation, KeyInput, KeyNotationError,
};

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
    ///
    /// If the key or sequence was previously bound to a different action, the
    /// old binding is replaced and the reverse lookup map is kept consistent.
    pub fn bind(&mut self, action: Action, input: KeyInput) {
        match &input {
            KeyInput::Single(code, mods) => {
                let key = (*code, *mods);
                // If this key was previously bound, remove the old reverse entry.
                if let Some(prev_action) = self.singles.insert(key, action) {
                    if prev_action != action {
                        Self::remove_reverse_entry(&mut self.reverse, prev_action, &input);
                    }
                }
            }
            KeyInput::Sequence(keys) => {
                // If an identical sequence already exists, update its action.
                let mut updated_existing = false;
                for (existing_keys, existing_action) in &mut self.sequences {
                    if existing_keys == keys {
                        if *existing_action != action {
                            let prev_action = *existing_action;
                            *existing_action = action;
                            Self::remove_reverse_entry(&mut self.reverse, prev_action, &input);
                        }
                        updated_existing = true;
                        break;
                    }
                }

                if !updated_existing {
                    // Add all proper prefixes to the prefix set.
                    if keys.len() > 1 {
                        for prefix_key in &keys[..keys.len() - 1] {
                            self.sequence_prefixes.insert(*prefix_key);
                        }
                    }
                    self.sequences.push((keys.clone(), action));
                }
            }
        }
        // Update reverse map, avoiding duplicate entries for the same input.
        let entry = self.reverse.entry(action).or_default();
        if !entry.contains(&input) {
            entry.push(input);
        }
    }

    /// Removes a specific input from the reverse map for a given action.
    fn remove_reverse_entry(
        reverse: &mut HashMap<Action, Vec<KeyInput>>,
        action: Action,
        input: &KeyInput,
    ) {
        if let Some(inputs) = reverse.get_mut(&action) {
            inputs.retain(|k| k != input);
            if inputs.is_empty() {
                reverse.remove(&action);
            }
        }
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
            if keys.len() > 1 {
                for prefix_key in &keys[..keys.len() - 1] {
                    self.sequence_prefixes.insert(*prefix_key);
                }
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
        map.bind(Action::EnterEditMode, single_char('i'));

        // Display
        map.bind(Action::ToggleWordDiff, single_char('w'));
        map.bind(
            Action::ToggleBasePane,
            KeyInput::Single(KeyCode::Char('b'), KeyModifiers::CONTROL),
        );
        map.bind(Action::ToggleSyntaxHighlight, single_char('h'));

        // AI
        map.bind(Action::AiSuggest, single_char('s'));
        map.bind(Action::AiSuggestAll, single_char('S'));
        map.bind(Action::AiExplainOrHelp, single_char('?'));
        map.bind(Action::DismissAiSuggestion, single_key(KeyCode::Esc));

        // AST Merge
        map.bind(Action::AstSuggest, single_char('a'));
        map.bind(Action::AstSuggestAll, single_char('A'));
        // DismissAstSuggestion has no default binding; Esc covers both AI and AST

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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crossterm::event::{KeyCode, KeyModifiers};

    use super::*;

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
    fn h_key_bound_to_toggle_syntax_highlight() {
        let map = KeybindingMap::defaults();
        assert_eq!(
            map.lookup_single(KeyCode::Char('h'), KeyModifiers::NONE),
            Some(Action::ToggleSyntaxHighlight)
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
