//! Merge session management.
//!
//! All types in this module are **stable** and covered by semantic versioning.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{
    parse_conflict_markers, ApplyError, CompletionError, ConflictHunk, FileVersion, HunkId,
    HunkState, LifecycleError, MergeInput, MergeResult, MergeSummary, ParseError, ParsedConflict,
    Resolution, ResolutionError, Segment, ValidationError,
};

/// The state of a merge session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum MergeState {
    /// Raw input provided, no parsing performed.
    #[default]
    Uninitialized,
    /// Conflict markers parsed, hunks created.
    Parsed,
    /// User is applying resolutions.
    Active,
    /// All hunks resolved, no output generated yet.
    FullyResolved,
    /// Resolutions applied, output text produced.
    Applied,
    /// Output contains no conflict markers, file is valid.
    Validated,
    /// Final result generated.
    Completed,
}

/// Represents a single merge attempt for a file.
#[derive(Debug, Clone)]
pub struct MergeSession {
    /// The original merge inputs.
    input: MergeInput,
    /// Parsed conflict regions.
    hunks: Vec<ConflictHunk>,
    /// File structure (clean segments and conflict references).
    segments: Vec<Segment>,
    /// Current session state.
    state: MergeState,
    /// Applied resolutions.
    resolutions: HashMap<HunkId, Resolution>,
}

impl MergeSession {
    /// Creates a new merge session from input.
    ///
    /// Note: This constructor is intended for future 3-way merge generation.
    /// For parsing existing Git conflicts, use [`from_conflicted`](Self::from_conflicted).
    ///
    /// # Errors
    ///
    /// Returns `ParseError` if conflict markers cannot be parsed.
    pub fn new(input: MergeInput) -> Result<Self, ParseError> {
        // Placeholder implementation - actual parsing will be implemented later
        Ok(Self {
            input,
            hunks: Vec::new(),
            segments: Vec::new(),
            state: MergeState::Parsed,
            resolutions: HashMap::new(),
        })
    }

    /// Creates a merge session by parsing existing conflict markers.
    ///
    /// This is the primary entry point for resolving Git merge conflicts.
    /// The content should be the working copy file containing conflict markers.
    ///
    /// # Arguments
    ///
    /// * `content` - File content with Git conflict markers.
    /// * `path` - Path for identification (not used for I/O).
    ///
    /// # Errors
    ///
    /// Returns `ParseError` if conflict markers are malformed.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use meldr_core::MergeSession;
    ///
    /// let content = r#"before
    /// <<<<<<< HEAD
    /// left
    /// =======
    /// right
    /// >>>>>>> feature
    /// after"#;
    ///
    /// let session = MergeSession::from_conflicted(content, PathBuf::from("file.rs")).unwrap();
    /// assert_eq!(session.hunks().len(), 1);
    /// ```
    pub fn from_conflicted(content: &str, path: PathBuf) -> Result<Self, ParseError> {
        let ParsedConflict { hunks, segments } = parse_conflict_markers(content)?;

        // Determine state based on whether conflicts were found
        let state = if hunks.is_empty() {
            MergeState::Validated // No conflicts = already clean
        } else {
            MergeState::Parsed
        };

        // Store segments for later reconstruction
        // For now, we store the original content in the input
        let input = MergeInput {
            left: FileVersion {
                path: path.clone(),
                content: content.to_string(),
            },
            right: FileVersion {
                path,
                content: String::new(), // Not used for conflict parsing
            },
            base: None,
        };

        Ok(Self {
            input,
            hunks,
            state,
            resolutions: HashMap::new(),
            segments,
        })
    }

    /// Returns all conflict hunks.
    #[must_use]
    pub fn hunks(&self) -> &[ConflictHunk] {
        &self.hunks
    }

    /// Returns the current session state.
    #[must_use]
    pub fn state(&self) -> MergeState {
        self.state
    }

    /// Returns the original merge input.
    #[must_use]
    pub fn input(&self) -> &MergeInput {
        &self.input
    }

    /// Returns the resolutions map.
    #[must_use]
    pub fn resolutions(&self) -> &HashMap<HunkId, Resolution> {
        &self.resolutions
    }

    /// Returns the file segments (clean text and conflict references).
    ///
    /// This preserves the file structure for reconstruction after resolution.
    #[must_use]
    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }

    /// Checks if all hunks are resolved.
    #[must_use]
    pub fn is_fully_resolved(&self) -> bool {
        self.hunks
            .iter()
            .all(|h| matches!(h.state, HunkState::Resolved(_)))
    }

    /// Returns the IDs of unresolved hunks.
    #[must_use]
    pub fn unresolved_hunks(&self) -> Vec<HunkId> {
        self.hunks
            .iter()
            .filter(|h| !matches!(h.state, HunkState::Resolved(_)))
            .map(|h| h.id)
            .collect()
    }

    // --- State Transition Helpers ---

    /// Checks if a transition is valid according to the state machine.
    #[allow(clippy::unnested_or_patterns)] // Keep patterns expanded for readability
    fn can_transition(from: MergeState, to: MergeState) -> bool {
        matches!(
            (from, to),
            (MergeState::Uninitialized, MergeState::Parsed)
                | (MergeState::Parsed, MergeState::Active)
                | (MergeState::Active, MergeState::FullyResolved)
                | (MergeState::FullyResolved, MergeState::Active)
                | (MergeState::FullyResolved, MergeState::Applied)
                | (MergeState::Applied, MergeState::Validated)
                | (MergeState::Validated, MergeState::Completed)
        )
    }

    /// Returns a human-readable reason why a transition failed.
    #[allow(dead_code)]
    fn transition_failure_reason(&self, to: MergeState) -> String {
        match (self.state, to) {
            (MergeState::Uninitialized, _) => "session not yet parsed".to_string(),
            (MergeState::Parsed, MergeState::Applied) => {
                "cannot apply before resolving conflicts".to_string()
            }
            (MergeState::Active, MergeState::Applied) => "not all hunks are resolved".to_string(),
            (MergeState::FullyResolved, MergeState::Completed) => {
                "must apply and validate before completing".to_string()
            }
            (MergeState::Applied, MergeState::Completed) => {
                "must validate before completing".to_string()
            }
            (_, MergeState::Parsed) => "session already parsed".to_string(),
            _ => format!("transition not allowed from {:?}", self.state),
        }
    }

    /// Attempts to transition to a new state.
    ///
    /// This is the only method that should modify `self.state` directly.
    #[allow(dead_code)]
    fn transition(&mut self, to: MergeState) -> Result<(), LifecycleError> {
        if Self::can_transition(self.state, to) {
            self.state = to;
            Ok(())
        } else {
            Err(LifecycleError::InvalidTransition {
                from: self.state,
                to,
                reason: self.transition_failure_reason(to),
            })
        }
    }

    /// Updates state based on hunk resolution status.
    ///
    /// Called after `set_resolution` or `clear_resolution` to maintain state consistency.
    fn update_state_from_hunks(&mut self) {
        let all_resolved = self.is_fully_resolved();

        match self.state {
            MergeState::Parsed if !self.hunks.is_empty() => {
                // Resolution activity moves from Parsed to either Active or FullyResolved
                if all_resolved {
                    self.state = MergeState::FullyResolved;
                } else {
                    self.state = MergeState::Active;
                }
            }
            MergeState::Active if all_resolved => {
                // All resolved moves to FullyResolved
                self.state = MergeState::FullyResolved;
            }
            MergeState::FullyResolved if !all_resolved => {
                // Resolution cleared, back to Active
                self.state = MergeState::Active;
            }
            _ => {}
        }
    }

    // --- Resolution Methods ---

    /// Applies a resolution to a hunk.
    ///
    /// This method validates that the session is in a state that allows resolution,
    /// finds the hunk by ID, and applies the resolution. State transitions happen
    /// automatically based on hunk status.
    ///
    /// # Errors
    ///
    /// Returns `ResolutionError::HunkNotFound` if the hunk doesn't exist.
    /// Returns `ResolutionError::InvalidResolution` if the session state doesn't allow resolution.
    pub fn set_resolution(
        &mut self,
        hunk_id: HunkId,
        resolution: Resolution,
    ) -> Result<(), ResolutionError> {
        // Check state allows resolution
        match self.state {
            MergeState::Parsed | MergeState::Active | MergeState::FullyResolved => {}
            state => {
                return Err(ResolutionError::InvalidResolution(format!(
                    "cannot set resolution in state {state:?}"
                )));
            }
        }

        // Find and update the hunk
        let hunk = self
            .hunks
            .iter_mut()
            .find(|h| h.id == hunk_id)
            .ok_or(ResolutionError::HunkNotFound(hunk_id))?;

        hunk.state = HunkState::Resolved(resolution.clone());
        self.resolutions.insert(hunk_id, resolution);

        // Update session state based on hunk status
        self.update_state_from_hunks();

        Ok(())
    }

    /// Clears the resolution for a hunk, returning it to `Unresolved` state.
    ///
    /// This enables undo/retry workflows. State transitions happen automatically
    /// based on hunk status.
    ///
    /// # Errors
    ///
    /// Returns `ResolutionError::HunkNotFound` if the hunk doesn't exist.
    /// Returns `ResolutionError::InvalidResolution` if the session state doesn't allow clearing.
    pub fn clear_resolution(&mut self, hunk_id: HunkId) -> Result<(), ResolutionError> {
        // Check state allows clearing resolution
        match self.state {
            MergeState::Parsed | MergeState::Active | MergeState::FullyResolved => {}
            state => {
                return Err(ResolutionError::InvalidResolution(format!(
                    "cannot clear resolution in state {state:?}"
                )));
            }
        }

        // Find and update the hunk
        let hunk = self
            .hunks
            .iter_mut()
            .find(|h| h.id == hunk_id)
            .ok_or(ResolutionError::HunkNotFound(hunk_id))?;

        hunk.state = HunkState::Unresolved;
        self.resolutions.remove(&hunk_id);

        // Update session state based on hunk status
        self.update_state_from_hunks();

        Ok(())
    }

    // --- Lifecycle Methods ---

    /// Generates the merged output text from all resolutions.
    ///
    /// This reconstructs the file by replacing conflict regions with their
    /// resolved content while preserving clean segments.
    ///
    /// # Errors
    ///
    /// Returns `ApplyError::NotFullyResolved` if not all hunks are resolved.
    pub fn apply(&mut self) -> Result<String, ApplyError> {
        // Validate state
        if self.state != MergeState::FullyResolved {
            return Err(ApplyError::NotFullyResolved);
        }

        // Generate output using shared helper
        let output = self.generate_output()?;

        // Transition to Applied state
        self.state = MergeState::Applied;

        Ok(output)
    }

    /// Validates that the session is ready for completion.
    ///
    /// Checks:
    /// - Session is in `Applied` state
    /// - No conflict markers remain in resolved content
    ///
    /// # Errors
    ///
    /// Returns `ValidationError::UnresolvedHunks` if not in correct state.
    /// Returns `ValidationError::MarkersRemain` if conflict markers found.
    pub fn validate(&mut self) -> Result<(), ValidationError> {
        // Check state is Applied
        if self.state != MergeState::Applied {
            let unresolved = self.unresolved_hunks();
            return Err(ValidationError::UnresolvedHunks(unresolved));
        }

        // Check for conflict markers in resolved content
        let marker_count = self.count_conflict_markers();
        if marker_count > 0 {
            return Err(ValidationError::MarkersRemain(marker_count));
        }

        // Transition to Validated
        self.state = MergeState::Validated;

        Ok(())
    }

    /// Counts conflict markers in all resolved content.
    ///
    /// Only counts markers at line starts to match Git's conflict marker format.
    fn count_conflict_markers(&self) -> usize {
        let mut count = 0;
        for hunk in &self.hunks {
            if let HunkState::Resolved(resolution) = &hunk.state {
                let has_markers = resolution.content.lines().any(|line| {
                    line.starts_with("<<<<<<<")
                        || line.starts_with("=======")
                        || line.starts_with(">>>>>>>")
                });
                if has_markers {
                    count += 1;
                }
            }
        }
        count
    }

    /// Finalizes the session and returns the immutable result.
    ///
    /// This consumes the session.
    ///
    /// # Errors
    ///
    /// Returns `CompletionError::LifecycleError` if the session is not in `Validated` state.
    pub fn complete(mut self) -> Result<MergeResult, CompletionError> {
        // Must be validated first
        if self.state != MergeState::Validated {
            return Err(CompletionError::LifecycleError(
                LifecycleError::OperationNotAllowed {
                    operation: "complete",
                    state: self.state,
                },
            ));
        }

        // Generate final output
        let content = self.generate_output()?;

        // Build summary
        let total_hunks = self.hunks.len();
        let resolved_hunks = self
            .hunks
            .iter()
            .filter(|h| matches!(h.state, HunkState::Resolved(_)))
            .count();

        // Transition to Completed
        self.state = MergeState::Completed;

        Ok(MergeResult {
            content,
            unresolved_hunks: vec![],
            warnings: vec![],
            summary: MergeSummary {
                total_hunks,
                resolved_hunks,
            },
        })
    }

    /// Internal helper to generate output from resolved hunks.
    fn generate_output(&self) -> Result<String, ApplyError> {
        let mut output = String::new();
        let segment_count = self.segments.len();

        for (i, segment) in self.segments.iter().enumerate() {
            match segment {
                Segment::Clean(text) => {
                    output.push_str(text);
                }
                Segment::Conflict(hunk_index) => {
                    let hunk = &self.hunks[*hunk_index];
                    if let HunkState::Resolved(resolution) = &hunk.state {
                        output.push_str(&resolution.content);
                    } else {
                        return Err(ApplyError::InternalError(format!(
                            "hunk {hunk_index} not resolved"
                        )));
                    }
                }
            }
            if i < segment_count - 1 {
                output.push('\n');
            }
        }

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::FileVersion;

    fn test_input() -> MergeInput {
        MergeInput {
            left: FileVersion {
                path: PathBuf::from("test.rs"),
                content: String::from("left content"),
            },
            right: FileVersion {
                path: PathBuf::from("test.rs"),
                content: String::from("right content"),
            },
            base: None,
        }
    }

    #[test]
    fn merge_state_default() {
        assert_eq!(MergeState::default(), MergeState::Uninitialized);
    }

    #[test]
    fn session_creation() {
        let session = MergeSession::new(test_input()).expect("should create session");
        assert_eq!(session.state(), MergeState::Parsed);
        assert!(session.hunks().is_empty());
    }

    #[test]
    fn session_fully_resolved_when_empty() {
        let session = MergeSession::new(test_input()).expect("should create session");
        // Empty hunks means fully resolved (vacuously true)
        assert!(session.is_fully_resolved());
    }

    #[test]
    fn session_unresolved_hunks_empty() {
        let session = MergeSession::new(test_input()).expect("should create session");
        assert!(session.unresolved_hunks().is_empty());
    }

    #[test]
    fn session_input_accessible() {
        let input = test_input();
        let session = MergeSession::new(input.clone()).expect("should create session");
        assert_eq!(session.input().left.content, "left content");
    }

    #[test]
    fn from_conflicted_parses_hunks() {
        let content = r"before
<<<<<<< HEAD
left content
=======
right content
>>>>>>> feature
after";

        let session =
            MergeSession::from_conflicted(content, PathBuf::from("test.rs")).expect("should parse");
        assert_eq!(session.hunks().len(), 1);
        assert_eq!(session.hunks()[0].left.text, "left content");
        assert_eq!(session.hunks()[0].right.text, "right content");
        assert_eq!(session.state(), MergeState::Parsed);
    }

    #[test]
    fn from_conflicted_no_conflicts_returns_validated() {
        let content = "clean content\nno conflicts here";

        let session = MergeSession::from_conflicted(content, PathBuf::from("clean.rs"))
            .expect("should parse");
        assert!(session.hunks().is_empty());
        assert_eq!(session.state(), MergeState::Validated);
    }

    #[test]
    fn from_conflicted_preserves_segments() {
        let content = r"before
<<<<<<< HEAD
left
=======
right
>>>>>>> feature
after";

        let session =
            MergeSession::from_conflicted(content, PathBuf::from("test.rs")).expect("should parse");
        assert_eq!(session.segments().len(), 3);
    }

    #[test]
    fn from_conflicted_stores_original_content() {
        let content = "some content";
        let session =
            MergeSession::from_conflicted(content, PathBuf::from("test.rs")).expect("should parse");
        assert_eq!(session.input().left.content, content);
    }

    #[test]
    fn from_conflicted_error_on_malformed() {
        let content = r"<<<<<<< HEAD
unclosed conflict";

        let result = MergeSession::from_conflicted(content, PathBuf::from("bad.rs"));
        assert!(result.is_err());
    }

    // --- Lifecycle Tests ---

    fn session_with_conflict() -> MergeSession {
        let content = r"before
<<<<<<< HEAD
left
=======
right
>>>>>>> feature
after";
        MergeSession::from_conflicted(content, PathBuf::from("test.rs")).unwrap()
    }

    fn session_with_multiple_conflicts() -> MergeSession {
        let content = r"before
<<<<<<< HEAD
left1
=======
right1
>>>>>>> feature
middle
<<<<<<< HEAD
left2
=======
right2
>>>>>>> feature
after";
        MergeSession::from_conflicted(content, PathBuf::from("test.rs")).unwrap()
    }

    // Valid transitions

    #[test]
    fn single_hunk_goes_directly_to_fully_resolved() {
        let mut session = session_with_conflict();
        assert_eq!(session.state(), MergeState::Parsed);

        let hunk_id = session.hunks()[0].id;
        let resolution = Resolution::accept_left(&session.hunks()[0]);
        session.set_resolution(hunk_id, resolution).unwrap();

        // With single hunk, goes directly to FullyResolved
        assert_eq!(session.state(), MergeState::FullyResolved);
    }

    #[test]
    fn multiple_hunks_transitions_through_active() {
        let mut session = session_with_multiple_conflicts();
        assert_eq!(session.state(), MergeState::Parsed);
        assert_eq!(session.hunks().len(), 2);

        // Resolve first hunk - should move to Active
        let hunk1_id = session.hunks()[0].id;
        let resolution1 = Resolution::accept_left(&session.hunks()[0]);
        session.set_resolution(hunk1_id, resolution1).unwrap();
        assert_eq!(session.state(), MergeState::Active);

        // Resolve second hunk - should move to FullyResolved
        let hunk2_id = session.hunks()[1].id;
        let resolution2 = Resolution::accept_right(&session.hunks()[1]);
        session.set_resolution(hunk2_id, resolution2).unwrap();
        assert_eq!(session.state(), MergeState::FullyResolved);
    }

    #[test]
    fn fully_resolved_to_active_on_clear() {
        let mut session = session_with_conflict();
        let hunk_id = session.hunks()[0].id;
        let resolution = Resolution::accept_left(&session.hunks()[0]);

        session.set_resolution(hunk_id, resolution).unwrap();
        assert_eq!(session.state(), MergeState::FullyResolved);

        session.clear_resolution(hunk_id).unwrap();
        assert_eq!(session.state(), MergeState::Active);
    }

    #[test]
    fn fully_resolved_to_applied() {
        let mut session = session_with_conflict();
        let hunk_id = session.hunks()[0].id;
        let resolution = Resolution::accept_left(&session.hunks()[0]);

        session.set_resolution(hunk_id, resolution).unwrap();
        let output = session.apply().unwrap();

        assert_eq!(session.state(), MergeState::Applied);
        assert!(output.contains("left"));
        assert!(!output.contains("<<<<<<<"));
    }

    #[test]
    fn applied_to_validated() {
        let mut session = session_with_conflict();
        let hunk_id = session.hunks()[0].id;
        let resolution = Resolution::accept_left(&session.hunks()[0]);

        session.set_resolution(hunk_id, resolution).unwrap();
        let _ = session.apply().unwrap();
        session.validate().unwrap();

        assert_eq!(session.state(), MergeState::Validated);
    }

    #[test]
    fn validated_to_completed() {
        let mut session = session_with_conflict();
        let hunk_id = session.hunks()[0].id;
        let resolution = Resolution::accept_left(&session.hunks()[0]);

        session.set_resolution(hunk_id, resolution).unwrap();
        let _ = session.apply().unwrap();
        session.validate().unwrap();
        let result = session.complete().unwrap();

        assert!(!result.content.is_empty());
        assert!(result.content.contains("before"));
        assert!(result.content.contains("left"));
        assert!(result.content.contains("after"));
        assert_eq!(result.summary.total_hunks, 1);
        assert_eq!(result.summary.resolved_hunks, 1);
    }

    #[test]
    fn full_lifecycle_roundtrip() {
        let mut session = session_with_conflict();

        // Parsed → FullyResolved (single hunk, resolving it completes all)
        let hunk_id = session.hunks()[0].id;
        let resolution = Resolution::accept_left(&session.hunks()[0]);
        session.set_resolution(hunk_id, resolution).unwrap();

        // FullyResolved → Applied
        let _ = session.apply().unwrap();

        // Applied → Validated
        session.validate().unwrap();

        // Validated → Completed
        let result = session.complete().unwrap();

        assert_eq!(result.content, "before\nleft\nafter");
    }

    // Invalid transitions

    #[test]
    fn cannot_apply_before_all_resolved() {
        let mut session = session_with_conflict();
        assert_eq!(session.state(), MergeState::Parsed);

        let result = session.apply();
        assert!(matches!(result, Err(ApplyError::NotFullyResolved)));
    }

    #[test]
    fn cannot_apply_with_partial_resolution() {
        let mut session = session_with_multiple_conflicts();

        // Only resolve first hunk
        let hunk_id = session.hunks()[0].id;
        let resolution = Resolution::accept_left(&session.hunks()[0]);
        session.set_resolution(hunk_id, resolution).unwrap();
        assert_eq!(session.state(), MergeState::Active);

        let result = session.apply();
        assert!(matches!(result, Err(ApplyError::NotFullyResolved)));
    }

    #[test]
    fn cannot_complete_without_validation() {
        let mut session = session_with_conflict();
        let hunk_id = session.hunks()[0].id;
        let resolution = Resolution::accept_left(&session.hunks()[0]);

        session.set_resolution(hunk_id, resolution).unwrap();
        let _ = session.apply().unwrap();
        // Skip validate()

        let result = session.complete();
        assert!(matches!(result, Err(CompletionError::LifecycleError(_))));
    }

    #[test]
    fn cannot_validate_before_apply() {
        let mut session = session_with_conflict();
        let hunk_id = session.hunks()[0].id;
        let resolution = Resolution::accept_left(&session.hunks()[0]);

        session.set_resolution(hunk_id, resolution).unwrap();
        // Skip apply()

        let result = session.validate();
        assert!(matches!(result, Err(ValidationError::UnresolvedHunks(_))));
    }

    #[test]
    fn cannot_set_resolution_after_applied() {
        let mut session = session_with_conflict();
        let hunk_id = session.hunks()[0].id;
        let resolution = Resolution::accept_left(&session.hunks()[0]);

        session.set_resolution(hunk_id, resolution.clone()).unwrap();
        let _ = session.apply().unwrap();

        let result = session.set_resolution(hunk_id, resolution);
        assert!(matches!(result, Err(ResolutionError::InvalidResolution(_))));
    }

    #[test]
    fn cannot_clear_resolution_after_applied() {
        let mut session = session_with_conflict();
        let hunk_id = session.hunks()[0].id;
        let resolution = Resolution::accept_left(&session.hunks()[0]);

        session.set_resolution(hunk_id, resolution).unwrap();
        let _ = session.apply().unwrap();

        let result = session.clear_resolution(hunk_id);
        assert!(matches!(result, Err(ResolutionError::InvalidResolution(_))));
    }

    #[test]
    fn set_resolution_hunk_not_found() {
        let mut session = session_with_conflict();
        let resolution = Resolution::manual("test".to_string());

        let result = session.set_resolution(HunkId(999), resolution);
        assert!(matches!(result, Err(ResolutionError::HunkNotFound(_))));
    }

    #[test]
    fn clear_resolution_hunk_not_found() {
        let mut session = session_with_conflict();

        let result = session.clear_resolution(HunkId(999));
        assert!(matches!(result, Err(ResolutionError::HunkNotFound(_))));
    }

    #[test]
    fn validate_fails_with_conflict_markers_in_resolution() {
        let mut session = session_with_conflict();
        let hunk_id = session.hunks()[0].id;

        // Create a resolution that contains conflict markers
        let bad_resolution =
            Resolution::manual("<<<<<<< HEAD\nfoo\n=======\nbar\n>>>>>>>".to_string());
        session.set_resolution(hunk_id, bad_resolution).unwrap();
        let _ = session.apply().unwrap();

        let result = session.validate();
        assert!(matches!(result, Err(ValidationError::MarkersRemain(_))));
    }

    // Determinism tests

    #[test]
    fn same_inputs_same_output() {
        let content = r"before
<<<<<<< HEAD
left
=======
right
>>>>>>> feature
after";

        let run = || {
            let mut session =
                MergeSession::from_conflicted(content, PathBuf::from("test.rs")).unwrap();
            let hunk_id = session.hunks()[0].id;
            let resolution = Resolution::accept_left(&session.hunks()[0]);
            session.set_resolution(hunk_id, resolution).unwrap();
            let _ = session.apply().unwrap();
            session.validate().unwrap();
            session.complete().unwrap()
        };

        let result1 = run();
        let result2 = run();

        assert_eq!(result1.content, result2.content);
        assert_eq!(result1.summary, result2.summary);
    }

    #[test]
    fn override_resolution_works() {
        let mut session = session_with_conflict();
        let hunk_id = session.hunks()[0].id;

        // First set to accept_left
        let left_resolution = Resolution::accept_left(&session.hunks()[0]);
        session.set_resolution(hunk_id, left_resolution).unwrap();

        // Override with accept_right
        let right_resolution = Resolution::accept_right(&session.hunks()[0]);
        session.set_resolution(hunk_id, right_resolution).unwrap();

        let _ = session.apply().unwrap();
        session.validate().unwrap();
        let result = session.complete().unwrap();

        assert!(result.content.contains("right"));
        assert!(!result.content.contains("left"));
    }

    #[test]
    fn can_transition_valid_transitions() {
        // Test all valid transitions
        assert!(MergeSession::can_transition(
            MergeState::Uninitialized,
            MergeState::Parsed
        ));
        assert!(MergeSession::can_transition(
            MergeState::Parsed,
            MergeState::Active
        ));
        assert!(MergeSession::can_transition(
            MergeState::Active,
            MergeState::FullyResolved
        ));
        assert!(MergeSession::can_transition(
            MergeState::FullyResolved,
            MergeState::Active
        ));
        assert!(MergeSession::can_transition(
            MergeState::FullyResolved,
            MergeState::Applied
        ));
        assert!(MergeSession::can_transition(
            MergeState::Applied,
            MergeState::Validated
        ));
        assert!(MergeSession::can_transition(
            MergeState::Validated,
            MergeState::Completed
        ));
    }

    #[test]
    fn can_transition_invalid_transitions() {
        // Test some invalid transitions
        assert!(!MergeSession::can_transition(
            MergeState::Parsed,
            MergeState::Applied
        ));
        assert!(!MergeSession::can_transition(
            MergeState::Active,
            MergeState::Validated
        ));
        assert!(!MergeSession::can_transition(
            MergeState::Applied,
            MergeState::Completed
        ));
        assert!(!MergeSession::can_transition(
            MergeState::Parsed,
            MergeState::Parsed
        ));
    }
}
