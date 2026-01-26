//! Error types for weavr-git.

use std::path::PathBuf;

use thiserror::Error;

/// Git operation errors.
#[derive(Debug, Error)]
pub enum GitError {
    /// Not inside a Git repository.
    #[error("not in a git repository")]
    NotGitRepo,

    /// Failed to discover repository root.
    #[error("failed to discover repository root: {0}")]
    DiscoveryFailed(String),

    /// Git command execution failed.
    #[error("git command failed: {0}")]
    CommandFailed(#[source] std::io::Error),

    /// Git command returned non-zero exit status.
    #[error("git command returned error: {stderr}")]
    CommandError {
        /// The stderr output from the git command.
        stderr: String,
    },

    /// Failed to parse Git output.
    #[error("failed to parse git output: {0}")]
    ParseError(String),

    /// File operation failed.
    #[error("file operation failed on {path}: {source}")]
    FileError {
        /// The path that caused the error.
        path: PathBuf,
        /// The underlying IO error.
        #[source]
        source: std::io::Error,
    },
}
