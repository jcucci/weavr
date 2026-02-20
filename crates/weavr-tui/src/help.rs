//! Structured help content for the help overlay.
//!
//! Provides data-driven help sections and keybinding descriptions
//! used by the help dialog renderer.

/// A single keybinding entry.
pub struct HelpBinding {
    /// The key or key combination (e.g., "o", "Ctrl+d").
    pub key: &'static str,
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

/// Returns the default help sections with all built-in keybindings.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn default_help_sections() -> Vec<HelpSection> {
    vec![
        HelpSection {
            title: "Resolution",
            bindings: vec![
                HelpBinding {
                    key: "o",
                    description: "Accept ours (left)",
                },
                HelpBinding {
                    key: "t",
                    description: "Accept theirs (right)",
                },
                HelpBinding {
                    key: "b",
                    description: "Accept both (default)",
                },
                HelpBinding {
                    key: "B",
                    description: "Accept both (options)",
                },
                HelpBinding {
                    key: "e",
                    description: "Edit in $EDITOR",
                },
                HelpBinding {
                    key: "x",
                    description: "Clear resolution",
                },
                HelpBinding {
                    key: "u",
                    description: "Undo last action",
                },
                HelpBinding {
                    key: "Ctrl+r",
                    description: "Redo last action",
                },
            ],
        },
        HelpSection {
            title: "Navigation",
            bindings: vec![
                HelpBinding {
                    key: "j/k",
                    description: "Next/prev hunk",
                },
                HelpBinding {
                    key: "n/N",
                    description: "Next/prev unresolved",
                },
                HelpBinding {
                    key: "gg/G",
                    description: "First/last hunk",
                },
                HelpBinding {
                    key: "Tab",
                    description: "Cycle panes",
                },
                HelpBinding {
                    key: "Enter",
                    description: "Focus result pane",
                },
            ],
        },
        HelpSection {
            title: "Scrolling",
            bindings: vec![
                HelpBinding {
                    key: "Ctrl+d",
                    description: "Scroll down",
                },
                HelpBinding {
                    key: "Ctrl+u",
                    description: "Scroll up",
                },
                HelpBinding {
                    key: "PgDn",
                    description: "Page down",
                },
                HelpBinding {
                    key: "PgUp",
                    description: "Page up",
                },
            ],
        },
        HelpSection {
            title: "AI (when configured)",
            bindings: vec![
                HelpBinding {
                    key: "s",
                    description: "AI suggest (current hunk)",
                },
                HelpBinding {
                    key: "S",
                    description: "AI suggest (all unresolved)",
                },
                HelpBinding {
                    key: "?",
                    description: "AI explain (when suggestion shown)",
                },
                HelpBinding {
                    key: "Enter",
                    description: "Accept AI suggestion",
                },
                HelpBinding {
                    key: "Esc",
                    description: "Dismiss AI suggestion",
                },
            ],
        },
        HelpSection {
            title: "Commands",
            bindings: vec![
                HelpBinding {
                    key: ":w",
                    description: "Save file",
                },
                HelpBinding {
                    key: ":q",
                    description: "Quit",
                },
                HelpBinding {
                    key: ":wq",
                    description: "Save and quit",
                },
                HelpBinding {
                    key: ":q!",
                    description: "Force quit",
                },
                HelpBinding {
                    key: ":help",
                    description: "Show this help",
                },
                HelpBinding {
                    key: "F1",
                    description: "Show this help",
                },
            ],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_sections_not_empty() {
        let sections = default_help_sections();
        assert!(!sections.is_empty());
        for section in &sections {
            assert!(!section.title.is_empty());
            assert!(!section.bindings.is_empty());
        }
    }

    #[test]
    fn all_bindings_have_content() {
        let sections = default_help_sections();
        for section in &sections {
            for binding in &section.bindings {
                assert!(!binding.key.is_empty());
                assert!(!binding.description.is_empty());
            }
        }
    }
}
