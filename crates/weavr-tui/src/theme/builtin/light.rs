//! Default light theme.

use ratatui::style::{Color, Style};

use crate::theme::types::{ColorPalette, ConflictColors, DiffColors, Theme, UiColors};

/// Creates the default light theme.
#[must_use]
pub fn theme() -> Theme {
    let base = ColorPalette::new(
        Color::Rgb(250, 250, 250), // background
        Color::Rgb(30, 30, 30),    // foreground
        Color::Rgb(128, 128, 128), // muted
        Color::Rgb(180, 130, 0),   // accent (dark gold)
        Color::Rgb(50, 100, 180),  // secondary (blue)
    );

    let conflict = ConflictColors::new(
        Style::default().fg(Color::Rgb(0, 100, 200)), // left - blue
        Style::default().fg(Color::Rgb(200, 100, 0)), // right - orange
        Style::default().fg(Color::Rgb(0, 150, 0)),   // both - green
        Style::default().fg(Color::Rgb(200, 50, 50)), // unresolved - red
        Style::default().fg(Color::Rgb(50, 150, 50)), // resolved - green
    );

    let diff = DiffColors::new(
        Style::default()
            .fg(Color::Rgb(0, 100, 0))
            .bg(Color::Rgb(220, 255, 220)), // added
        Style::default()
            .fg(Color::Rgb(150, 0, 0))
            .bg(Color::Rgb(255, 220, 220)), // removed
        Style::default()
            .fg(Color::Rgb(150, 100, 0))
            .bg(Color::Rgb(255, 250, 200)), // modified
        Style::default().fg(Color::Rgb(80, 80, 80)), // context
    );

    let ui = UiColors::new(
        Color::Rgb(180, 130, 0),                        // border_focused
        Color::Rgb(180, 180, 180),                      // border_unfocused
        Style::default().fg(Color::Rgb(0, 120, 180)),   // title
        Style::default().fg(Color::Rgb(128, 128, 128)), // status
        Style::default()
            .fg(Color::Rgb(0, 0, 0))
            .bg(Color::Rgb(200, 220, 255)), // selection
    );

    Theme::new(base, conflict, diff, ui)
}
