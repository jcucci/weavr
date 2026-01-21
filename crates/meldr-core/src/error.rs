//! Error types for meldr-core.

use thiserror::Error;

use crate::HunkId;

/// Error parsing conflict markers.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum ParseError {
    /// Invalid conflict markers in content.
    #[error("invalid conflict markers: {0}")]
    InvalidMarkers(String),
    /// Malformed content.
    #[error("malformed content: {0}")]
    MalformedContent(String),
}

/// Error applying a resolution.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum ResolutionError {
    /// Hunk not found.
    #[error("hunk not found: {0:?}")]
    HunkNotFound(HunkId),
    /// Invalid resolution.
    #[error("invalid resolution: {0}")]
    InvalidResolution(String),
}

/// Error validating merge output.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum ValidationError {
    /// Unresolved hunks remain.
    #[error("unresolved hunks: {0:?}")]
    UnresolvedHunks(Vec<HunkId>),
    /// Conflict markers remain in output.
    #[error("conflict markers remain: {0} markers")]
    MarkersRemain(usize),
    /// Syntax error in output.
    #[error("syntax error: {0}")]
    SyntaxError(String),
}

/// Error applying resolutions to generate output.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum ApplyError {
    /// Not all hunks are resolved.
    #[error("not all hunks are resolved")]
    NotFullyResolved,
    /// Internal error during application.
    #[error("internal error: {0}")]
    InternalError(String),
}

/// Error completing a merge session.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum CompletionError {
    /// Validation failed.
    #[error("validation failed: {0}")]
    ValidationFailed(#[from] ValidationError),
    /// Apply failed.
    #[error("apply failed: {0}")]
    ApplyFailed(#[from] ApplyError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_error_display() {
        let err = ParseError::InvalidMarkers(String::from("missing end marker"));
        assert_eq!(
            err.to_string(),
            "invalid conflict markers: missing end marker"
        );
    }

    #[test]
    fn resolution_error_display() {
        let err = ResolutionError::HunkNotFound(HunkId(42));
        assert_eq!(err.to_string(), "hunk not found: HunkId(42)");
    }

    #[test]
    fn validation_error_display() {
        let err = ValidationError::MarkersRemain(3);
        assert_eq!(err.to_string(), "conflict markers remain: 3 markers");
    }

    #[test]
    fn apply_error_display() {
        let err = ApplyError::NotFullyResolved;
        assert_eq!(err.to_string(), "not all hunks are resolved");
    }

    #[test]
    fn completion_error_from_validation() {
        let validation_err = ValidationError::MarkersRemain(1);
        let completion_err: CompletionError = validation_err.into();
        assert!(matches!(
            completion_err,
            CompletionError::ValidationFailed(_)
        ));
    }

    #[test]
    fn completion_error_from_apply() {
        let apply_err = ApplyError::NotFullyResolved;
        let completion_err: CompletionError = apply_err.into();
        assert!(matches!(completion_err, CompletionError::ApplyFailed(_)));
    }
}
