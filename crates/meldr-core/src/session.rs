//! Merge session management.
//!
//! All types in this module are **stable** and covered by semantic versioning.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{ConflictHunk, HunkId, HunkState, MergeInput, ParseError, Resolution};

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
    /// Current session state.
    state: MergeState,
    /// Applied resolutions.
    resolutions: HashMap<HunkId, Resolution>,
}

impl MergeSession {
    /// Creates a new merge session from input.
    ///
    /// # Errors
    ///
    /// Returns `ParseError` if conflict markers cannot be parsed.
    pub fn new(input: MergeInput) -> Result<Self, ParseError> {
        // Placeholder implementation - actual parsing will be implemented later
        Ok(Self {
            input,
            hunks: Vec::new(),
            state: MergeState::Parsed,
            resolutions: HashMap::new(),
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
}
