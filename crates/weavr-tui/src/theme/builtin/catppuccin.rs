//! Catppuccin themes.
//!
//! Catppuccin is a community-driven pastel theme with four flavors:
//! - Latte (light)
//! - Frappe (medium dark)
//! - Macchiato (dark)
//! - Mocha (darkest)

use catppuccin::{Flavor, PALETTE};
use ratatui::style::{Color, Style};

use crate::theme::types::{ColorPalette, ConflictColors, DiffColors, Theme, UiColors};

/// Creates the Catppuccin Latte theme (light).
#[must_use]
pub fn latte() -> Theme {
    from_flavor(&PALETTE.latte)
}

/// Creates the Catppuccin Frappe theme (medium dark).
#[must_use]
pub fn frappe() -> Theme {
    from_flavor(&PALETTE.frappe)
}

/// Creates the Catppuccin Macchiato theme (dark).
#[must_use]
pub fn macchiato() -> Theme {
    from_flavor(&PALETTE.macchiato)
}

/// Creates the Catppuccin Mocha theme (darkest).
#[must_use]
pub fn mocha() -> Theme {
    from_flavor(&PALETTE.mocha)
}

fn from_flavor(flavor: &Flavor) -> Theme {
    let c = &flavor.colors;

    let base = ColorPalette::new(
        Color::from(c.base),
        Color::from(c.text),
        Color::from(c.overlay0),
        Color::from(c.yellow),
        Color::from(c.blue),
    );

    let conflict = ConflictColors::new(
        Style::default().fg(Color::from(c.blue)),
        Style::default().fg(Color::from(c.peach)),
        Style::default().fg(Color::from(c.green)),
        Style::default().fg(Color::from(c.red)),
        Style::default().fg(Color::from(c.green)),
    );

    let diff = DiffColors::new(
        Style::default()
            .fg(Color::from(c.green))
            .bg(Color::from(c.surface0)),
        Style::default()
            .fg(Color::from(c.red))
            .bg(Color::from(c.surface0)),
        Style::default()
            .fg(Color::from(c.yellow))
            .bg(Color::from(c.surface0)),
        Style::default().fg(Color::from(c.subtext0)),
    );

    let ui = UiColors::new(
        Color::from(c.yellow),
        Color::from(c.surface1),
        Style::default().fg(Color::from(c.sapphire)),
        Style::default().fg(Color::from(c.overlay0)),
        Style::default()
            .fg(Color::from(c.text))
            .bg(Color::from(c.surface1)),
    );

    Theme::new(base, conflict, diff, ui)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latte_creates_theme() {
        let theme = latte();
        assert_eq!(
            theme.ui.border_focused,
            Color::from(PALETTE.latte.colors.yellow)
        );
    }

    #[test]
    fn mocha_creates_theme() {
        let theme = mocha();
        assert_eq!(
            theme.ui.border_focused,
            Color::from(PALETTE.mocha.colors.yellow)
        );
    }
}
