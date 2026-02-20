//! Resolution handling for conflict hunks.
//!
//! This module handles:
//! - Applying resolutions (left, right, both, manual)
//! - Clearing resolutions
//! - Undo/redo support

use weavr_core::{AcceptBothOptions, Action, ConflictHunk, Resolution};

use crate::App;

/// Resolves the current hunk by accepting the left (ours) content.
pub fn resolve_left(app: &mut App) {
    apply_resolution(app, "Accept ours", Resolution::accept_left);
}

/// Resolves the current hunk by accepting the right (theirs) content.
pub fn resolve_right(app: &mut App) {
    apply_resolution(app, "Accept theirs", Resolution::accept_right);
}

/// Resolves the current hunk by accepting both sides (left then right).
pub fn resolve_both(app: &mut App) {
    apply_resolution(app, "Accept both", |hunk| {
        Resolution::accept_both(hunk, &AcceptBothOptions::default())
    });
}

/// Clears the resolution for the current hunk, returning it to unresolved state.
pub fn clear_current_resolution(app: &mut App) {
    // Get hunk info and current resolution for undo
    let Some((hunk_id, prev)) = app.session.as_ref().and_then(|session| {
        session
            .hunks()
            .get(app.current_hunk_index)
            .map(|hunk| (hunk.id, session.resolutions().get(&hunk.id).cloned()))
    }) else {
        return;
    };

    // Only proceed if there was a resolution to clear
    let Some(old_resolution) = prev else {
        app.set_status_message("No resolution to clear");
        return;
    };

    if let Some(session) = app.session.as_mut() {
        match session.clear_resolution(hunk_id) {
            Ok(()) => {
                app.action_history.record(Action::ClearResolution {
                    hunk_id,
                    old: old_resolution,
                });
                app.set_status_message("Cleared resolution");
            }
            Err(_) => {
                app.set_status_message("Failed to clear resolution");
            }
        }
    }
}

/// Undoes the last resolution action.
pub fn undo(app: &mut App) {
    let Some(action) = app.action_history.undo().cloned() else {
        app.set_status_message("Nothing to undo");
        return;
    };

    if let Some(session) = &mut app.session {
        let result = match &action {
            Action::SetResolution { hunk_id, old, .. } => {
                if let Some(old_resolution) = old {
                    session.set_resolution(*hunk_id, old_resolution.clone())
                } else {
                    session.clear_resolution(*hunk_id)
                }
            }
            Action::ClearResolution { hunk_id, old } => {
                session.set_resolution(*hunk_id, old.clone())
            }
        };

        match result {
            Ok(()) => app.set_status_message(&format!("Undid: {}", action.description())),
            Err(_) => app.set_status_message("Failed to undo"),
        }
    }
}

/// Redoes the last undone resolution action.
pub fn redo(app: &mut App) {
    let Some(action) = app.action_history.redo().cloned() else {
        app.set_status_message("Nothing to redo");
        return;
    };

    if let Some(session) = &mut app.session {
        let result = match &action {
            Action::SetResolution { hunk_id, new, .. } => {
                session.set_resolution(*hunk_id, new.clone())
            }
            Action::ClearResolution { hunk_id, .. } => session.clear_resolution(*hunk_id),
        };

        match result {
            Ok(()) => app.set_status_message(&format!("Redid: {}", action.description())),
            Err(_) => app.set_status_message("Failed to redo"),
        }
    }
}

/// Applies a resolution to the current hunk with undo support.
///
/// This is a helper that handles the common pattern of:
/// 1. Getting the current hunk and its previous resolution
/// 2. Recording an undo action
/// 3. Applying the new resolution
/// 4. Setting a status message
///
/// This function is `pub(crate)` to allow use by dialog and editor modules.
pub(crate) fn apply_resolution<F>(app: &mut App, action: &str, make_resolution: F)
where
    F: FnOnce(&ConflictHunk) -> Resolution,
{
    // Extract all data upfront to end the immutable borrow
    let Some((hunk_id, resolution, prev)) = app.session.as_ref().and_then(|session| {
        session.hunks().get(app.current_hunk_index).map(|hunk| {
            let prev = session.resolutions().get(&hunk.id).cloned();
            (hunk.id, make_resolution(hunk), prev)
        })
    }) else {
        return;
    };

    // Apply resolution and only record action / set status on success
    if let Some(session) = app.session.as_mut() {
        match session.set_resolution(hunk_id, resolution.clone()) {
            Ok(()) => {
                app.action_history.record(Action::SetResolution {
                    hunk_id,
                    old: prev,
                    new: resolution,
                });
                app.set_status_message(action);
            }
            Err(_) => {
                app.set_status_message("Failed to apply resolution");
            }
        }
    }
}
