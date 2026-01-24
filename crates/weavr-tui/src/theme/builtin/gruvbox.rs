//! Gruvbox themes.
//!
//! A retro groove color scheme with dark and light variants.
//! <https://github.com/morhetz/gruvbox>

use ratatui::style::{Color, Style};

use crate::theme::types::{ColorPalette, ConflictColors, DiffColors, Theme, UiColors};

// Gruvbox dark palette
#[allow(dead_code)]
mod dark_colors {
    use ratatui::style::Color;

    pub const BG: Color = Color::Rgb(40, 40, 40);
    pub const BG1: Color = Color::Rgb(60, 56, 54);
    pub const FG: Color = Color::Rgb(235, 219, 178);
    pub const GRAY: Color = Color::Rgb(146, 131, 116);
    pub const RED: Color = Color::Rgb(251, 73, 52);
    pub const GREEN: Color = Color::Rgb(184, 187, 38);
    pub const YELLOW: Color = Color::Rgb(250, 189, 47);
    pub const BLUE: Color = Color::Rgb(131, 165, 152);
    pub const PURPLE: Color = Color::Rgb(211, 134, 155);
    pub const AQUA: Color = Color::Rgb(142, 192, 124);
    pub const ORANGE: Color = Color::Rgb(254, 128, 25);
}

// Gruvbox light palette
#[allow(dead_code)]
mod light_colors {
    use ratatui::style::Color;

    pub const BG: Color = Color::Rgb(251, 241, 199);
    pub const BG1: Color = Color::Rgb(235, 219, 178);
    pub const FG: Color = Color::Rgb(60, 56, 54);
    pub const GRAY: Color = Color::Rgb(146, 131, 116);
    pub const RED: Color = Color::Rgb(204, 36, 29);
    pub const GREEN: Color = Color::Rgb(152, 151, 26);
    pub const YELLOW: Color = Color::Rgb(215, 153, 33);
    pub const BLUE: Color = Color::Rgb(69, 133, 136);
    pub const PURPLE: Color = Color::Rgb(177, 98, 134);
    pub const AQUA: Color = Color::Rgb(104, 157, 106);
    pub const ORANGE: Color = Color::Rgb(214, 93, 14);
}

/// Creates the Gruvbox dark theme.
#[must_use]
#[allow(clippy::wildcard_imports)]
pub fn dark() -> Theme {
    use dark_colors::*;

    let base = ColorPalette::new(BG, FG, GRAY, YELLOW, BLUE);

    let conflict = ConflictColors::new(
        Style::default().fg(BLUE),
        Style::default().fg(ORANGE),
        Style::default().fg(GREEN),
        Style::default().fg(RED),
        Style::default().fg(GREEN),
    );

    let diff = DiffColors::new(
        Style::default().fg(GREEN).bg(Color::Rgb(50, 55, 40)),
        Style::default().fg(RED).bg(Color::Rgb(60, 45, 45)),
        Style::default().fg(YELLOW).bg(Color::Rgb(55, 50, 35)),
        Style::default().fg(GRAY),
    );

    let ui = UiColors::new(
        YELLOW,
        BG1,
        Style::default().fg(AQUA),
        Style::default().fg(GRAY),
        Style::default().fg(FG).bg(BG1),
    );

    Theme::new(base, conflict, diff, ui)
}

/// Creates the Gruvbox light theme.
#[must_use]
#[allow(clippy::wildcard_imports)]
pub fn light() -> Theme {
    use light_colors::*;

    let base = ColorPalette::new(BG, FG, GRAY, YELLOW, BLUE);

    let conflict = ConflictColors::new(
        Style::default().fg(BLUE),
        Style::default().fg(ORANGE),
        Style::default().fg(GREEN),
        Style::default().fg(RED),
        Style::default().fg(GREEN),
    );

    let diff = DiffColors::new(
        Style::default().fg(GREEN).bg(Color::Rgb(230, 240, 220)),
        Style::default().fg(RED).bg(Color::Rgb(250, 220, 220)),
        Style::default().fg(YELLOW).bg(Color::Rgb(250, 245, 210)),
        Style::default().fg(GRAY),
    );

    let ui = UiColors::new(
        YELLOW,
        BG1,
        Style::default().fg(AQUA),
        Style::default().fg(GRAY),
        Style::default().fg(FG).bg(BG1),
    );

    Theme::new(base, conflict, diff, ui)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gruvbox_dark_creates_theme() {
        let theme = dark();
        assert_eq!(theme.base.background, dark_colors::BG);
    }

    #[test]
    fn gruvbox_light_creates_theme() {
        let theme = light();
        assert_eq!(theme.base.background, light_colors::BG);
    }
}
