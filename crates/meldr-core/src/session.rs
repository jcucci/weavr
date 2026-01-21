//! Merge session management.
//!
//! All types in this module are **stable** and covered by semantic versioning.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{
    parse_conflict_markers, ConflictHunk, FileVersion, HunkId, HunkState, MergeInput, ParseError,
    ParsedConflict, Resolution, Segment,
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
        let content = r#"before
<<<<<<< HEAD
left content
=======
right content
>>>>>>> feature
after"#;

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

        let session =
            MergeSession::from_conflicted(content, PathBuf::from("clean.rs")).expect("should parse");
        assert!(session.hunks().is_empty());
        assert_eq!(session.state(), MergeState::Validated);
    }

    #[test]
    fn from_conflicted_preserves_segments() {
        let content = r#"before
<<<<<<< HEAD
left
=======
right
>>>>>>> feature
after"#;

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
        let content = r#"<<<<<<< HEAD
unclosed conflict"#;

        let result = MergeSession::from_conflicted(content, PathBuf::from("bad.rs"));
        assert!(result.is_err());
    }
}
