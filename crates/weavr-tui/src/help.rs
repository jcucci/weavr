//! Structured help content for the help overlay.
//!
//! Provides data-driven help sections and keybinding descriptions
//! used by the help dialog renderer.

use std::sync::OnceLock;

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
    pub bindings: &'static [HelpBinding],
}

static HELP_SECTIONS: OnceLock<Vec<HelpSection>> = OnceLock::new();

#[allow(clippy::too_many_lines)]
fn build_help_sections() -> Vec<HelpSection> {
    vec![
        HelpSection {
            title: "Resolution",
            bindings: &[
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
            bindings: &[
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
            bindings: &[
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
            bindings: &[
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
                    description: "Help / AI explain (when suggestion shown)",
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
            bindings: &[
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

/// Returns the default help sections with all built-in keybindings.
///
/// This is backed by a `OnceLock` so repeated calls (e.g., per-frame
/// rendering) do not allocate.
#[must_use]
pub fn default_help_sections() -> &'static [HelpSection] {
    HELP_SECTIONS.get_or_init(build_help_sections)
}

/// Returns the total number of display lines the help sections produce.
///
/// Includes section headers, bindings, blank separators, and the
/// closing hint line.
#[must_use]
pub fn help_line_count() -> usize {
    let sections = default_help_sections();
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
        let sections = default_help_sections();
        assert!(!sections.is_empty());
        for section in sections {
            assert!(!section.title.is_empty());
            assert!(!section.bindings.is_empty());
        }
    }

    #[test]
    fn all_bindings_have_content() {
        let sections = default_help_sections();
        for section in sections {
            for binding in section.bindings {
                assert!(!binding.key.is_empty());
                assert!(!binding.description.is_empty());
            }
        }
    }

    #[test]
    fn help_line_count_matches_sections() {
        let count = help_line_count();
        // Should be positive and consistent
        assert!(count > 0);
        assert_eq!(count, help_line_count());
    }

    #[test]
    fn repeated_calls_return_same_reference() {
        let a = default_help_sections();
        let b = default_help_sections();
        assert!(std::ptr::eq(a, b));
    }
}
