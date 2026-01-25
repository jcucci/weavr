//! Development entry point for weavr-tui.
//!
//! This binary is for development and testing purposes.
//! Production use will be through weavr-cli.

use std::io::{self, Write};
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
        // Check for pending editor (Phase 7)
        if let Some(content) = app.take_editor_pending() {
            // Suspend TUI
            ratatui::restore();

            // Run external editor
            let result = run_editor(&content)?;

            // Resume TUI
            *terminal = ratatui::init();

            // Apply result if editor succeeded
            if let Some(new_content) = result {
                app.apply_editor_result(&new_content);
            } else {
                app.set_status_message("Editor cancelled");
            }
            continue;
        }

        terminal.draw(|frame| ui::draw(frame, app))?;

        if let Some(evt) = event::poll_event(Duration::from_millis(100))? {
            event::handle_event(app, &evt);
        }
    }

    Ok(())
}

/// Runs the external editor with the given content.
///
/// Returns `Some(content)` if the editor exited successfully, `None` otherwise.
fn run_editor(content: &str) -> io::Result<Option<String>> {
    // Prefer VISUAL, then EDITOR, then fall back to vi
    let editor_cmd = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".into());

    // Parse the editor command into program + args using shell-style splitting
    let mut parts = shell_words::split(&editor_cmd)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

    if parts.is_empty() {
        parts.push("vi".into());
    }

    let program = parts.remove(0);

    // Create temp file with content
    let mut tmp = tempfile::NamedTempFile::new()?;
    tmp.write_all(content.as_bytes())?;
    tmp.flush()?;

    // Run editor with any additional arguments
    let status = std::process::Command::new(&program)
        .args(&parts)
        .arg(tmp.path())
        .status()?;

    if status.success() {
        Ok(Some(std::fs::read_to_string(tmp.path())?))
    } else {
        Ok(None) // Editor exited with error, cancel
    }
}
