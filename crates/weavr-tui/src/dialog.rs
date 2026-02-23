//! Dialog management for modal overlays.
//!
//! This module handles:
//! - Help dialog
//! - `AcceptBoth` options dialog

use weavr_core::{AcceptBothOptions, BothOrder, Resolution};

use crate::input::{AcceptBothOptionsState, Dialog, FileListState, HelpState, InputMode};
use crate::resolution;
use crate::App;

/// Shows the help dialog.
pub fn show_help(app: &mut App) {
    app.active_dialog = Some(Dialog::Help(HelpState::default()));
    app.input_mode = InputMode::Dialog;
}

/// Scrolls the help dialog down by the given number of lines.
pub fn help_scroll_down(app: &mut App, lines: usize) {
    if let Some(Dialog::Help(ref mut state)) = app.active_dialog {
        state.scroll = state.scroll.saturating_add(lines);
    }
}

/// Scrolls the help dialog up by the given number of lines.
pub fn help_scroll_up(app: &mut App, lines: usize) {
    if let Some(Dialog::Help(ref mut state)) = app.active_dialog {
        state.scroll = state.scroll.saturating_sub(lines);
    }
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

/// Shows the staging prompt dialog.
pub fn show_staging_prompt(app: &mut App) {
    app.active_dialog = Some(Dialog::StagingPrompt);
    app.input_mode = InputMode::Dialog;
}

/// Confirms staging in the staging prompt dialog.
pub fn confirm_staging(app: &mut App) {
    let multi = app.is_multi_file();
    app.stage_requested = true;
    if let Some(ref mut ws) = app.workspace {
        ws.current_mut().stage_requested = true;
    }
    close_dialog(app);
    if multi {
        app.complete_current_and_advance();
    } else {
        app.quit();
    }
}

/// Denies staging in the staging prompt dialog.
pub fn deny_staging(app: &mut App) {
    let multi = app.is_multi_file();
    close_dialog(app);
    if multi {
        app.complete_current_and_advance();
    } else {
        app.quit();
    }
}

/// Shows the file list dialog.
pub fn show_file_list(app: &mut App) {
    let selected = app.current_file_index();
    app.active_dialog = Some(Dialog::FileList(FileListState {
        selected_index: selected,
    }));
    app.input_mode = InputMode::Dialog;
}

/// Moves the file list selection down.
pub fn file_list_move_down(app: &mut App) {
    let count = app.file_count();
    if let Some(Dialog::FileList(ref mut state)) = app.active_dialog {
        if state.selected_index + 1 < count {
            state.selected_index += 1;
        }
    }
}

/// Moves the file list selection up.
pub fn file_list_move_up(app: &mut App) {
    if let Some(Dialog::FileList(ref mut state)) = app.active_dialog {
        state.selected_index = state.selected_index.saturating_sub(1);
    }
}

/// Selects the current item in the file list dialog and switches to that file.
pub fn file_list_select(app: &mut App) {
    let selected = if let Some(Dialog::FileList(ref state)) = app.active_dialog {
        state.selected_index
    } else {
        return;
    };
    close_dialog(app);
    app.go_to_file(selected);
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
