//! Rose Pine themes.
//!
//! All natural pine, faux fur and a bit of soho vibes.
//! <https://rosepinetheme.com/>

use ratatui::style::Style;

use crate::theme::types::{ColorPalette, ConflictColors, DiffColors, Theme, UiColors};

// Rose Pine (default) palette
#[allow(dead_code)]
mod main_colors {
    use ratatui::style::Color;

    pub const BASE: Color = Color::Rgb(25, 23, 36);
    pub const SURFACE: Color = Color::Rgb(31, 29, 46);
    pub const OVERLAY: Color = Color::Rgb(38, 35, 58);
    pub const MUTED: Color = Color::Rgb(110, 106, 134);
    pub const SUBTLE: Color = Color::Rgb(144, 140, 170);
    pub const TEXT: Color = Color::Rgb(224, 222, 244);
    pub const LOVE: Color = Color::Rgb(235, 111, 146);
    pub const GOLD: Color = Color::Rgb(246, 193, 119);
    pub const ROSE: Color = Color::Rgb(234, 154, 151);
    pub const PINE: Color = Color::Rgb(49, 116, 143);
    pub const FOAM: Color = Color::Rgb(156, 207, 216);
    pub const IRIS: Color = Color::Rgb(196, 167, 231);
}

// Rose Pine Moon palette
#[allow(dead_code)]
mod moon_colors {
    use ratatui::style::Color;

    pub const BASE: Color = Color::Rgb(35, 33, 54);
    pub const SURFACE: Color = Color::Rgb(42, 39, 63);
    pub const OVERLAY: Color = Color::Rgb(57, 53, 82);
    pub const MUTED: Color = Color::Rgb(110, 106, 134);
    pub const SUBTLE: Color = Color::Rgb(144, 140, 170);
    pub const TEXT: Color = Color::Rgb(224, 222, 244);
    pub const LOVE: Color = Color::Rgb(235, 111, 146);
    pub const GOLD: Color = Color::Rgb(246, 193, 119);
    pub const ROSE: Color = Color::Rgb(234, 154, 151);
    pub const PINE: Color = Color::Rgb(62, 143, 176);
    pub const FOAM: Color = Color::Rgb(156, 207, 216);
    pub const IRIS: Color = Color::Rgb(196, 167, 231);
}

// Rose Pine Dawn palette
#[allow(dead_code)]
mod dawn_colors {
    use ratatui::style::Color;

    pub const BASE: Color = Color::Rgb(250, 244, 237);
    pub const SURFACE: Color = Color::Rgb(255, 250, 243);
    pub const OVERLAY: Color = Color::Rgb(242, 233, 222);
    pub const MUTED: Color = Color::Rgb(152, 147, 165);
    pub const SUBTLE: Color = Color::Rgb(121, 117, 147);
    pub const TEXT: Color = Color::Rgb(87, 82, 121);
    pub const LOVE: Color = Color::Rgb(180, 99, 122);
    pub const GOLD: Color = Color::Rgb(234, 157, 52);
    pub const ROSE: Color = Color::Rgb(215, 130, 126);
    pub const PINE: Color = Color::Rgb(40, 105, 131);
    pub const FOAM: Color = Color::Rgb(86, 148, 159);
    pub const IRIS: Color = Color::Rgb(144, 122, 169);
}

/// Creates the Rose Pine theme (default).
#[must_use]
#[allow(clippy::wildcard_imports)]
pub fn main() -> Theme {
    use main_colors::*;

    let base = ColorPalette::new(BASE, TEXT, MUTED, GOLD, FOAM);

    let conflict = ConflictColors::new(
        Style::default().fg(FOAM),
        Style::default().fg(ROSE),
        Style::default().fg(PINE),
        Style::default().fg(LOVE),
        Style::default().fg(PINE),
    );

    let diff = DiffColors::new(
        Style::default().fg(PINE).bg(SURFACE),
        Style::default().fg(LOVE).bg(SURFACE),
        Style::default().fg(GOLD).bg(SURFACE),
        Style::default().fg(MUTED),
    );

    let ui = UiColors::new(
        GOLD,
        OVERLAY,
        Style::default().fg(IRIS),
        Style::default().fg(MUTED),
        Style::default().fg(TEXT).bg(OVERLAY),
    );

    Theme::new(base, conflict, diff, ui)
}

/// Creates the Rose Pine Moon theme.
#[must_use]
#[allow(clippy::wildcard_imports)]
pub fn moon() -> Theme {
    use moon_colors::*;

    let base = ColorPalette::new(BASE, TEXT, MUTED, GOLD, FOAM);

    let conflict = ConflictColors::new(
        Style::default().fg(FOAM),
        Style::default().fg(ROSE),
        Style::default().fg(PINE),
        Style::default().fg(LOVE),
        Style::default().fg(PINE),
    );

    let diff = DiffColors::new(
        Style::default().fg(PINE).bg(SURFACE),
        Style::default().fg(LOVE).bg(SURFACE),
        Style::default().fg(GOLD).bg(SURFACE),
        Style::default().fg(MUTED),
    );

    let ui = UiColors::new(
        GOLD,
        OVERLAY,
        Style::default().fg(IRIS),
        Style::default().fg(MUTED),
        Style::default().fg(TEXT).bg(OVERLAY),
    );

    Theme::new(base, conflict, diff, ui)
}

/// Creates the Rose Pine Dawn theme (light).
#[must_use]
#[allow(clippy::wildcard_imports)]
pub fn dawn() -> Theme {
    use dawn_colors::*;

    let base = ColorPalette::new(BASE, TEXT, MUTED, GOLD, FOAM);

    let conflict = ConflictColors::new(
        Style::default().fg(FOAM),
        Style::default().fg(ROSE),
        Style::default().fg(PINE),
        Style::default().fg(LOVE),
        Style::default().fg(PINE),
    );

    let diff = DiffColors::new(
        Style::default().fg(PINE).bg(SURFACE),
        Style::default().fg(LOVE).bg(SURFACE),
        Style::default().fg(GOLD).bg(SURFACE),
        Style::default().fg(MUTED),
    );

    let ui = UiColors::new(
        GOLD,
        OVERLAY,
        Style::default().fg(IRIS),
        Style::default().fg(MUTED),
        Style::default().fg(TEXT).bg(OVERLAY),
    );

    Theme::new(base, conflict, diff, ui)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rose_pine_main_creates_theme() {
        let theme = main();
        assert_eq!(theme.base.background, main_colors::BASE);
    }

    #[test]
    fn rose_pine_moon_creates_theme() {
        let theme = moon();
        assert_eq!(theme.base.background, moon_colors::BASE);
    }

    #[test]
    fn rose_pine_dawn_creates_theme() {
        let theme = dawn();
        assert_eq!(theme.base.background, dawn_colors::BASE);
    }
}
