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

impl From<GitError> for weavr_vcs::VcsError {
    fn from(err: GitError) -> Self {
        match err {
            GitError::NotGitRepo => Self::NotInRepo,
            GitError::DiscoveryFailed(msg) => Self::DiscoveryFailed(msg),
            GitError::CommandFailed(io) => Self::CommandFailed(io),
            GitError::CommandError { stderr } => Self::OperationError(stderr),
            GitError::ParseError(msg) => Self::ParseError(msg),
            GitError::FileError { path, source } => Self::FileError { path, source },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_git_repo_converts_to_not_in_repo() {
        let vcs_err: weavr_vcs::VcsError = GitError::NotGitRepo.into();
        assert!(matches!(vcs_err, weavr_vcs::VcsError::NotInRepo));
    }

    #[test]
    fn discovery_failed_converts() {
        let vcs_err: weavr_vcs::VcsError = GitError::DiscoveryFailed("bad path".into()).into();
        assert!(matches!(vcs_err, weavr_vcs::VcsError::DiscoveryFailed(msg) if msg == "bad path"));
    }

    #[test]
    fn command_error_converts_to_operation_error() {
        let vcs_err: weavr_vcs::VcsError = GitError::CommandError {
            stderr: "fatal".into(),
        }
        .into();
        assert!(matches!(vcs_err, weavr_vcs::VcsError::OperationError(msg) if msg == "fatal"));
    }

    #[test]
    fn parse_error_converts() {
        let vcs_err: weavr_vcs::VcsError = GitError::ParseError("bad output".into()).into();
        assert!(matches!(vcs_err, weavr_vcs::VcsError::ParseError(msg) if msg == "bad output"));
    }
}
