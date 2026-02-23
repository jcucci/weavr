//! Event handling for the TUI.
//!
//! This module handles keyboard and terminal events using crossterm.

use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::ai;
use crate::input::{Dialog, InputMode};
use crate::keybindings::Action;
use crate::{App, KEY_SEQUENCE_TIMEOUT};

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

    match app.input_mode() {
        InputMode::Normal => handle_normal_mode(app, key),
        InputMode::Command => handle_command_mode(app, key),
        InputMode::Dialog => handle_dialog_mode(app, key),
    }
}

/// Normalizes a key event for consistent lookup.
///
/// `BackTab` inherently means Shift+Tab, but crossterm may or may not include
/// the SHIFT modifier depending on the platform. Strip the redundant SHIFT so
/// bindings registered as `(BackTab, NONE)` always match.
fn normalize_key(code: KeyCode, mods: KeyModifiers) -> (KeyCode, KeyModifiers) {
    match code {
        // BackTab inherently means Shift+Tab; strip the redundant SHIFT modifier.
        KeyCode::BackTab => (KeyCode::BackTab, mods.difference(KeyModifiers::SHIFT)),
        // Some terminals send Tab+SHIFT instead of BackTab; normalize to BackTab.
        KeyCode::Tab if mods.contains(KeyModifiers::SHIFT) => {
            (KeyCode::BackTab, mods.difference(KeyModifiers::SHIFT))
        }
        _ => (code, mods),
    }
}

/// Handles key events in normal mode using the keybinding map.
fn handle_normal_mode(app: &mut App, key: KeyEvent) {
    let (code, mods) = normalize_key(key.code, key.modifiers);

    // Check if this key could be the start of a multi-key sequence
    if app.keybindings.is_sequence_prefix(code, mods) {
        if app.key_sequence.has_pending() {
            // We have buffered key(s) + this new key. Check for a complete sequence.
            let mut pending = app.key_sequence.pending_keys(KEY_SEQUENCE_TIMEOUT);
            pending.push((code, mods));

            if let Some(action) = app.keybindings.lookup_sequence(&pending) {
                app.key_sequence.clear();
                dispatch_action(app, action);
                return;
            }
        }
        // Buffer this key as a potential sequence start
        app.key_sequence.push(code, mods);
        return;
    }

    // Not a sequence prefix. If we have buffered keys, check for a complete
    // sequence with the current key appended.
    if app.key_sequence.has_pending() {
        let mut pending = app.key_sequence.pending_keys(KEY_SEQUENCE_TIMEOUT);
        pending.push((code, mods));

        if let Some(action) = app.keybindings.lookup_sequence(&pending) {
            app.key_sequence.clear();
            dispatch_action(app, action);
            return;
        }

        // No sequence match. Dispatch buffered keys as singles, then this key.
        let buffered = app.key_sequence.drain();
        for (buf_code, buf_mods) in buffered {
            if let Some(action) = app.keybindings.lookup_single(buf_code, buf_mods) {
                dispatch_action(app, action);
            }
        }
    }

    app.key_sequence.clear();

    // Dispatch the current key
    if let Some(action) = app.keybindings.lookup_single(code, mods) {
        dispatch_action(app, action);
    }
}

/// Dispatches a resolved action to the appropriate app method.
fn dispatch_action(app: &mut App, action: Action) {
    match action {
        // Application
        Action::Quit => app.quit(),
        Action::EnterCommandMode => app.enter_command_mode(),
        Action::ShowHelp => app.show_help(),

        // Navigation
        Action::NextHunk => app.next_hunk(),
        Action::PrevHunk => app.prev_hunk(),
        Action::NextUnresolved => app.next_unresolved_hunk(),
        Action::PrevUnresolved => app.prev_unresolved_hunk(),
        Action::FirstHunk => app.go_to_hunk(0),
        Action::LastHunk => {
            let last = app.total_hunks().saturating_sub(1);
            app.go_to_hunk(last);
        }
        Action::CycleFocus => app.cycle_focus(),
        Action::CycleFocusBack => app.cycle_focus_back(),
        Action::FocusResult => {
            // Context-sensitive: accept AI suggestion if present, else focus result
            if app
                .current_hunk()
                .is_some_and(|h| app.ai_state().has_suggestion_for(h.id))
            {
                ai::accept_suggestion(app);
            } else {
                app.focus_result();
            }
        }

        // Scrolling
        Action::ScrollHalfDown => app.scroll_down(10),
        Action::ScrollHalfUp => app.scroll_up(10),
        Action::ScrollPageDown => app.scroll_down(20),
        Action::ScrollPageUp => app.scroll_up(20),

        // Resolution
        Action::ResolveLeft => app.resolve_left(),
        Action::ResolveRight => app.resolve_right(),
        Action::ResolveBoth => app.resolve_both(),
        Action::ResolveBothOptions => app.show_accept_both_dialog(),
        Action::ClearResolution => app.clear_current_resolution(),
        Action::Undo => app.undo(),
        Action::Redo => app.redo(),
        Action::EditInEditor => {
            app.prepare_editor();
        }

        // AI
        Action::AiSuggest => ai::request_suggestion(app),
        Action::AiSuggestAll => ai::request_all_suggestions(app),
        Action::AiExplainOrHelp => {
            let has_suggestion = app
                .current_hunk()
                .is_some_and(|h| app.ai_state().has_suggestion_for(h.id));
            if has_suggestion || app.ai_state().is_loading() {
                ai::request_explanation(app);
            } else {
                app.show_help();
            }
        }
        Action::DismissAiSuggestion => {
            if app
                .current_hunk()
                .is_some_and(|h| app.ai_state().has_suggestion_for(h.id))
            {
                ai::dismiss_suggestion(app);
            }
        }
    }
}

/// Handles key events in command mode.
fn handle_command_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => app.exit_command_mode(),
        KeyCode::Enter => app.execute_command(),
        KeyCode::Backspace => {
            app.backspace_command();
            // Exit command mode if buffer becomes empty
            if app.command_buffer().is_empty() {
                app.exit_command_mode();
            }
        }
        KeyCode::Char(c) => app.append_to_command(c),
        _ => {}
    }
}

/// Handles key events in dialog mode.
fn handle_dialog_mode(app: &mut App, key: KeyEvent) {
    // Check which dialog is active
    match app.active_dialog() {
        Some(Dialog::Help(_)) => {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q' | '?') => app.close_dialog(),
                // Scroll: j/Down = 1 line, k/Up = 1 line
                KeyCode::Char('j') | KeyCode::Down => {
                    crate::dialog::help_scroll_down(app, 1);
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    crate::dialog::help_scroll_up(app, 1);
                }
                // Scroll: Ctrl+d/PageDown = half page, Ctrl+u/PageUp = half page
                KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    crate::dialog::help_scroll_down(app, 10);
                }
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    crate::dialog::help_scroll_up(app, 10);
                }
                KeyCode::PageDown => {
                    crate::dialog::help_scroll_down(app, 10);
                }
                KeyCode::PageUp => {
                    crate::dialog::help_scroll_up(app, 10);
                }
                _ => {}
            }
        }
        Some(Dialog::AcceptBothOptions(_)) => {
            // AcceptBoth options dialog
            match key.code {
                KeyCode::Esc => app.close_dialog(),
                KeyCode::Char('l' | 'L' | 'r' | 'R') => app.toggle_accept_both_order(),
                KeyCode::Char(' ') => app.toggle_accept_both_dedupe(),
                KeyCode::Enter => app.confirm_accept_both(),
                _ => {}
            }
        }
        Some(Dialog::AiExplanation(_)) => {
            // AI explanation dialog
            match key.code {
                KeyCode::Esc | KeyCode::Char('q' | '?') => app.close_dialog(),
                _ => {}
            }
        }
        None => {}
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
    fn backtab_with_shift_modifier_cycles_focus_backward() {
        use crate::FocusedPane;

        let mut app = App::new();
        assert_eq!(app.focused_pane(), FocusedPane::Left);

        // crossterm on macOS sends BackTab with SHIFT set
        let event = Event::Key(make_key_event(KeyCode::BackTab, KeyModifiers::SHIFT));
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
    fn gg_sequence_goes_to_first_hunk() {
        let mut app = App::new();
        // First g - sets pending
        let event1 = Event::Key(make_key_event(KeyCode::Char('g'), KeyModifiers::NONE));
        handle_event(&mut app, &event1);
        // Second g - triggers go_to_hunk(0)
        let event2 = Event::Key(make_key_event(KeyCode::Char('g'), KeyModifiers::NONE));
        handle_event(&mut app, &event2);
        // Should have called go_to_hunk(0) - no panic without session
    }

    #[test]
    fn single_g_does_not_trigger_first_hunk() {
        let mut app = App::new();
        // First g
        let event1 = Event::Key(make_key_event(KeyCode::Char('g'), KeyModifiers::NONE));
        handle_event(&mut app, &event1);
        // Different key clears pending
        let event2 = Event::Key(make_key_event(KeyCode::Char('j'), KeyModifiers::NONE));
        handle_event(&mut app, &event2);
        // Should not have gone to first hunk
    }

    #[test]
    fn shift_g_goes_to_last_hunk() {
        let mut app = App::new();
        let event = Event::Key(make_key_event(KeyCode::Char('G'), KeyModifiers::NONE));
        handle_event(&mut app, &event);
    }

    #[test]
    fn capital_n_calls_prev_unresolved() {
        let mut app = App::new();
        let event = Event::Key(make_key_event(KeyCode::Char('N'), KeyModifiers::NONE));
        handle_event(&mut app, &event);
        // Should not panic without session
    }

    #[test]
    fn enter_key_focuses_result_pane() {
        use crate::FocusedPane;

        let mut app = App::new();
        assert_eq!(app.focused_pane(), FocusedPane::Left);

        let event = Event::Key(make_key_event(KeyCode::Enter, KeyModifiers::NONE));
        handle_event(&mut app, &event);

        assert_eq!(app.focused_pane(), FocusedPane::Result);
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

    #[test]
    fn shift_b_opens_accept_both_dialog() {
        use crate::input::InputMode;

        let mut app = App::new();
        assert_eq!(app.input_mode(), InputMode::Normal);

        let event = Event::Key(make_key_event(KeyCode::Char('B'), KeyModifiers::NONE));
        handle_event(&mut app, &event);

        assert_eq!(app.input_mode(), InputMode::Dialog);
        assert!(app.active_dialog().is_some());
    }

    #[test]
    fn e_key_prepares_editor() {
        let mut app = App::new();

        // Without a session, prepare_editor returns false but doesn't crash
        let event = Event::Key(make_key_event(KeyCode::Char('e'), KeyModifiers::NONE));
        handle_event(&mut app, &event);
        // No crash is success
    }

    #[test]
    fn accept_both_dialog_l_toggles_order() {
        use crate::input::Dialog;
        use weavr_core::BothOrder;

        let mut app = App::new();
        app.show_accept_both_dialog();

        // Verify initial state
        if let Some(Dialog::AcceptBothOptions(state)) = app.active_dialog() {
            assert_eq!(state.order, BothOrder::LeftThenRight);
        }

        // Press 'l' to toggle
        let event = Event::Key(make_key_event(KeyCode::Char('l'), KeyModifiers::NONE));
        handle_event(&mut app, &event);

        if let Some(Dialog::AcceptBothOptions(state)) = app.active_dialog() {
            assert_eq!(state.order, BothOrder::RightThenLeft);
        }
    }

    #[test]
    fn accept_both_dialog_space_toggles_dedupe() {
        use crate::input::Dialog;

        let mut app = App::new();
        app.show_accept_both_dialog();

        // Verify initial state
        if let Some(Dialog::AcceptBothOptions(state)) = app.active_dialog() {
            assert!(!state.deduplicate);
        }

        // Press space to toggle
        let event = Event::Key(make_key_event(KeyCode::Char(' '), KeyModifiers::NONE));
        handle_event(&mut app, &event);

        if let Some(Dialog::AcceptBothOptions(state)) = app.active_dialog() {
            assert!(state.deduplicate);
        }
    }

    #[test]
    fn accept_both_dialog_esc_closes() {
        use crate::input::InputMode;

        let mut app = App::new();
        app.show_accept_both_dialog();
        assert_eq!(app.input_mode(), InputMode::Dialog);

        let event = Event::Key(make_key_event(KeyCode::Esc, KeyModifiers::NONE));
        handle_event(&mut app, &event);

        assert_eq!(app.input_mode(), InputMode::Normal);
        assert!(app.active_dialog().is_none());
    }
}
