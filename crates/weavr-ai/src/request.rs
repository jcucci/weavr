//! Request and response types for AI providers.

use std::path::Path;

use serde::{Deserialize, Serialize};
use weavr_core::ConflictHunk;

/// Context provided to the AI provider about the conflict.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictContext {
    /// Lines before the conflict region.
    pub before: Vec<String>,
    /// Lines after the conflict region.
    pub after: Vec<String>,
    /// Path to the file (for language detection).
    pub file_path: Option<String>,
    /// Detected or specified language.
    pub language: Option<String>,
}

/// Request payload sent to AI providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiRequest {
    /// Left (ours) content.
    pub left: String,
    /// Right (theirs) content.
    pub right: String,
    /// Base content if available (3-way merge).
    pub base: Option<String>,
    /// Surrounding context.
    pub context: ConflictContext,
}

/// Response from AI provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiResponse {
    /// The suggested merged content.
    pub suggestion: String,
    /// Confidence score (0-100 percentage).
    pub confidence: u8,
    /// Explanation of the merge reasoning.
    pub explanation: Option<String>,
}

impl AiRequest {
    /// Creates a request from a `ConflictHunk`.
    #[must_use]
    pub fn from_hunk(hunk: &ConflictHunk, file_path: Option<&str>) -> Self {
        let language = file_path
            .map(|p| weavr_core::detect_language(Path::new(p)))
            .filter(|lang| *lang != weavr_core::Language::Unknown)
            .map(|lang| lang.display_name().to_string());

        Self {
            left: hunk.left.text.clone(),
            right: hunk.right.text.clone(),
            base: hunk.base.as_ref().map(|b| b.text.clone()),
            context: ConflictContext {
                before: hunk.context.before.clone(),
                after: hunk.context.after.clone(),
                file_path: file_path.map(String::from),
                language,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use weavr_core::{HunkContent, HunkContext, HunkId, HunkState};

    fn test_hunk() -> ConflictHunk {
        ConflictHunk {
            id: HunkId(1),
            left: HunkContent {
                text: "left".to_string(),
            },
            right: HunkContent {
                text: "right".to_string(),
            },
            base: None,
            context: HunkContext::default(),
            state: HunkState::default(),
        }
    }

    #[test]
    fn from_hunk_detects_rust() {
        let req = AiRequest::from_hunk(&test_hunk(), Some("src/main.rs"));
        assert_eq!(req.context.language.as_deref(), Some("rust"));
    }

    #[test]
    fn from_hunk_detects_typescript() {
        let req = AiRequest::from_hunk(&test_hunk(), Some("app.tsx"));
        assert_eq!(req.context.language.as_deref(), Some("typescript"));
    }

    #[test]
    fn from_hunk_unknown_extension_returns_none() {
        let req = AiRequest::from_hunk(&test_hunk(), Some("file.xyz"));
        assert!(req.context.language.is_none());
    }

    #[test]
    fn from_hunk_no_path_returns_none() {
        let req = AiRequest::from_hunk(&test_hunk(), None);
        assert!(req.context.language.is_none());
    }

    #[test]
    fn from_hunk_case_insensitive() {
        let req = AiRequest::from_hunk(&test_hunk(), Some("Main.RS"));
        assert_eq!(req.context.language.as_deref(), Some("rust"));
    }
}
