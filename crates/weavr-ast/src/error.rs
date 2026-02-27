//! Error types for AST merge operations.

use thiserror::Error;
use weavr_core::Language;

/// Errors that can occur during AST-based merging.
#[derive(Debug, Error)]
pub enum AstError {
    /// No merger is registered for the given language.
    #[error("no AST merger available for {0}")]
    UnsupportedLanguage(Language),

    /// The merger encountered a parse error in the source.
    #[error("failed to parse source: {0}")]
    ParseError(String),

    /// An internal error in the merge logic.
    #[error("internal AST merge error: {0}")]
    Internal(String),
}
