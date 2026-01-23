//! Development entry point for weavr-tui.
//!
//! This binary is for development and testing purposes.
//! Production use will be through weavr-cli.

use std::io;
use std::time::Duration;

use ratatui::DefaultTerminal;
use weavr_tui::{event, ui, App};

fn main() -> io::Result<()> {
    // ratatui::init() sets up terminal and panic hook automatically
    let mut terminal = ratatui::init();

    let mut app = App::new();
    let result = run(&mut terminal, &mut app);

    // Always restore terminal, even if run() failed
    ratatui::restore();

    result
}

/// Main event loop.
fn run(terminal: &mut DefaultTerminal, app: &mut App) -> io::Result<()> {
    while !app.should_quit() {
        terminal.draw(|frame| ui::draw(frame, app))?;

        if let Some(evt) = event::poll_event(Duration::from_millis(100))? {
            event::handle_event(app, &evt);
        }
    }

    Ok(())
}
