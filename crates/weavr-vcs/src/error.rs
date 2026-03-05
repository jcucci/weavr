//! Error types for weavr-vcs.

use std::path::PathBuf;

use thiserror::Error;

/// VCS operation errors.
#[derive(Debug, Error)]
pub enum VcsError {
    /// Not inside a VCS repository.
    #[error("not in a repository")]
    NotInRepo,

    /// Failed to discover repository root.
    #[error("failed to discover repository: {0}")]
    DiscoveryFailed(String),

    /// VCS command execution failed.
    #[error("command failed: {0}")]
    CommandFailed(#[source] std::io::Error),

    /// A VCS operation returned an error.
    #[error("operation error: {0}")]
    OperationError(String),

    /// Failed to parse VCS output.
    #[error("failed to parse output: {0}")]
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

    /// The operation is not supported by this backend.
    #[error("unsupported: {0}")]
    Unsupported(String),
}
