//! One Dark theme.
//!
//! A dark theme based on Atom's One Dark theme.
//! <https://github.com/atom/atom/tree/master/packages/one-dark-syntax>

use ratatui::style::{Color, Style};

use crate::theme::types::{ColorPalette, ConflictColors, DiffColors, Theme, UiColors};

// One Dark palette
const BG: Color = Color::Rgb(40, 44, 52);
const BG_LIGHT: Color = Color::Rgb(50, 56, 66);
const FG: Color = Color::Rgb(171, 178, 191);
const COMMENT: Color = Color::Rgb(92, 99, 112);
const CYAN: Color = Color::Rgb(86, 182, 194);
const BLUE: Color = Color::Rgb(97, 175, 239);
#[allow(dead_code)]
const PURPLE: Color = Color::Rgb(198, 120, 221);
const GREEN: Color = Color::Rgb(152, 195, 121);
const RED: Color = Color::Rgb(224, 108, 117);
const ORANGE: Color = Color::Rgb(209, 154, 102);
const YELLOW: Color = Color::Rgb(229, 192, 123);

/// Creates the One Dark theme.
#[must_use]
pub fn theme() -> Theme {
    let base = ColorPalette::new(BG, FG, COMMENT, YELLOW, BLUE);

    let conflict = ConflictColors::new(
        Style::default().fg(BLUE),
        Style::default().fg(ORANGE),
        Style::default().fg(GREEN),
        Style::default().fg(RED),
        Style::default().fg(GREEN),
    );

    let diff = DiffColors::new(
        Style::default().fg(GREEN).bg(BG_LIGHT),
        Style::default().fg(RED).bg(BG_LIGHT),
        Style::default().fg(YELLOW).bg(BG_LIGHT),
        Style::default().fg(COMMENT),
    );

    let ui = UiColors::new(
        YELLOW,
        BG_LIGHT,
        Style::default().fg(CYAN),
        Style::default().fg(COMMENT),
        Style::default().fg(FG).bg(BG_LIGHT),
    );

    Theme::new(base, conflict, diff, ui)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_dark_creates_theme() {
        let theme = theme();
        assert_eq!(theme.base.background, BG);
        assert_eq!(theme.ui.border_focused, YELLOW);
    }
}
