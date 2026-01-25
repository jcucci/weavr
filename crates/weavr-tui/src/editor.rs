//! External editor integration.
//!
//! This module handles:
//! - Preparing content for external editing
//! - Applying edited content as manual resolution

use weavr_core::Resolution;

use crate::resolution;
use crate::App;

/// Prepares content for external editor and sets pending state.
/// Returns true if editor should be launched.
pub fn prepare_editor(app: &mut App) -> bool {
    if let Some(content) = get_current_hunk_content(app) {
        app.editor_pending = Some(content);
        true
    } else {
        app.set_status_message("No hunk to edit");
        false
    }
}

/// Takes the pending editor content, clearing the pending state.
pub fn take_editor_pending(app: &mut App) -> Option<String> {
    app.editor_pending.take()
}

/// Applies content returned from the external editor as a manual resolution.
pub fn apply_editor_result(app: &mut App, content: &str) {
    let owned = content.to_string();
    resolution::apply_resolution(app, "Manual edit", |_hunk| Resolution::manual(owned.clone()));
}

/// Gets the content of the current hunk for editing.
fn get_current_hunk_content(app: &App) -> Option<String> {
    app.session.as_ref().and_then(|session| {
        session.hunks().get(app.current_hunk_index).map(|hunk| {
            // If already resolved, use that; otherwise combine left/right
            if let Some(resolution) = session.resolutions().get(&hunk.id) {
                resolution.content.clone()
            } else {
                format!(
                    "<<<<<<< OURS\n{}\n=======\n{}\n>>>>>>> THEIRS",
                    hunk.left.text, hunk.right.text
                )
            }
        })
    })
}
