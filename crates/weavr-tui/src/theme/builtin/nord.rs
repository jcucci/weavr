//! Nord theme.
//!
//! An arctic, north-bluish color palette.
//! <https://www.nordtheme.com/>

use ratatui::style::{Color, Style};

use crate::theme::types::{ColorPalette, ConflictColors, DiffColors, Theme, UiColors};

// Nord Polar Night
const NORD0: Color = Color::Rgb(46, 52, 64);
const NORD1: Color = Color::Rgb(59, 66, 82);
const NORD2: Color = Color::Rgb(67, 76, 94);
const NORD3: Color = Color::Rgb(76, 86, 106);

// Nord Snow Storm
const NORD4: Color = Color::Rgb(216, 222, 233);
#[allow(dead_code)]
const NORD5: Color = Color::Rgb(229, 233, 240);
const NORD6: Color = Color::Rgb(236, 239, 244);

// Nord Frost
#[allow(dead_code)]
const NORD7: Color = Color::Rgb(143, 188, 187);
const NORD8: Color = Color::Rgb(136, 192, 208);
#[allow(dead_code)]
const NORD9: Color = Color::Rgb(129, 161, 193);
#[allow(dead_code)]
const NORD10: Color = Color::Rgb(94, 129, 172);

// Nord Aurora
const NORD11: Color = Color::Rgb(191, 97, 106); // Red
const NORD12: Color = Color::Rgb(208, 135, 112); // Orange
const NORD13: Color = Color::Rgb(235, 203, 139); // Yellow
const NORD14: Color = Color::Rgb(163, 190, 140); // Green
#[allow(dead_code)]
const NORD15: Color = Color::Rgb(180, 142, 173); // Purple

/// Creates the Nord theme.
#[must_use]
pub fn theme() -> Theme {
    let base = ColorPalette::new(
        NORD0,  // background
        NORD4,  // foreground
        NORD3,  // muted
        NORD13, // accent (yellow)
        NORD8,  // secondary (frost blue)
    );

    let conflict = ConflictColors::new(
        Style::default().fg(NORD8),  // left - frost blue
        Style::default().fg(NORD12), // right - orange
        Style::default().fg(NORD14), // both - green
        Style::default().fg(NORD11), // unresolved - red
        Style::default().fg(NORD14), // resolved - green
    );

    let diff = DiffColors::new(
        Style::default().fg(NORD14).bg(NORD1),
        Style::default().fg(NORD11).bg(NORD1),
        Style::default().fg(NORD13).bg(NORD1),
        Style::default().fg(NORD3),
    );

    let ui = UiColors::new(
        NORD13, // border_focused (yellow)
        NORD2,  // border_unfocused
        Style::default().fg(NORD8),
        Style::default().fg(NORD3),
        Style::default().fg(NORD6).bg(NORD2),
    );

    Theme::new(base, conflict, diff, ui)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nord_creates_theme() {
        let theme = theme();
        assert_eq!(theme.base.background, NORD0);
        assert_eq!(theme.ui.border_focused, NORD13);
    }
}
