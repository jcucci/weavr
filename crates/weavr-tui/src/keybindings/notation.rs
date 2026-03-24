use crossterm::event::{KeyCode, KeyModifiers};

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
pub(super) fn display_single_key(code: KeyCode, mods: KeyModifiers) -> String {
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
}
