//! Navigation and focus management for the TUI.
//!
//! This module handles:
//! - Pane focus cycling
//! - Hunk navigation (next/prev, unresolved, go-to)
//! - Scrolling within panes

use weavr_core::HunkState;

use crate::{App, FocusedPane};

// --- Focus Management ---

/// Cycles focus to the next pane (Left -> Right -> Result -> Left).
pub fn cycle_focus(app: &mut App) {
    app.focused_pane = match app.focused_pane {
        FocusedPane::Left => FocusedPane::Right,
        FocusedPane::Right => FocusedPane::Result,
        FocusedPane::Result => FocusedPane::Left,
    };
}

/// Cycles focus to the previous pane (Left -> Result -> Right -> Left).
pub fn cycle_focus_back(app: &mut App) {
    app.focused_pane = match app.focused_pane {
        FocusedPane::Left => FocusedPane::Result,
        FocusedPane::Right => FocusedPane::Left,
        FocusedPane::Result => FocusedPane::Right,
    };
}

/// Sets focus directly to the result pane.
pub fn focus_result(app: &mut App) {
    app.focused_pane = FocusedPane::Result;
}

// --- Hunk Navigation ---

/// Moves to the next hunk.
pub fn next_hunk(app: &mut App) {
    let total = app.total_hunks();
    if total > 0 && app.current_hunk_index < total - 1 {
        app.current_hunk_index += 1;
        reset_scroll(app);
    }
}

/// Moves to the previous hunk.
pub fn prev_hunk(app: &mut App) {
    if app.current_hunk_index > 0 {
        app.current_hunk_index -= 1;
        reset_scroll(app);
    }
}

/// Moves to a specific hunk by index.
pub fn go_to_hunk(app: &mut App, index: usize) {
    let total = app.total_hunks();
    if total > 0 && index < total {
        app.current_hunk_index = index;
        reset_scroll(app);
    }
}

/// Moves to the next unresolved hunk, wrapping around if necessary.
pub fn next_unresolved_hunk(app: &mut App) {
    if let Some(session) = &app.session {
        let hunks = session.hunks();
        let total = hunks.len();
        if total == 0 {
            return;
        }

        // Search forward from current position
        for i in 1..=total {
            let idx = (app.current_hunk_index + i) % total;
            if matches!(hunks[idx].state, HunkState::Unresolved) {
                app.current_hunk_index = idx;
                reset_scroll(app);
                return;
            }
        }
    }
}

/// Moves to the previous unresolved hunk, wrapping around if necessary.
pub fn prev_unresolved_hunk(app: &mut App) {
    if let Some(session) = &app.session {
        let hunks = session.hunks();
        let total = hunks.len();
        if total == 0 {
            return;
        }

        // Search backward from current position
        for i in 1..=total {
            let idx = (app.current_hunk_index + total - i) % total;
            if matches!(hunks[idx].state, HunkState::Unresolved) {
                app.current_hunk_index = idx;
                reset_scroll(app);
                return;
            }
        }
    }
}

// --- Scrolling ---

/// Scrolls up by the specified number of lines.
pub fn scroll_up(app: &mut App, lines: u16) {
    match app.focused_pane {
        FocusedPane::Left | FocusedPane::Right => {
            app.left_right_scroll = app.left_right_scroll.saturating_sub(lines);
        }
        FocusedPane::Result => {
            app.result_scroll = app.result_scroll.saturating_sub(lines);
        }
    }
}

/// Scrolls down by the specified number of lines.
pub fn scroll_down(app: &mut App, lines: u16) {
    match app.focused_pane {
        FocusedPane::Left | FocusedPane::Right => {
            app.left_right_scroll = app.left_right_scroll.saturating_add(lines);
        }
        FocusedPane::Result => {
            app.result_scroll = app.result_scroll.saturating_add(lines);
        }
    }
}

/// Resets scroll positions when changing hunks.
fn reset_scroll(app: &mut App) {
    app.left_right_scroll = 0;
    app.result_scroll = 0;
}
