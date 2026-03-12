//! Error types for weavr-jj.

use thiserror::Error;

/// Jujutsu operation errors.
#[derive(Debug, Error)]
pub enum JjError {
    /// Not inside a Jujutsu repository.
    #[error("not in a jj repository")]
    NotJjRepo,

    /// Failed to discover repository root.
    #[error("failed to discover repository root: {0}")]
    DiscoveryFailed(String),

    /// jj command execution failed.
    #[error("jj command failed: {0}")]
    CommandFailed(#[source] std::io::Error),

    /// jj command returned non-zero exit status.
    #[error("jj command returned error: {stderr}")]
    CommandError {
        /// The stderr output from the jj command.
        stderr: String,
    },

    /// Failed to parse jj output.
    #[error("failed to parse jj output: {0}")]
    ParseError(String),
}

impl From<JjError> for weavr_vcs::VcsError {
    fn from(err: JjError) -> Self {
        match err {
            JjError::NotJjRepo => Self::NotInRepo,
            JjError::DiscoveryFailed(msg) => Self::DiscoveryFailed(msg),
            JjError::CommandFailed(io) => Self::CommandFailed(io),
            JjError::CommandError { stderr } => Self::OperationError(stderr),
            JjError::ParseError(msg) => Self::ParseError(msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_jj_repo_converts_to_not_in_repo() {
        let vcs_err: weavr_vcs::VcsError = JjError::NotJjRepo.into();
        assert!(matches!(vcs_err, weavr_vcs::VcsError::NotInRepo));
    }

    #[test]
    fn discovery_failed_converts() {
        let vcs_err: weavr_vcs::VcsError = JjError::DiscoveryFailed("bad path".into()).into();
        assert!(matches!(vcs_err, weavr_vcs::VcsError::DiscoveryFailed(msg) if msg == "bad path"));
    }

    #[test]
    fn command_error_converts_to_operation_error() {
        let vcs_err: weavr_vcs::VcsError = JjError::CommandError {
            stderr: "fatal".into(),
        }
        .into();
        assert!(matches!(vcs_err, weavr_vcs::VcsError::OperationError(msg) if msg == "fatal"));
    }

    #[test]
    fn parse_error_converts() {
        let vcs_err: weavr_vcs::VcsError = JjError::ParseError("bad output".into()).into();
        assert!(matches!(vcs_err, weavr_vcs::VcsError::ParseError(msg) if msg == "bad output"));
    }
}
