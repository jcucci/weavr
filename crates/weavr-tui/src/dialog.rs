//! Dialog management for modal overlays.
//!
//! This module handles:
//! - Help dialog
//! - `AcceptBoth` options dialog

use weavr_core::{AcceptBothOptions, BothOrder, Resolution};

use crate::input::{AcceptBothOptionsState, Dialog, InputMode};
use crate::resolution;
use crate::App;

/// Shows the help dialog.
pub fn show_help(app: &mut App) {
    app.active_dialog = Some(Dialog::Help);
    app.input_mode = InputMode::Dialog;
}

/// Closes any open dialog and returns to normal mode.
pub fn close_dialog(app: &mut App) {
    app.active_dialog = None;
    app.input_mode = InputMode::Normal;
}

/// Shows the `AcceptBoth` options dialog.
pub fn show_accept_both_dialog(app: &mut App) {
    app.active_dialog = Some(Dialog::AcceptBothOptions(AcceptBothOptionsState::default()));
    app.input_mode = InputMode::Dialog;
}

/// Toggles the order in the `AcceptBoth` options dialog.
pub fn toggle_accept_both_order(app: &mut App) {
    if let Some(Dialog::AcceptBothOptions(ref mut state)) = app.active_dialog {
        state.order = match state.order {
            BothOrder::LeftThenRight => BothOrder::RightThenLeft,
            BothOrder::RightThenLeft => BothOrder::LeftThenRight,
        };
    }
}

/// Toggles the deduplicate option in the `AcceptBoth` options dialog.
pub fn toggle_accept_both_dedupe(app: &mut App) {
    if let Some(Dialog::AcceptBothOptions(ref mut state)) = app.active_dialog {
        state.deduplicate = !state.deduplicate;
    }
}

/// Confirms the `AcceptBoth` options and applies the resolution.
pub fn confirm_accept_both(app: &mut App) {
    // Extract options from dialog
    let options = if let Some(Dialog::AcceptBothOptions(ref state)) = app.active_dialog {
        AcceptBothOptions {
            order: state.order,
            deduplicate: state.deduplicate,
            trim_whitespace: false,
        }
    } else {
        return;
    };

    // Close dialog first
    close_dialog(app);

    // Apply resolution with extracted options
    resolution::apply_resolution(app, "Accept both", |hunk| {
        Resolution::accept_both(hunk, &options)
    });
}
