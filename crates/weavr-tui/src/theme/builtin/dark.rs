//! Default dark theme.

use ratatui::style::{Color, Style};

use crate::theme::types::{ColorPalette, ConflictColors, DiffColors, Theme, UiColors};

/// Creates the default dark theme.
#[must_use]
pub fn theme() -> Theme {
    let base = ColorPalette::new(
        Color::Rgb(30, 30, 30),    // background
        Color::Rgb(220, 220, 220), // foreground
        Color::Rgb(128, 128, 128), // muted
        Color::Rgb(255, 215, 0),   // accent (gold)
        Color::Rgb(100, 149, 237), // secondary (cornflower blue)
    );

    let conflict = ConflictColors::new(
        Style::default().fg(Color::Rgb(100, 180, 255)), // left - blue
        Style::default().fg(Color::Rgb(255, 150, 100)), // right - orange
        Style::default().fg(Color::Rgb(150, 220, 150)), // both - green
        Style::default().fg(Color::Rgb(255, 100, 100)), // unresolved - red
        Style::default().fg(Color::Rgb(100, 200, 100)), // resolved - bright green
    );

    let diff = DiffColors::new(
        Style::default()
            .fg(Color::Rgb(150, 255, 150))
            .bg(Color::Rgb(40, 60, 40)), // added
        Style::default()
            .fg(Color::Rgb(255, 150, 150))
            .bg(Color::Rgb(60, 40, 40)), // removed
        Style::default()
            .fg(Color::Rgb(255, 220, 100))
            .bg(Color::Rgb(60, 55, 40)), // modified
        Style::default().fg(Color::Rgb(180, 180, 180)), // context
    );

    let ui = UiColors::new(
        Color::Rgb(255, 215, 0),                        // border_focused (gold)
        Color::Rgb(80, 80, 80),                         // border_unfocused
        Style::default().fg(Color::Rgb(100, 200, 255)), // title
        Style::default().fg(Color::Rgb(128, 128, 128)), // status
        Style::default()
            .fg(Color::Rgb(255, 255, 255))
            .bg(Color::Rgb(70, 70, 70)), // selection
    );

    Theme::new(base, conflict, diff, ui)
}
