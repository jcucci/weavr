//! AST merge integration for the TUI.
//!
//! Unlike AI suggestions (which use channel-based async workers), AST merging
//! is synchronous CPU-bound work with no I/O. When the user presses the AST
//! keybinding, we call `AstStrategy::try_resolve` directly on the main thread
//! and store the result immediately.

use std::collections::HashMap;

use weavr_core::{HunkId, Resolution};

use crate::resolution;
use crate::App;

// ---------------------------------------------------------------------------
// AstSuggestion / AstState
// ---------------------------------------------------------------------------

/// A pending AST merge suggestion displayed as ghost text.
#[derive(Debug, Clone)]
pub struct AstSuggestion {
    /// Which hunk this suggestion is for.
    pub hunk_id: HunkId,
    /// The suggested resolution.
    pub resolution: Resolution,
    /// Confidence score (0–100).
    pub confidence: Option<u8>,
    /// Description of the merge (e.g., "Merged 3 imports").
    pub description: String,
}

/// Tracks AST merge suggestion state for UI rendering.
#[derive(Debug, Default)]
pub struct AstState {
    /// Suggestions keyed by hunk ID.
    pub suggestions: HashMap<HunkId, AstSuggestion>,
}

impl AstState {
    /// Returns true if there is an AST suggestion ready for the given hunk.
    #[must_use]
    pub fn has_suggestion_for(&self, hunk_id: HunkId) -> bool {
        self.suggestions.contains_key(&hunk_id)
    }

    /// Returns the suggestion for the given hunk, if any.
    #[must_use]
    pub fn suggestion_for(&self, hunk_id: HunkId) -> Option<&AstSuggestion> {
        self.suggestions.get(&hunk_id)
    }

    /// Clears all suggestions.
    pub fn clear(&mut self) {
        self.suggestions.clear();
    }
}

// ---------------------------------------------------------------------------
// Action functions
// ---------------------------------------------------------------------------

/// Requests an AST merge suggestion for the current hunk.
///
/// Calls `AstStrategy::try_resolve` synchronously. The result is stored
/// in `app.ast_state.suggestions` for ghost-text rendering.
#[cfg(feature = "ast")]
pub fn request_suggestion(app: &mut App) {
    let Some(ref ast_strategy) = app.ast_strategy else {
        app.set_status_message("AST merging not available");
        return;
    };
    let Some(session) = &app.session else {
        return;
    };
    let Some(hunk) = session.hunks().get(app.current_hunk_index) else {
        return;
    };
    // Don't re-request if we already have a suggestion for this hunk
    if app.ast_state.has_suggestion_for(hunk.id) {
        return;
    }

    let hunk_id = hunk.id;
    let file_path = session.input().left.path.clone();
    let language = weavr_core::detect_language(&file_path);

    match ast_strategy.try_resolve(hunk, &file_path, language) {
        Ok(Some(resolution)) => {
            let confidence = resolution.metadata.confidence;
            let description = resolution.metadata.notes.clone().unwrap_or_default();
            app.ast_state.suggestions.insert(
                hunk_id,
                AstSuggestion {
                    hunk_id,
                    resolution,
                    confidence,
                    description,
                },
            );
            let conf_str = confidence
                .map(|c| format!(" ({c}% confidence)"))
                .unwrap_or_default();
            app.set_status_message(&format!(
                "AST merge ready{conf_str} - Enter to accept, Esc to dismiss"
            ));
        }
        Ok(None) => {
            app.set_status_message("AST: cannot merge this hunk structurally");
        }
        Err(e) => {
            app.set_status_message(&format!("AST error: {e}"));
        }
    }
}

/// Stub when `ast` feature is disabled.
#[cfg(not(feature = "ast"))]
pub fn request_suggestion(app: &mut App) {
    app.set_status_message("AST merging not compiled (enable `ast` feature)");
}

/// Requests AST merge suggestions for all unresolved hunks.
#[cfg(feature = "ast")]
pub fn request_all_suggestions(app: &mut App) {
    let Some(strategy) = &app.ast_strategy else {
        app.set_status_message("AST merging not available");
        return;
    };

    // Gather file metadata and candidate hunk IDs without cloning full hunks.
    let (file_path, language, hunk_ids) = {
        let Some(session) = &app.session else {
            return;
        };
        let file_path = session.input().left.path.clone();
        let language = weavr_core::detect_language(&file_path);
        let ids: Vec<HunkId> = session
            .hunks()
            .iter()
            .filter(|h| matches!(h.state, weavr_core::HunkState::Unresolved))
            .filter(|h| !app.ast_state.has_suggestion_for(h.id))
            .map(|h| h.id)
            .collect();
        (file_path, language, ids)
    };

    if hunk_ids.is_empty() {
        app.set_status_message("No unresolved hunks to merge");
        return;
    }

    let total = hunk_ids.len();
    let mut count = 0;
    let mut errors = 0;

    for hunk_id in &hunk_ids {
        // Scope borrows so we can mutate ast_state after try_resolve returns.
        let result = {
            let session = app.session.as_ref().unwrap();
            session
                .hunks()
                .iter()
                .find(|h| h.id == *hunk_id)
                .map(|h| strategy.try_resolve(h, &file_path, language))
        };

        match result {
            Some(Ok(Some(resolution))) => {
                let confidence = resolution.metadata.confidence;
                let description = resolution.metadata.notes.clone().unwrap_or_default();
                app.ast_state.suggestions.insert(
                    *hunk_id,
                    AstSuggestion {
                        hunk_id: *hunk_id,
                        resolution,
                        confidence,
                        description,
                    },
                );
                count += 1;
            }
            Some(Err(_)) => {
                errors += 1;
            }
            // Ok(None) = merger declined, None = hunk not found
            _ => {}
        }
    }

    let msg = match (count, errors) {
        (0, 0) => "AST: no hunks could be merged structurally".to_string(),
        (0, e) => format!(
            "AST: no hunks merged ({e} parse error{})",
            if e == 1 { "" } else { "s" }
        ),
        (c, 0) => format!("AST merge: {c}/{total} hunks resolved"),
        (c, e) => format!(
            "AST merge: {c}/{total} hunks resolved ({e} error{})",
            if e == 1 { "" } else { "s" }
        ),
    };
    app.set_status_message(&msg);
}

/// Stub when `ast` feature is disabled.
#[cfg(not(feature = "ast"))]
pub fn request_all_suggestions(app: &mut App) {
    app.set_status_message("AST merging not compiled (enable `ast` feature)");
}

/// Accepts the current AST suggestion, applying it as a resolution.
pub fn accept_suggestion(app: &mut App) {
    let Some(hunk) = app.current_hunk() else {
        return;
    };
    let hunk_id = hunk.id;
    let Some(suggestion) = app.ast_state.suggestions.remove(&hunk_id) else {
        return;
    };
    let resolution = suggestion.resolution;
    resolution::apply_resolution(app, "AST merge (accepted)", |_hunk| resolution);
}

/// Dismisses the current AST suggestion without applying it.
pub fn dismiss_suggestion(app: &mut App) {
    let Some(hunk) = app.current_hunk() else {
        return;
    };
    let hunk_id = hunk.id;
    if app.ast_state.suggestions.remove(&hunk_id).is_some() {
        app.set_status_message("AST suggestion dismissed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ast_state_default_is_empty() {
        let state = AstState::default();
        assert!(state.suggestions.is_empty());
    }

    #[test]
    fn ast_state_has_suggestion_for_matching_hunk() {
        let mut state = AstState::default();
        let hunk_id = HunkId(42);
        state.suggestions.insert(
            hunk_id,
            AstSuggestion {
                hunk_id,
                resolution: Resolution::manual("test".into()),
                confidence: Some(85),
                description: "Merged imports".into(),
            },
        );
        assert!(state.has_suggestion_for(hunk_id));
        assert!(!state.has_suggestion_for(HunkId(99)));
    }

    #[test]
    fn ast_state_suggestion_for_returns_ref() {
        let mut state = AstState::default();
        let hunk_id = HunkId(1);
        state.suggestions.insert(
            hunk_id,
            AstSuggestion {
                hunk_id,
                resolution: Resolution::manual("merged".into()),
                confidence: Some(90),
                description: "test".into(),
            },
        );
        let suggestion = state.suggestion_for(hunk_id).unwrap();
        assert_eq!(suggestion.confidence, Some(90));
        assert_eq!(suggestion.description, "test");
    }

    #[test]
    fn ast_state_clear_removes_suggestions() {
        let mut state = AstState::default();
        state.suggestions.insert(
            HunkId(1),
            AstSuggestion {
                hunk_id: HunkId(1),
                resolution: Resolution::manual("test".into()),
                confidence: None,
                description: String::new(),
            },
        );
        state.clear();
        assert!(state.suggestions.is_empty());
    }

    #[test]
    fn accept_suggestion_noop_without_hunk() {
        let mut app = App::new();
        accept_suggestion(&mut app);
        // No crash
    }

    #[test]
    fn dismiss_suggestion_noop_without_hunk() {
        let mut app = App::new();
        dismiss_suggestion(&mut app);
        assert!(app.status_message().is_none());
    }

    #[test]
    fn request_suggestion_without_ast_strategy() {
        let mut app = App::new();
        request_suggestion(&mut app);
        assert!(app.status_message().is_some());
    }

    #[test]
    fn request_all_without_ast_strategy() {
        let mut app = App::new();
        request_all_suggestions(&mut app);
        assert!(app.status_message().is_some());
    }
}
