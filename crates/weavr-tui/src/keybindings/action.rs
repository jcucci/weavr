use std::fmt;
use std::str::FromStr;

use super::notation::KeyNotationError;

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
    /// Enter inline edit mode in the result pane.
    EnterEditMode,

    // Display
    /// Toggle word-level diff highlighting.
    ToggleWordDiff,
    /// Toggle base (ancestor) pane visibility.
    ToggleBasePane,
    /// Toggle syntax highlighting in panes.
    ToggleSyntaxHighlight,

    // AI
    /// Request an AI suggestion for the current hunk.
    AiSuggest,
    /// Request AI suggestions for all unresolved hunks.
    AiSuggestAll,
    /// Show AI explanation (when suggestion present) or help.
    AiExplainOrHelp,
    /// Dismiss the current AI suggestion.
    DismissAiSuggestion,

    // AST Merge
    /// Request an AST-based merge for the current hunk.
    AstSuggest,
    /// Request AST-based merges for all unresolved hunks.
    AstSuggestAll,
    /// Dismiss the current AST suggestion.
    DismissAstSuggestion,
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
    Action::EnterEditMode,
    Action::ToggleWordDiff,
    Action::ToggleBasePane,
    Action::ToggleSyntaxHighlight,
    Action::AiSuggest,
    Action::AiSuggestAll,
    Action::AiExplainOrHelp,
    Action::DismissAiSuggestion,
    Action::AstSuggest,
    Action::AstSuggestAll,
    Action::DismissAstSuggestion,
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
            Self::EnterEditMode => "enter_edit_mode",
            Self::ToggleWordDiff => "toggle_word_diff",
            Self::ToggleBasePane => "toggle_base_pane",
            Self::ToggleSyntaxHighlight => "toggle_syntax_highlight",
            Self::AiSuggest => "ai_suggest",
            Self::AiSuggestAll => "ai_suggest_all",
            Self::AiExplainOrHelp => "ai_explain_or_help",
            Self::DismissAiSuggestion => "dismiss_ai_suggestion",
            Self::AstSuggest => "ast_suggest",
            Self::AstSuggestAll => "ast_suggest_all",
            Self::DismissAstSuggestion => "dismiss_ast_suggestion",
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

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

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
}
