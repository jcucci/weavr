//! Structured help content for the help overlay.
//!
//! Provides data-driven help sections sourced from the keybinding map,
//! so that help always reflects the user's actual (possibly customized)
//! keybindings.

use crate::keybindings::{Action, KeybindingMap};

/// A single keybinding entry.
pub struct HelpBinding {
    /// The key or key combination (e.g., "o", "Ctrl+d").
    pub key: String,
    /// What the keybinding does.
    pub description: &'static str,
}

/// A titled group of keybindings.
pub struct HelpSection {
    /// Section header (e.g., "Resolution").
    pub title: &'static str,
    /// The bindings in this section.
    pub bindings: Vec<HelpBinding>,
}

/// Builds help sections from the keybinding map.
///
/// Each section groups related actions and shows the keys currently
/// bound to them (which may differ from defaults if the user has
/// customized their config).
#[must_use]
pub fn build_help_sections(map: &KeybindingMap) -> Vec<HelpSection> {
    vec![
        HelpSection {
            title: "Resolution",
            bindings: vec![
                binding(map, Action::ResolveLeft, "Accept ours (left)"),
                binding(map, Action::ResolveRight, "Accept theirs (right)"),
                binding(map, Action::ResolveBoth, "Accept both (default)"),
                binding(map, Action::ResolveBothOptions, "Accept both (options)"),
                binding(map, Action::EditInEditor, "Edit in $EDITOR"),
                binding(map, Action::ClearResolution, "Clear resolution"),
                binding(map, Action::Undo, "Undo last action"),
                binding(map, Action::Redo, "Redo last action"),
            ],
        },
        HelpSection {
            title: "Navigation",
            bindings: vec![
                paired_binding(map, Action::NextHunk, Action::PrevHunk, "Next/prev hunk"),
                paired_binding(
                    map,
                    Action::NextUnresolved,
                    Action::PrevUnresolved,
                    "Next/prev unresolved",
                ),
                paired_binding(map, Action::FirstHunk, Action::LastHunk, "First/last hunk"),
                binding(map, Action::CycleFocus, "Cycle panes"),
                binding(map, Action::FocusResult, "Focus result pane"),
            ],
        },
        HelpSection {
            title: "Scrolling",
            bindings: vec![
                binding(map, Action::ScrollHalfDown, "Scroll down"),
                binding(map, Action::ScrollHalfUp, "Scroll up"),
                binding(map, Action::ScrollPageDown, "Page down"),
                binding(map, Action::ScrollPageUp, "Page up"),
            ],
        },
        HelpSection {
            title: "AI (when configured)",
            bindings: vec![
                binding(map, Action::AiSuggest, "AI suggest (current hunk)"),
                binding(map, Action::AiSuggestAll, "AI suggest (all unresolved)"),
                binding(
                    map,
                    Action::AiExplainOrHelp,
                    "Help / AI explain (when suggestion shown)",
                ),
                binding(map, Action::FocusResult, "Accept AI suggestion"),
                binding(map, Action::DismissAiSuggestion, "Dismiss AI suggestion"),
            ],
        },
        HelpSection {
            title: "Commands",
            bindings: vec![
                HelpBinding {
                    key: ":w".to_string(),
                    description: "Save file",
                },
                HelpBinding {
                    key: ":q".to_string(),
                    description: "Quit",
                },
                HelpBinding {
                    key: ":wq".to_string(),
                    description: "Save and quit",
                },
                HelpBinding {
                    key: ":q!".to_string(),
                    description: "Force quit",
                },
                HelpBinding {
                    key: ":help".to_string(),
                    description: "Show this help",
                },
                binding(map, Action::ShowHelp, "Show this help"),
            ],
        },
    ]
}

/// Creates a help binding for a single action.
fn binding(map: &KeybindingMap, action: Action, description: &'static str) -> HelpBinding {
    HelpBinding {
        key: map.display_keys_for(action),
        description,
    }
}

/// Creates a help binding for a pair of related actions (e.g., next/prev).
///
/// Displays as "key1/key2" where key1 is the first action's primary key
/// and key2 is the second action's primary key.
fn paired_binding(
    map: &KeybindingMap,
    first: Action,
    second: Action,
    description: &'static str,
) -> HelpBinding {
    let first_key = map.display_primary_key_for(first);
    let second_key = map.display_primary_key_for(second);
    HelpBinding {
        key: format!("{first_key}/{second_key}"),
        description,
    }
}

/// Returns the total number of display lines a set of help sections produces.
///
/// Includes section headers, bindings, blank separators, and the
/// closing hint line.
#[must_use]
pub fn help_line_count(sections: &[HelpSection]) -> usize {
    let mut count = 0;
    for (i, section) in sections.iter().enumerate() {
        if i > 0 {
            count += 1; // blank separator
        }
        count += 1; // section header
        count += section.bindings.len();
    }
    count += 2; // trailing blank + hint line
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_sections_not_empty() {
        let map = KeybindingMap::defaults();
        let sections = build_help_sections(&map);
        assert!(!sections.is_empty());
        for section in &sections {
            assert!(!section.title.is_empty());
            assert!(!section.bindings.is_empty());
        }
    }

    #[test]
    fn all_bindings_have_content() {
        let map = KeybindingMap::defaults();
        let sections = build_help_sections(&map);
        for section in &sections {
            for binding in &section.bindings {
                assert!(
                    !binding.key.is_empty(),
                    "empty key for: {}",
                    binding.description
                );
                assert!(!binding.description.is_empty());
            }
        }
    }

    #[test]
    fn help_line_count_positive() {
        let map = KeybindingMap::defaults();
        let sections = build_help_sections(&map);
        let count = help_line_count(&sections);
        assert!(count > 0);
    }

    #[test]
    fn custom_bindings_reflected_in_help() {
        use std::collections::BTreeMap;

        let mut overrides = BTreeMap::new();
        overrides.insert("resolve_left".to_string(), vec!["a".to_string()]);
        let (map, _) = crate::keybindings::build_from_config(&overrides).unwrap();
        let sections = build_help_sections(&map);

        // Find the Resolution section and check that 'a' appears
        let resolution = sections.iter().find(|s| s.title == "Resolution").unwrap();
        let left_binding = resolution
            .bindings
            .iter()
            .find(|b| b.description == "Accept ours (left)")
            .unwrap();
        assert_eq!(left_binding.key, "a");
    }
}
