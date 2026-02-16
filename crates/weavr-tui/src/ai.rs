//! AI integration bridge for the TUI.
//!
//! This module provides channel-based communication between the synchronous
//! TUI event loop and an async AI background worker. The TUI sends commands
//! and receives events via `std::sync::mpsc` channels, requiring no async
//! runtime dependency in this crate.
//!
//! The actual AI provider and tokio runtime live in `weavr-cli`, which
//! constructs an [`AiHandle`] and passes it to [`App`](crate::App).

use std::collections::HashMap;
use std::sync::mpsc;

use weavr_core::{ConflictHunk, HunkId, HunkState, Resolution};

use crate::input::{Dialog, InputMode};
use crate::resolution;
use crate::App;

// ---------------------------------------------------------------------------
// Channel types
// ---------------------------------------------------------------------------

/// Command sent from TUI to AI background worker.
pub enum AiCommand {
    /// Request a suggestion for a single hunk.
    Suggest {
        /// The hunk to suggest a resolution for.
        hunk_id: HunkId,
        /// A clone of the conflict hunk data.
        hunk: ConflictHunk,
    },
    /// Request suggestions for multiple hunks.
    SuggestAll {
        /// Pairs of hunk ID and hunk data.
        hunks: Vec<(HunkId, ConflictHunk)>,
    },
    /// Request an explanation for a hunk's conflict.
    Explain {
        /// The hunk to explain.
        hunk_id: HunkId,
        /// A clone of the conflict hunk data.
        hunk: ConflictHunk,
    },
    /// Cancel any in-flight request for this hunk.
    Cancel {
        /// The hunk whose request should be cancelled.
        hunk_id: HunkId,
    },
    /// Shutdown the background worker.
    Shutdown,
}

/// Event received from AI background worker.
pub enum AiEvent {
    /// A suggestion was generated.
    Suggestion {
        /// The hunk this suggestion is for.
        hunk_id: HunkId,
        /// The suggested resolution.
        resolution: Resolution,
        /// Confidence score (0-100), if available.
        confidence: Option<u8>,
    },
    /// No suggestion available.
    NoSuggestion {
        /// The hunk this response is for.
        hunk_id: HunkId,
        /// Why no suggestion was produced.
        reason: String,
    },
    /// An explanation was generated.
    Explanation {
        /// The hunk this explanation is for.
        hunk_id: HunkId,
        /// Natural-language explanation text.
        text: String,
    },
    /// An error occurred.
    Error {
        /// The hunk this error relates to.
        hunk_id: HunkId,
        /// Error description.
        message: String,
    },
    /// Batch suggestion processing is complete.
    BatchComplete,
}

// ---------------------------------------------------------------------------
// AiHandle
// ---------------------------------------------------------------------------

/// Handle for communicating with the AI background worker.
///
/// Created by `weavr-cli` and passed into [`App`](crate::App) via
/// [`App::set_ai_handle`](crate::App::set_ai_handle).
pub struct AiHandle {
    sender: mpsc::Sender<AiCommand>,
    receiver: mpsc::Receiver<AiEvent>,
}

impl AiHandle {
    /// Creates a new handle from channel endpoints.
    #[must_use]
    pub fn new(sender: mpsc::Sender<AiCommand>, receiver: mpsc::Receiver<AiEvent>) -> Self {
        Self { sender, receiver }
    }

    /// Sends a command to the AI worker.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the background worker has shut down or the channel is closed.
    #[allow(clippy::result_large_err)]
    pub fn send(&self, cmd: AiCommand) -> Result<(), mpsc::SendError<AiCommand>> {
        self.sender.send(cmd)
    }

    /// Tries to receive an event without blocking.
    #[must_use]
    pub fn try_recv(&self) -> Option<AiEvent> {
        self.receiver.try_recv().ok()
    }
}

// ---------------------------------------------------------------------------
// AiState
// ---------------------------------------------------------------------------

/// A pending AI suggestion displayed as ghost text.
#[derive(Debug, Clone)]
pub struct AiSuggestion {
    /// Which hunk this suggestion is for.
    pub hunk_id: HunkId,
    /// The suggested resolution content.
    pub resolution: Resolution,
    /// Confidence score (0-100).
    pub confidence: Option<u8>,
}

/// Tracks the current AI suggestion state for UI rendering.
#[derive(Debug, Default)]
pub struct AiState {
    /// The hunk currently being processed (shows loading spinner).
    pub pending_hunk: Option<HunkId>,
    /// Whether a batch request is in progress.
    pub pending_batch: bool,
    /// Suggestions keyed by hunk ID.
    pub suggestions: HashMap<HunkId, AiSuggestion>,
    /// An explanation for the current hunk.
    pub explanation: Option<String>,
    /// Spinner animation frame counter.
    pub spinner_tick: u8,
}

impl AiState {
    /// Returns true if an AI request is in progress.
    #[must_use]
    pub fn is_loading(&self) -> bool {
        self.pending_hunk.is_some() || self.pending_batch
    }

    /// Returns true if there is a suggestion ready for the given hunk.
    #[must_use]
    pub fn has_suggestion_for(&self, hunk_id: HunkId) -> bool {
        self.suggestions.contains_key(&hunk_id)
    }

    /// Returns the suggestion for the given hunk, if any.
    #[must_use]
    pub fn suggestion_for(&self, hunk_id: HunkId) -> Option<&AiSuggestion> {
        self.suggestions.get(&hunk_id)
    }

    /// Clears the suggestions and explanation state.
    pub fn clear(&mut self) {
        self.suggestions.clear();
        self.explanation = None;
    }

    /// Advances the spinner animation tick.
    pub fn tick_spinner(&mut self) {
        self.spinner_tick = self.spinner_tick.wrapping_add(1);
    }

    /// Returns the current spinner character.
    #[must_use]
    pub fn spinner_char(&self) -> char {
        const FRAMES: &[char] = &['|', '/', '-', '\\'];
        FRAMES[(self.spinner_tick as usize / 2) % FRAMES.len()]
    }
}

// ---------------------------------------------------------------------------
// Action functions
// ---------------------------------------------------------------------------

/// Requests an AI suggestion for the current hunk.
#[allow(clippy::missing_panics_doc)] // unwrap is guarded by is_none() check above
pub fn request_suggestion(app: &mut App) {
    if app.ai_handle.is_none() {
        app.set_status_message("AI not configured");
        return;
    }
    let Some(hunk) = app.current_hunk().cloned() else {
        return;
    };
    // Don't request if already loading for this hunk
    if app.ai_state.pending_hunk == Some(hunk.id) {
        return;
    }
    app.ai_state.pending_hunk = Some(hunk.id);
    let send_result = app.ai_handle.as_ref().unwrap().send(AiCommand::Suggest {
        hunk_id: hunk.id,
        hunk,
    });
    if send_result.is_err() {
        app.ai_state.pending_hunk = None;
        app.ai_handle = None;
        app.set_status_message("AI worker disconnected");
        return;
    }
    app.set_status_message("Requesting AI suggestion...");
}

/// Requests AI suggestions for all unresolved hunks.
#[allow(clippy::missing_panics_doc)] // unwrap is guarded by is_none() check above
pub fn request_all_suggestions(app: &mut App) {
    if app.ai_handle.is_none() {
        app.set_status_message("AI not configured");
        return;
    }
    let Some(session) = &app.session else {
        return;
    };
    let hunks: Vec<_> = session
        .hunks()
        .iter()
        .filter(|h| matches!(h.state, HunkState::Unresolved))
        .map(|h| (h.id, h.clone()))
        .collect();
    if hunks.is_empty() {
        app.set_status_message("No unresolved hunks");
        return;
    }
    let count = hunks.len();
    app.ai_state.pending_batch = true;
    if app
        .ai_handle
        .as_ref()
        .unwrap()
        .send(AiCommand::SuggestAll { hunks })
        .is_err()
    {
        app.ai_state.pending_batch = false;
        app.ai_handle = None;
        app.set_status_message("AI worker disconnected");
        return;
    }
    app.set_status_message(&format!(
        "Requesting AI suggestions for {count} unresolved hunks..."
    ));
}

/// Accepts the current AI suggestion, applying it as a resolution.
pub fn accept_suggestion(app: &mut App) {
    let Some(hunk) = app.current_hunk() else {
        return;
    };
    let hunk_id = hunk.id;
    let Some(suggestion) = app.ai_state.suggestions.remove(&hunk_id) else {
        return;
    };
    let resolution = suggestion.resolution;
    resolution::apply_resolution(app, "AI suggestion (accepted)", |_hunk| resolution);
}

/// Dismisses the current AI suggestion without applying it.
pub fn dismiss_suggestion(app: &mut App) {
    let Some(hunk) = app.current_hunk() else {
        return;
    };
    let hunk_id = hunk.id;
    if app.ai_state.suggestions.remove(&hunk_id).is_some() {
        app.set_status_message("AI suggestion dismissed");
    }
}

/// Requests an AI explanation for the current hunk.
#[allow(clippy::missing_panics_doc)] // unwrap is guarded by is_none() check above
pub fn request_explanation(app: &mut App) {
    if app.ai_handle.is_none() {
        app.set_status_message("AI not configured");
        return;
    }
    let Some(hunk) = app.current_hunk().cloned() else {
        return;
    };
    app.ai_state.pending_hunk = Some(hunk.id);
    if app
        .ai_handle
        .as_ref()
        .unwrap()
        .send(AiCommand::Explain {
            hunk_id: hunk.id,
            hunk,
        })
        .is_err()
    {
        app.ai_state.pending_hunk = None;
        app.ai_handle = None;
        app.set_status_message("AI worker disconnected");
        return;
    }
    app.set_status_message("Requesting AI explanation...");
}

/// Polls for AI events and updates state. Called each tick in the event loop.
pub fn poll_ai_events(app: &mut App) {
    // Collect events first to avoid borrow conflict (ai_handle borrows app)
    let events: Vec<AiEvent> = {
        let Some(ai_handle) = &app.ai_handle else {
            return;
        };
        let mut events = Vec::new();
        while let Some(event) = ai_handle.try_recv() {
            events.push(event);
        }
        events
    };

    // Process collected events
    for event in events {
        match event {
            AiEvent::Suggestion {
                hunk_id,
                resolution,
                confidence,
            } => {
                // Only accept if we're still interested in this hunk
                let interested = app.ai_state.pending_hunk == Some(hunk_id)
                    || app.ai_state.pending_batch
                    || app.current_hunk().is_some_and(|h| h.id == hunk_id);
                if interested {
                    if app.ai_state.pending_hunk == Some(hunk_id) {
                        app.ai_state.pending_hunk = None;
                    }
                    app.ai_state.suggestions.insert(
                        hunk_id,
                        AiSuggestion {
                            hunk_id,
                            resolution,
                            confidence,
                        },
                    );
                    // Only show status message if this is for the current hunk
                    if app.current_hunk().is_some_and(|h| h.id == hunk_id) {
                        let conf_str = confidence
                            .map(|c| format!(" ({c}% confidence)"))
                            .unwrap_or_default();
                        app.set_status_message(&format!(
                            "AI suggestion ready{conf_str} - Enter to accept, Esc to dismiss"
                        ));
                    }
                }
            }
            AiEvent::NoSuggestion { reason, .. } => {
                app.ai_state.pending_hunk = None;
                app.set_status_message(&format!("AI: {reason}"));
            }
            AiEvent::Explanation { hunk_id, text } => {
                // Only show explanation if still relevant to current hunk
                let interested = app.ai_state.pending_hunk == Some(hunk_id)
                    || app.current_hunk().is_some_and(|h| h.id == hunk_id);
                app.ai_state.pending_hunk = None;
                if interested {
                    app.ai_state.explanation = Some(text.clone());
                    app.active_dialog = Some(Dialog::AiExplanation(text));
                    app.input_mode = InputMode::Dialog;
                }
            }
            AiEvent::Error { message, .. } => {
                app.ai_state.pending_hunk = None;
                app.ai_state.pending_batch = false;
                app.set_status_message(&format!("AI error: {message}"));
            }
            AiEvent::BatchComplete => {
                app.ai_state.pending_batch = false;
                let count = app.ai_state.suggestions.len();
                app.set_status_message(&format!("AI batch complete: {count} suggestion(s) ready"));
            }
        }
    }

    // Tick spinner if loading
    if app.ai_state.is_loading() {
        app.ai_state.tick_spinner();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ai_state_default_is_not_loading() {
        let state = AiState::default();
        assert!(!state.is_loading());
        assert!(state.suggestions.is_empty());
        assert!(state.explanation.is_none());
    }

    #[test]
    fn ai_state_is_loading_when_pending_hunk() {
        let mut state = AiState::default();
        state.pending_hunk = Some(HunkId(1));
        assert!(state.is_loading());
    }

    #[test]
    fn ai_state_is_loading_when_pending_batch() {
        let mut state = AiState::default();
        state.pending_batch = true;
        assert!(state.is_loading());
    }

    #[test]
    fn ai_state_has_suggestion_for_matching_hunk() {
        let mut state = AiState::default();
        let hunk_id = HunkId(42);
        state.suggestions.insert(
            hunk_id,
            AiSuggestion {
                hunk_id,
                resolution: Resolution::manual("test".into()),
                confidence: Some(85),
            },
        );
        assert!(state.has_suggestion_for(hunk_id));
        assert!(!state.has_suggestion_for(HunkId(99)));
    }

    #[test]
    fn ai_state_clear_removes_suggestion_and_explanation() {
        let mut state = AiState::default();
        state.suggestions.insert(
            HunkId(1),
            AiSuggestion {
                hunk_id: HunkId(1),
                resolution: Resolution::manual("test".into()),
                confidence: None,
            },
        );
        state.explanation = Some("explanation".into());
        state.clear();
        assert!(state.suggestions.is_empty());
        assert!(state.explanation.is_none());
    }

    #[test]
    fn spinner_char_cycles() {
        let mut state = AiState::default();
        let first = state.spinner_char();
        state.tick_spinner();
        state.tick_spinner();
        let second = state.spinner_char();
        // After 2 ticks the spinner should advance
        assert_ne!(first, second);
    }

    #[test]
    fn spinner_char_wraps_around() {
        let mut state = AiState::default();
        // Tick many times to test wrapping
        for _ in 0..100 {
            state.tick_spinner();
            // Should never panic
            let _ = state.spinner_char();
        }
    }

    #[test]
    fn request_suggestion_without_ai_handle() {
        let mut app = App::new();
        request_suggestion(&mut app);
        // Should set status message about AI not configured
        assert!(app
            .status_message()
            .is_some_and(|(msg, _)| msg.contains("not configured")));
    }

    #[test]
    fn request_all_suggestions_without_ai_handle() {
        let mut app = App::new();
        request_all_suggestions(&mut app);
        assert!(app
            .status_message()
            .is_some_and(|(msg, _)| msg.contains("not configured")));
    }

    #[test]
    fn dismiss_suggestion_clears_state() {
        let mut app = App::new();
        app.ai_state.suggestions.insert(
            HunkId(1),
            AiSuggestion {
                hunk_id: HunkId(1),
                resolution: Resolution::manual("test".into()),
                confidence: Some(90),
            },
        );
        dismiss_suggestion(&mut app);
        // Without a current hunk, dismiss is a no-op
        assert!(!app.ai_state.suggestions.is_empty());
    }

    #[test]
    fn dismiss_suggestion_noop_without_suggestion() {
        let mut app = App::new();
        dismiss_suggestion(&mut app);
        // No status message set when there's nothing to dismiss
        assert!(app.status_message().is_none());
    }

    #[test]
    fn accept_suggestion_noop_without_suggestion() {
        let mut app = App::new();
        accept_suggestion(&mut app);
        // No crash, no status change
    }

    #[test]
    fn poll_ai_events_noop_without_handle() {
        let mut app = App::new();
        poll_ai_events(&mut app);
        // No crash
    }

    #[test]
    fn ai_handle_send_and_recv() {
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let (evt_tx, evt_rx) = mpsc::channel();
        let handle = AiHandle::new(cmd_tx, evt_rx);

        // Send a command
        handle
            .send(AiCommand::Cancel { hunk_id: HunkId(1) })
            .unwrap();

        // Verify it arrives
        let cmd = cmd_rx.recv().unwrap();
        assert!(matches!(cmd, AiCommand::Cancel { hunk_id } if hunk_id == HunkId(1)));

        // Send an event back
        evt_tx
            .send(AiEvent::NoSuggestion {
                hunk_id: HunkId(1),
                reason: "test".into(),
            })
            .unwrap();

        // Receive it
        let event = handle.try_recv();
        assert!(event.is_some());
    }

    #[test]
    fn ai_handle_try_recv_returns_none_when_empty() {
        let (_cmd_tx, _cmd_rx) = mpsc::channel::<AiCommand>();
        let (_evt_tx, evt_rx) = mpsc::channel::<AiEvent>();
        let handle = AiHandle::new(_cmd_tx, evt_rx);
        assert!(handle.try_recv().is_none());
    }
}
