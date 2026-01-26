//! CLI-specific error types.

use std::path::PathBuf;

use thiserror::Error;

/// Exit codes for weavr CLI.
pub mod exit_codes {
    /// All conflicts resolved successfully.
    pub const SUCCESS: i32 = 0;
    /// Unresolved conflicts remain.
    pub const UNRESOLVED: i32 = 1;
    /// Error occurred (parse failure, IO error, etc.).
    pub const ERROR: i32 = 2;
}

/// CLI-specific errors.
#[derive(Debug, Error)]
pub enum CliError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(#[from] weavr_core::ParseError),

    #[error("No conflicted files found")]
    NoConflictedFiles,

    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    #[error("Not in a git repository")]
    NotGitRepo,

    #[error("Git command failed: {0}")]
    GitCommandFailed(std::io::Error),

    #[error("Resolution error: {0}")]
    Resolution(#[from] weavr_core::ResolutionError),

    #[error("Apply error: {0}")]
    Apply(#[from] weavr_core::ApplyError),

    #[error("Validation error: {0}")]
    Validation(#[from] weavr_core::ValidationError),

    #[error("Completion error: {0}")]
    Completion(#[from] weavr_core::CompletionError),

    #[error("Ambiguous hunks remain: {0} hunks could not be auto-resolved")]
    #[allow(dead_code)] // Reserved for --fail-on-ambiguous implementation
    AmbiguousHunks(usize),
}

impl CliError {
    /// Returns the appropriate exit code for this error.
    pub fn exit_code(&self) -> i32 {
        match self {
            CliError::NoConflictedFiles => exit_codes::SUCCESS,
            CliError::AmbiguousHunks(_) => exit_codes::UNRESOLVED,
            _ => exit_codes::ERROR,
        }
    }
}
