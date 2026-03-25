//! Display configuration and visual state.
//!
//! This module holds theme, layout, diff, syntax highlighting,
//! and status message state for the TUI.

use std::time::Instant;

use crate::diff;
use crate::highlight;
use crate::theme::Theme;
use crate::LayoutConfig;

/// Display configuration and visual state.
pub(crate) struct DisplayState {
    /// The active theme.
    pub(crate) theme: Theme,
    /// Layout configuration.
    pub(crate) layout_config: LayoutConfig,
    /// Configuration for diff highlighting.
    pub(crate) diff_config: diff::DiffConfig,
    /// Whether the base (ancestor) pane is visible.
    pub(crate) show_base_pane: bool,
    /// Whether syntax highlighting is enabled.
    pub(crate) syntax_highlight: bool,
    /// Lazy-initialized syntax highlighter.
    pub(crate) highlighter: Option<highlight::Highlighter>,
    /// Cached highlighted documents for the current file.
    pub(crate) highlight_cache: Option<highlight::HighlightCache>,
    /// Status message to display (with timestamp for auto-clear).
    pub(crate) status_message: Option<(String, Instant)>,
}

impl Default for DisplayState {
    fn default() -> Self {
        Self {
            theme: Theme::from(crate::theme::ThemeName::default()),
            layout_config: LayoutConfig::default(),
            diff_config: diff::DiffConfig::default(),
            show_base_pane: false,
            syntax_highlight: true,
            highlighter: None,
            highlight_cache: None,
            status_message: None,
        }
    }
}
