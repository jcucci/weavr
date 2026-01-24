//! Theme type definitions.
//!
//! This module defines the core types for theming the TUI.

use ratatui::style::{Color, Style};

/// A complete theme configuration for the TUI.
#[derive(Debug, Clone)]
pub struct Theme {
    /// Base color palette.
    pub base: ColorPalette,
    /// Colors for conflict visualization.
    pub conflict: ConflictColors,
    /// Colors for diff visualization.
    pub diff: DiffColors,
    /// Colors for UI elements.
    pub ui: UiColors,
}

/// Base color palette used throughout the UI.
#[derive(Debug, Clone, Copy)]
pub struct ColorPalette {
    /// Primary background color.
    pub background: Color,
    /// Primary foreground (text) color.
    pub foreground: Color,
    /// Muted/dimmed text color.
    pub muted: Color,
    /// Accent color for highlights.
    pub accent: Color,
    /// Secondary accent color.
    pub secondary: Color,
}

/// Colors for conflict visualization.
#[derive(Debug, Clone, Copy)]
pub struct ConflictColors {
    /// Style for left (ours) side.
    pub left: Style,
    /// Style for right (theirs) side.
    pub right: Style,
    /// Style for both sides (merged content).
    pub both: Style,
    /// Style for unresolved conflicts.
    pub unresolved: Style,
    /// Style for resolved conflicts.
    pub resolved: Style,
}

/// Colors for diff visualization.
#[derive(Debug, Clone, Copy)]
pub struct DiffColors {
    /// Style for added lines.
    pub added: Style,
    /// Style for removed lines.
    pub removed: Style,
    /// Style for modified lines.
    pub modified: Style,
    /// Style for context lines.
    pub context: Style,
}

/// Colors for UI elements.
#[derive(Debug, Clone, Copy)]
pub struct UiColors {
    /// Border color when focused.
    pub border_focused: Color,
    /// Border color when unfocused.
    pub border_unfocused: Color,
    /// Title bar style.
    pub title: Style,
    /// Status bar style.
    pub status: Style,
    /// Selection highlight style.
    pub selection: Style,
}

impl Theme {
    /// Creates a new theme with the given components.
    #[must_use]
    pub const fn new(
        base: ColorPalette,
        conflict: ConflictColors,
        diff: DiffColors,
        ui: UiColors,
    ) -> Self {
        Self {
            base,
            conflict,
            diff,
            ui,
        }
    }
}

impl ColorPalette {
    /// Creates a new color palette.
    #[must_use]
    pub const fn new(
        background: Color,
        foreground: Color,
        muted: Color,
        accent: Color,
        secondary: Color,
    ) -> Self {
        Self {
            background,
            foreground,
            muted,
            accent,
            secondary,
        }
    }
}

impl ConflictColors {
    /// Creates new conflict colors.
    #[must_use]
    pub const fn new(
        left: Style,
        right: Style,
        both: Style,
        unresolved: Style,
        resolved: Style,
    ) -> Self {
        Self {
            left,
            right,
            both,
            unresolved,
            resolved,
        }
    }
}

impl DiffColors {
    /// Creates new diff colors.
    #[must_use]
    pub const fn new(added: Style, removed: Style, modified: Style, context: Style) -> Self {
        Self {
            added,
            removed,
            modified,
            context,
        }
    }
}

impl UiColors {
    /// Creates new UI colors.
    #[must_use]
    pub const fn new(
        border_focused: Color,
        border_unfocused: Color,
        title: Style,
        status: Style,
        selection: Style,
    ) -> Self {
        Self {
            border_focused,
            border_unfocused,
            title,
            status,
            selection,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_palette_construction() {
        let palette = ColorPalette::new(
            Color::Black,
            Color::White,
            Color::Gray,
            Color::Yellow,
            Color::Cyan,
        );
        assert_eq!(palette.background, Color::Black);
        assert_eq!(palette.foreground, Color::White);
    }

    #[test]
    fn theme_construction() {
        let base = ColorPalette::new(
            Color::Black,
            Color::White,
            Color::Gray,
            Color::Yellow,
            Color::Cyan,
        );
        let conflict = ConflictColors::new(
            Style::default(),
            Style::default(),
            Style::default(),
            Style::default(),
            Style::default(),
        );
        let diff = DiffColors::new(
            Style::default(),
            Style::default(),
            Style::default(),
            Style::default(),
        );
        let ui = UiColors::new(
            Color::Yellow,
            Color::Gray,
            Style::default(),
            Style::default(),
            Style::default(),
        );
        let theme = Theme::new(base, conflict, diff, ui);
        assert_eq!(theme.base.background, Color::Black);
    }
}
