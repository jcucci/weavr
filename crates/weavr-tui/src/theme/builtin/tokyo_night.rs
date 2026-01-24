//! Tokyo Night themes.
//!
//! A clean, dark theme inspired by Tokyo at night.
//! <https://github.com/folke/tokyonight.nvim>

use ratatui::style::Style;

use crate::theme::types::{ColorPalette, ConflictColors, DiffColors, Theme, UiColors};

// Tokyo Night (default) palette
#[allow(dead_code)]
mod night_colors {
    use ratatui::style::Color;

    pub const BG: Color = Color::Rgb(26, 27, 38);
    pub const BG_HIGHLIGHT: Color = Color::Rgb(41, 46, 66);
    pub const FG: Color = Color::Rgb(192, 202, 245);
    pub const COMMENT: Color = Color::Rgb(86, 95, 137);
    pub const CYAN: Color = Color::Rgb(125, 207, 255);
    pub const BLUE: Color = Color::Rgb(122, 162, 247);
    pub const PURPLE: Color = Color::Rgb(187, 154, 247);
    pub const ORANGE: Color = Color::Rgb(255, 158, 100);
    pub const YELLOW: Color = Color::Rgb(224, 175, 104);
    pub const GREEN: Color = Color::Rgb(158, 206, 106);
    pub const RED: Color = Color::Rgb(247, 118, 142);
}

// Tokyo Night Storm palette
mod storm_colors {
    use ratatui::style::Color;

    pub const BG: Color = Color::Rgb(36, 40, 59);
    pub const BG_HIGHLIGHT: Color = Color::Rgb(52, 59, 88);
    pub const FG: Color = Color::Rgb(192, 202, 245);
    pub const COMMENT: Color = Color::Rgb(86, 95, 137);
}

// Tokyo Night Light palette
#[allow(dead_code)]
mod light_colors {
    use ratatui::style::Color;

    pub const BG: Color = Color::Rgb(213, 214, 219);
    pub const BG_HIGHLIGHT: Color = Color::Rgb(193, 194, 199);
    pub const FG: Color = Color::Rgb(52, 59, 88);
    pub const COMMENT: Color = Color::Rgb(150, 153, 163);
    pub const CYAN: Color = Color::Rgb(11, 139, 150);
    pub const BLUE: Color = Color::Rgb(52, 84, 138);
    pub const PURPLE: Color = Color::Rgb(92, 65, 125);
    pub const ORANGE: Color = Color::Rgb(150, 75, 0);
    pub const YELLOW: Color = Color::Rgb(143, 99, 0);
    pub const GREEN: Color = Color::Rgb(56, 95, 35);
    pub const RED: Color = Color::Rgb(140, 65, 85);
}

/// Creates the Tokyo Night theme (default).
#[must_use]
#[allow(clippy::wildcard_imports)]
pub fn night() -> Theme {
    use night_colors::*;

    let base = ColorPalette::new(BG, FG, COMMENT, YELLOW, BLUE);

    let conflict = ConflictColors::new(
        Style::default().fg(BLUE),
        Style::default().fg(ORANGE),
        Style::default().fg(GREEN),
        Style::default().fg(RED),
        Style::default().fg(GREEN),
    );

    let diff = DiffColors::new(
        Style::default().fg(GREEN).bg(BG_HIGHLIGHT),
        Style::default().fg(RED).bg(BG_HIGHLIGHT),
        Style::default().fg(YELLOW).bg(BG_HIGHLIGHT),
        Style::default().fg(COMMENT),
    );

    let ui = UiColors::new(
        YELLOW,
        BG_HIGHLIGHT,
        Style::default().fg(CYAN),
        Style::default().fg(COMMENT),
        Style::default().fg(FG).bg(BG_HIGHLIGHT),
    );

    Theme::new(base, conflict, diff, ui)
}

/// Creates the Tokyo Night Storm theme.
#[must_use]
#[allow(clippy::wildcard_imports)]
pub fn storm() -> Theme {
    use night_colors::*;
    use storm_colors::{BG, BG_HIGHLIGHT};

    let base = ColorPalette::new(BG, FG, storm_colors::COMMENT, YELLOW, BLUE);

    let conflict = ConflictColors::new(
        Style::default().fg(BLUE),
        Style::default().fg(ORANGE),
        Style::default().fg(GREEN),
        Style::default().fg(RED),
        Style::default().fg(GREEN),
    );

    let diff = DiffColors::new(
        Style::default().fg(GREEN).bg(BG_HIGHLIGHT),
        Style::default().fg(RED).bg(BG_HIGHLIGHT),
        Style::default().fg(YELLOW).bg(BG_HIGHLIGHT),
        Style::default().fg(storm_colors::COMMENT),
    );

    let ui = UiColors::new(
        YELLOW,
        BG_HIGHLIGHT,
        Style::default().fg(CYAN),
        Style::default().fg(storm_colors::COMMENT),
        Style::default().fg(storm_colors::FG).bg(BG_HIGHLIGHT),
    );

    Theme::new(base, conflict, diff, ui)
}

/// Creates the Tokyo Night Light theme.
#[must_use]
#[allow(clippy::wildcard_imports)]
pub fn light() -> Theme {
    use light_colors::*;

    let base = ColorPalette::new(BG, FG, COMMENT, YELLOW, BLUE);

    let conflict = ConflictColors::new(
        Style::default().fg(BLUE),
        Style::default().fg(ORANGE),
        Style::default().fg(GREEN),
        Style::default().fg(RED),
        Style::default().fg(GREEN),
    );

    let diff = DiffColors::new(
        Style::default().fg(GREEN).bg(BG_HIGHLIGHT),
        Style::default().fg(RED).bg(BG_HIGHLIGHT),
        Style::default().fg(YELLOW).bg(BG_HIGHLIGHT),
        Style::default().fg(COMMENT),
    );

    let ui = UiColors::new(
        YELLOW,
        BG_HIGHLIGHT,
        Style::default().fg(CYAN),
        Style::default().fg(COMMENT),
        Style::default().fg(FG).bg(BG_HIGHLIGHT),
    );

    Theme::new(base, conflict, diff, ui)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokyo_night_creates_theme() {
        let theme = night();
        assert_eq!(theme.base.background, night_colors::BG);
    }

    #[test]
    fn tokyo_storm_creates_theme() {
        let theme = storm();
        assert_eq!(theme.base.background, storm_colors::BG);
    }

    #[test]
    fn tokyo_light_creates_theme() {
        let theme = light();
        assert_eq!(theme.base.background, light_colors::BG);
    }
}
