//! Dracula theme.
//!
//! A dark theme based on the Dracula color scheme.
//! <https://draculatheme.com/>

use ratatui::style::{Color, Style};

use crate::theme::types::{ColorPalette, ConflictColors, DiffColors, Theme, UiColors};

// Dracula color palette
const BACKGROUND: Color = Color::Rgb(40, 42, 54);
const FOREGROUND: Color = Color::Rgb(248, 248, 242);
const CURRENT_LINE: Color = Color::Rgb(68, 71, 90);
const COMMENT: Color = Color::Rgb(98, 114, 164);
const CYAN: Color = Color::Rgb(139, 233, 253);
const GREEN: Color = Color::Rgb(80, 250, 123);
const ORANGE: Color = Color::Rgb(255, 184, 108);
const PINK: Color = Color::Rgb(255, 121, 198);
const PURPLE: Color = Color::Rgb(189, 147, 249);
const RED: Color = Color::Rgb(255, 85, 85);
const YELLOW: Color = Color::Rgb(241, 250, 140);

/// Creates the Dracula theme.
#[must_use]
pub fn theme() -> Theme {
    let base = ColorPalette::new(BACKGROUND, FOREGROUND, COMMENT, PURPLE, CYAN);

    let conflict = ConflictColors::new(
        Style::default().fg(CYAN),
        Style::default().fg(ORANGE),
        Style::default().fg(GREEN),
        Style::default().fg(RED),
        Style::default().fg(GREEN),
    );

    let diff = DiffColors::new(
        Style::default().fg(GREEN).bg(Color::Rgb(50, 60, 50)),
        Style::default().fg(RED).bg(Color::Rgb(60, 45, 50)),
        Style::default().fg(YELLOW).bg(Color::Rgb(55, 55, 45)),
        Style::default().fg(COMMENT),
    );

    let ui = UiColors::new(
        PURPLE,
        CURRENT_LINE,
        Style::default().fg(PINK),
        Style::default().fg(COMMENT),
        Style::default().fg(FOREGROUND).bg(CURRENT_LINE),
    );

    Theme::new(base, conflict, diff, ui)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dracula_creates_theme() {
        let theme = theme();
        assert_eq!(theme.base.background, BACKGROUND);
        assert_eq!(theme.ui.border_focused, PURPLE);
    }
}
