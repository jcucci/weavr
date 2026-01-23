//! Event handling for the TUI.
//!
//! This module handles keyboard and terminal events using crossterm.

use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::App;

/// Polls for an event with the given timeout.
///
/// Returns `None` if no event is available within the timeout.
///
/// # Errors
///
/// Returns an error if the terminal event polling fails.
pub fn poll_event(timeout: Duration) -> std::io::Result<Option<Event>> {
    if event::poll(timeout)? {
        Ok(Some(event::read()?))
    } else {
        Ok(None)
    }
}

/// Handles an event, updating app state as needed.
pub fn handle_event(app: &mut App, event: &Event) {
    if let Event::Key(key) = event {
        handle_key_event(app, *key);
    }
    // Resize and other events are handled automatically by ratatui on next draw
}

/// Handles a key event, updating app state.
fn handle_key_event(app: &mut App, key: KeyEvent) {
    // Only handle Press events (not Release on Windows)
    if key.kind != KeyEventKind::Press {
        return;
    }

    match key.code {
        // Quit
        KeyCode::Char('q') => app.quit(),

        // Focus cycling
        KeyCode::Tab => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                app.cycle_focus_back();
            } else {
                app.cycle_focus();
            }
        }
        KeyCode::BackTab => app.cycle_focus_back(),

        // Hunk navigation
        KeyCode::Char('j') | KeyCode::Down => app.next_hunk(),
        KeyCode::Char('k') | KeyCode::Up => app.prev_hunk(),
        KeyCode::Char('n') => app.next_unresolved_hunk(),
        KeyCode::Char('g') => app.go_to_hunk(0),
        KeyCode::Char('G') => {
            let last = app.total_hunks().saturating_sub(1);
            app.go_to_hunk(last);
        }

        // Scrolling (half page = 10 lines)
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.scroll_down(10);
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.scroll_up(10);
        }
        KeyCode::PageDown => app.scroll_down(20),
        KeyCode::PageUp => app.scroll_up(20),

        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    fn make_key_event(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn q_key_quits() {
        let mut app = App::new();
        assert!(!app.should_quit());

        let event = Event::Key(make_key_event(KeyCode::Char('q'), KeyModifiers::NONE));
        handle_event(&mut app, &event);

        assert!(app.should_quit());
    }

    #[test]
    fn tab_cycles_focus_forward() {
        use crate::FocusedPane;

        let mut app = App::new();
        assert_eq!(app.focused_pane(), FocusedPane::Left);

        let event = Event::Key(make_key_event(KeyCode::Tab, KeyModifiers::NONE));
        handle_event(&mut app, &event);

        assert_eq!(app.focused_pane(), FocusedPane::Right);
    }

    #[test]
    fn shift_tab_cycles_focus_backward() {
        use crate::FocusedPane;

        let mut app = App::new();
        assert_eq!(app.focused_pane(), FocusedPane::Left);

        let event = Event::Key(make_key_event(KeyCode::Tab, KeyModifiers::SHIFT));
        handle_event(&mut app, &event);

        assert_eq!(app.focused_pane(), FocusedPane::Result);
    }

    #[test]
    fn backtab_cycles_focus_backward() {
        use crate::FocusedPane;

        let mut app = App::new();
        assert_eq!(app.focused_pane(), FocusedPane::Left);

        let event = Event::Key(make_key_event(KeyCode::BackTab, KeyModifiers::NONE));
        handle_event(&mut app, &event);

        assert_eq!(app.focused_pane(), FocusedPane::Result);
    }

    #[test]
    fn key_release_is_ignored() {
        let mut app = App::new();
        assert!(!app.should_quit());

        // Create a key release event for 'q'
        let key_event = KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Release,
            state: KeyEventState::NONE,
        };
        let event = Event::Key(key_event);
        handle_event(&mut app, &event);

        // Should NOT quit because it was a release event
        assert!(!app.should_quit());
    }

    #[test]
    fn resize_event_does_not_panic() {
        let mut app = App::new();
        let event = Event::Resize(80, 24);
        handle_event(&mut app, &event);
        // Just verify no panic occurs
    }

    #[test]
    fn j_key_calls_next_hunk() {
        let mut app = App::new();
        let event = Event::Key(make_key_event(KeyCode::Char('j'), KeyModifiers::NONE));
        handle_event(&mut app, &event);
        // Without a session, this is a no-op but shouldn't panic
    }

    #[test]
    fn k_key_calls_prev_hunk() {
        let mut app = App::new();
        let event = Event::Key(make_key_event(KeyCode::Char('k'), KeyModifiers::NONE));
        handle_event(&mut app, &event);
        // Without a session, this is a no-op but shouldn't panic
    }

    #[test]
    fn down_arrow_calls_next_hunk() {
        let mut app = App::new();
        let event = Event::Key(make_key_event(KeyCode::Down, KeyModifiers::NONE));
        handle_event(&mut app, &event);
    }

    #[test]
    fn up_arrow_calls_prev_hunk() {
        let mut app = App::new();
        let event = Event::Key(make_key_event(KeyCode::Up, KeyModifiers::NONE));
        handle_event(&mut app, &event);
    }

    #[test]
    fn n_key_calls_next_unresolved() {
        let mut app = App::new();
        let event = Event::Key(make_key_event(KeyCode::Char('n'), KeyModifiers::NONE));
        handle_event(&mut app, &event);
    }

    #[test]
    fn g_key_goes_to_first_hunk() {
        let mut app = App::new();
        let event = Event::Key(make_key_event(KeyCode::Char('g'), KeyModifiers::NONE));
        handle_event(&mut app, &event);
    }

    #[test]
    fn shift_g_goes_to_last_hunk() {
        let mut app = App::new();
        let event = Event::Key(make_key_event(KeyCode::Char('G'), KeyModifiers::NONE));
        handle_event(&mut app, &event);
    }

    #[test]
    fn ctrl_d_scrolls_down() {
        let mut app = App::new();
        assert_eq!(app.left_right_scroll(), 0);

        let event = Event::Key(make_key_event(KeyCode::Char('d'), KeyModifiers::CONTROL));
        handle_event(&mut app, &event);

        assert_eq!(app.left_right_scroll(), 10);
    }

    #[test]
    fn ctrl_u_scrolls_up() {
        let mut app = App::new();
        // First scroll down
        app.scroll_down(20);
        assert_eq!(app.left_right_scroll(), 20);

        let event = Event::Key(make_key_event(KeyCode::Char('u'), KeyModifiers::CONTROL));
        handle_event(&mut app, &event);

        assert_eq!(app.left_right_scroll(), 10);
    }

    #[test]
    fn page_down_scrolls_down() {
        let mut app = App::new();
        let event = Event::Key(make_key_event(KeyCode::PageDown, KeyModifiers::NONE));
        handle_event(&mut app, &event);
        assert_eq!(app.left_right_scroll(), 20);
    }

    #[test]
    fn page_up_scrolls_up() {
        let mut app = App::new();
        app.scroll_down(30);
        let event = Event::Key(make_key_event(KeyCode::PageUp, KeyModifiers::NONE));
        handle_event(&mut app, &event);
        assert_eq!(app.left_right_scroll(), 10);
    }
}
