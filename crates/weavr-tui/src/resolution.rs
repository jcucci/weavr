//! Resolution handling for conflict hunks.
//!
//! This module handles:
//! - Applying resolutions (left, right, both, manual)
//! - Clearing resolutions
//! - Undo support

use weavr_core::{AcceptBothOptions, ConflictHunk, Resolution};

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

    if let Some(session) = app.session.as_mut() {
        match session.clear_resolution(hunk_id) {
            Ok(()) => {
                // Only push undo if there was a resolution to clear
                if prev.is_some() {
                    app.undo_stack.push(hunk_id, prev, "Clear resolution");
                }
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
    let Some(entry) = app.undo_stack.pop() else {
        app.set_status_message("Nothing to undo");
        return;
    };

    if let Some(session) = &mut app.session {
        let result = if let Some(resolution) = entry.previous_resolution {
            // Restore previous resolution
            session.set_resolution(entry.hunk_id, resolution)
        } else {
            // Was unresolved before
            session.clear_resolution(entry.hunk_id)
        };

        match result {
            Ok(()) => app.set_status_message(&format!("Undid: {}", entry.action)),
            Err(_) => app.set_status_message("Failed to undo"),
        }
    }
}

/// Applies a resolution to the current hunk with undo support.
///
/// This is a helper that handles the common pattern of:
/// 1. Getting the current hunk and its previous resolution
/// 2. Pushing an undo entry
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

    // Apply resolution and only push undo / set status on success
    if let Some(session) = app.session.as_mut() {
        match session.set_resolution(hunk_id, resolution) {
            Ok(()) => {
                app.undo_stack.push(hunk_id, prev, action);
                app.set_status_message(action);
            }
            Err(_) => {
                app.set_status_message("Failed to apply resolution");
            }
        }
    }
}
