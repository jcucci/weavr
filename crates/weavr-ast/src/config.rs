//! Configuration for AST merge strategies.

use weavr_core::Language;

/// Configuration for the AST merge strategy.
#[derive(Debug, Clone)]
pub struct AstConfig {
    /// Whether AST merging is enabled globally.
    pub enabled: bool,
    /// Minimum confidence threshold (0.0–1.0) for accepting an AST merge result.
    pub min_confidence: f32,
    /// Languages to exclude from AST merging, even if a merger is registered.
    pub excluded_languages: Vec<Language>,
}

impl Default for AstConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_confidence: 0.5,
            excluded_languages: Vec::new(),
        }
    }
}
