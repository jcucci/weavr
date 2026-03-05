//! VCS backend trait definition.

use std::path::Path;

use crate::error::VcsError;
use crate::types::{ConflictedFile, VcsOperation};

/// A backend-agnostic interface to a version control system.
///
/// Implementations provide concrete VCS operations (Git, Jujutsu, etc.).
/// Discovery is not part of the trait — each backend provides its own
/// constructor, and consumers work with `Box<dyn VcsBackend>`.
///
/// All operations are synchronous; backends that shell out to subprocesses
/// can use blocking calls directly.
pub trait VcsBackend: Send + Sync {
    /// Returns the name of this VCS backend (e.g., "git", "jj").
    fn name(&self) -> &str;

    /// Returns the root directory of the repository's working tree.
    fn root(&self) -> &Path;

    /// Returns all files with detected conflicts.
    ///
    /// # Errors
    ///
    /// Returns [`VcsError`] if the backend cannot determine conflict status.
    fn conflicted_files(&self) -> Result<Vec<ConflictedFile>, VcsError>;

    /// Stages a resolved file for the next commit.
    ///
    /// The `path` must be relative to the repository root, matching the paths
    /// returned by [`conflicted_files()`](Self::conflicted_files).
    ///
    /// # Errors
    ///
    /// Returns [`VcsError`] if staging fails.
    fn stage_file(&self, path: &Path) -> Result<(), VcsError>;

    /// Returns the current VCS operation in progress, if any.
    ///
    /// # Errors
    ///
    /// Returns [`VcsError`] if the backend cannot determine the current operation.
    fn current_operation(&self) -> Result<VcsOperation, VcsError>;
}
