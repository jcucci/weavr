//! Solarized themes.
//!
//! Precision colors for machines and people.
//! <https://ethanschoonover.com/solarized/>

use ratatui::style::{Color, Style};

use crate::theme::types::{ColorPalette, ConflictColors, DiffColors, Theme, UiColors};

// Solarized base colors
const BASE03: Color = Color::Rgb(0, 43, 54);
const BASE02: Color = Color::Rgb(7, 54, 66);
const BASE01: Color = Color::Rgb(88, 110, 117);
const BASE00: Color = Color::Rgb(101, 123, 131);
const BASE0: Color = Color::Rgb(131, 148, 150);
const BASE1: Color = Color::Rgb(147, 161, 161);
const BASE2: Color = Color::Rgb(238, 232, 213);
const BASE3: Color = Color::Rgb(253, 246, 227);

// Solarized accent colors
const YELLOW: Color = Color::Rgb(181, 137, 0);
const ORANGE: Color = Color::Rgb(203, 75, 22);
const RED: Color = Color::Rgb(220, 50, 47);
#[allow(dead_code)]
const MAGENTA: Color = Color::Rgb(211, 54, 130);
#[allow(dead_code)]
const VIOLET: Color = Color::Rgb(108, 113, 196);
const BLUE: Color = Color::Rgb(38, 139, 210);
const CYAN: Color = Color::Rgb(42, 161, 152);
const GREEN: Color = Color::Rgb(133, 153, 0);

/// Creates the Solarized Dark theme.
#[must_use]
pub fn dark() -> Theme {
    let base = ColorPalette::new(
        BASE03, // background
        BASE0,  // foreground
        BASE01, // muted
        YELLOW, // accent
        BLUE,   // secondary
    );

    let conflict = ConflictColors::new(
        Style::default().fg(BLUE),
        Style::default().fg(ORANGE),
        Style::default().fg(GREEN),
        Style::default().fg(RED),
        Style::default().fg(GREEN),
    );

    let diff = DiffColors::new(
        Style::default().fg(GREEN).bg(BASE02),
        Style::default().fg(RED).bg(BASE02),
        Style::default().fg(YELLOW).bg(BASE02),
        Style::default().fg(BASE01),
    );

    let ui = UiColors::new(
        YELLOW,
        BASE02,
        Style::default().fg(CYAN),
        Style::default().fg(BASE01),
        Style::default().fg(BASE0).bg(BASE02),
    );

    Theme::new(base, conflict, diff, ui)
}

/// Creates the Solarized Light theme.
#[must_use]
pub fn light() -> Theme {
    let base = ColorPalette::new(
        BASE3,  // background
        BASE00, // foreground
        BASE1,  // muted
        YELLOW, // accent
        BLUE,   // secondary
    );

    let conflict = ConflictColors::new(
        Style::default().fg(BLUE),
        Style::default().fg(ORANGE),
        Style::default().fg(GREEN),
        Style::default().fg(RED),
        Style::default().fg(GREEN),
    );

    let diff = DiffColors::new(
        Style::default().fg(GREEN).bg(BASE2),
        Style::default().fg(RED).bg(BASE2),
        Style::default().fg(YELLOW).bg(BASE2),
        Style::default().fg(BASE1),
    );

    let ui = UiColors::new(
        YELLOW,
        BASE2,
        Style::default().fg(CYAN),
        Style::default().fg(BASE1),
        Style::default().fg(BASE00).bg(BASE2),
    );

    Theme::new(base, conflict, diff, ui)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solarized_dark_creates_theme() {
        let theme = dark();
        assert_eq!(theme.base.background, BASE03);
    }

    #[test]
    fn solarized_light_creates_theme() {
        let theme = light();
        assert_eq!(theme.base.background, BASE3);
    }
}
