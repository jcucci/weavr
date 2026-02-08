//! Request and response types for AI providers.

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
        Self {
            left: hunk.left.text.clone(),
            right: hunk.right.text.clone(),
            base: hunk.base.as_ref().map(|b| b.text.clone()),
            context: ConflictContext {
                before: hunk.context.before.clone(),
                after: hunk.context.after.clone(),
                file_path: file_path.map(String::from),
                language: file_path.and_then(detect_language),
            },
        }
    }
}

/// Detects programming language from file extension.
fn detect_language(path: &str) -> Option<String> {
    let ext = path.rsplit('.').next()?;
    match ext.to_lowercase().as_str() {
        "rs" => Some("rust".into()),
        "cs" => Some("csharp".into()),
        "ts" | "tsx" => Some("typescript".into()),
        "js" | "jsx" => Some("javascript".into()),
        "go" => Some("go".into()),
        "py" => Some("python".into()),
        "rb" => Some("ruby".into()),
        "java" => Some("java".into()),
        "kt" | "kts" => Some("kotlin".into()),
        "swift" => Some("swift".into()),
        "c" | "h" => Some("c".into()),
        "cpp" | "cc" | "cxx" | "hpp" => Some("cpp".into()),
        "json" => Some("json".into()),
        "yaml" | "yml" => Some("yaml".into()),
        "toml" => Some("toml".into()),
        "md" => Some("markdown".into()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_rust() {
        assert_eq!(detect_language("src/main.rs"), Some("rust".into()));
    }

    #[test]
    fn detect_typescript() {
        assert_eq!(detect_language("app.tsx"), Some("typescript".into()));
        assert_eq!(detect_language("index.ts"), Some("typescript".into()));
    }

    #[test]
    fn detect_unknown() {
        assert_eq!(detect_language("file.xyz"), None);
    }

    #[test]
    fn detect_case_insensitive() {
        assert_eq!(detect_language("Main.RS"), Some("rust".into()));
    }
}
