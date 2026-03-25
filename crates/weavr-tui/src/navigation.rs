//! Navigation and focus management for the TUI.
//!
//! This module handles:
//! - Pane focus cycling
//! - Hunk navigation (next/prev, unresolved, go-to)
//! - Scrolling within panes

use weavr_core::HunkState;

use crate::{App, FocusedPane};

/// Scroll, hunk index, and pane focus state.
#[derive(Debug, Clone, Default)]
pub(crate) struct ScrollState {
    /// Synchronized scroll offset for left/right panes.
    pub(crate) left_right_scroll: u16,
    /// Independent scroll offset for result pane.
    pub(crate) result_scroll: u16,
    /// Current hunk index (0-based).
    pub(crate) current_hunk_index: usize,
    /// Which pane has focus.
    pub(crate) focused_pane: FocusedPane,
}

// --- Focus Management ---

/// Cycles focus to the next pane.
///
/// When the base pane is visible: Left -> Base -> Right -> Result -> Left.
/// Otherwise: Left -> Right -> Result -> Left.
pub fn cycle_focus(app: &mut App) {
    app.scroll.focused_pane = match app.scroll.focused_pane {
        FocusedPane::Left => {
            if app.display.show_base_pane {
                FocusedPane::Base
            } else {
                FocusedPane::Right
            }
        }
        FocusedPane::Base => FocusedPane::Right,
        FocusedPane::Right => FocusedPane::Result,
        FocusedPane::Result => FocusedPane::Left,
    };
}

/// Cycles focus to the previous pane.
///
/// When the base pane is visible: Left -> Result -> Right -> Base -> Left.
/// Otherwise: Left -> Result -> Right -> Left.
pub fn cycle_focus_back(app: &mut App) {
    app.scroll.focused_pane = match app.scroll.focused_pane {
        FocusedPane::Left => FocusedPane::Result,
        FocusedPane::Base => FocusedPane::Left,
        FocusedPane::Right => {
            if app.display.show_base_pane {
                FocusedPane::Base
            } else {
                FocusedPane::Left
            }
        }
        FocusedPane::Result => FocusedPane::Right,
    };
}

/// Sets focus directly to the result pane.
pub fn focus_result(app: &mut App) {
    app.scroll.focused_pane = FocusedPane::Result;
}

// --- Hunk Navigation ---

/// Moves to the next hunk.
pub fn next_hunk(app: &mut App) {
    let total = app.total_hunks();
    if total > 0 && app.scroll.current_hunk_index < total - 1 {
        app.scroll.current_hunk_index += 1;
        reset_scroll(app);
    }
}

/// Moves to the previous hunk.
pub fn prev_hunk(app: &mut App) {
    if app.scroll.current_hunk_index > 0 {
        app.scroll.current_hunk_index -= 1;
        reset_scroll(app);
    }
}

/// Moves to a specific hunk by index.
pub fn go_to_hunk(app: &mut App, index: usize) {
    let total = app.total_hunks();
    if total > 0 && index < total {
        app.scroll.current_hunk_index = index;
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
            let idx = (app.scroll.current_hunk_index + i) % total;
            if matches!(hunks[idx].state, HunkState::Unresolved) {
                app.scroll.current_hunk_index = idx;
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
            let idx = (app.scroll.current_hunk_index + total - i) % total;
            if matches!(hunks[idx].state, HunkState::Unresolved) {
                app.scroll.current_hunk_index = idx;
                reset_scroll(app);
                return;
            }
        }
    }
}

// --- Scrolling ---

/// Scrolls up by the specified number of lines.
pub fn scroll_up(app: &mut App, lines: u16) {
    match app.scroll.focused_pane {
        FocusedPane::Left | FocusedPane::Base | FocusedPane::Right => {
            app.scroll.left_right_scroll = app.scroll.left_right_scroll.saturating_sub(lines);
        }
        FocusedPane::Result => {
            app.scroll.result_scroll = app.scroll.result_scroll.saturating_sub(lines);
        }
    }
}

/// Scrolls down by the specified number of lines.
pub fn scroll_down(app: &mut App, lines: u16) {
    match app.scroll.focused_pane {
        FocusedPane::Left | FocusedPane::Base | FocusedPane::Right => {
            app.scroll.left_right_scroll = app.scroll.left_right_scroll.saturating_add(lines);
        }
        FocusedPane::Result => {
            app.scroll.result_scroll = app.scroll.result_scroll.saturating_add(lines);
        }
    }
}

/// Resets scroll positions and clears hunk-specific AI state when changing hunks.
///
/// Preserves batch state and cached suggestions for other hunks.
fn reset_scroll(app: &mut App) {
    app.scroll.left_right_scroll = 0;
    app.scroll.result_scroll = 0;

    // Cancel in-flight single-hunk AI request when navigating
    if let Some(pending_id) = app.ai_state.pending_hunk {
        if let Some(ai_handle) = &app.ai_handle {
            let _ = ai_handle.send(crate::ai::AiCommand::Cancel {
                hunk_id: pending_id,
            });
        }
    }
    app.ai_state.pending_hunk = None;
    // Clear in-flight explanation hash; keep cached explanations and suggestions
    app.ai_state.pending_explanation_hash = None;
}
