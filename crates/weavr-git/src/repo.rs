//! Git repository abstraction.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::GitError;
use crate::porcelain::{parse_porcelain_v1, ConflictEntry};
use crate::state::GitOperation;

/// A handle to a Git repository.
#[derive(Debug, Clone)]
pub struct GitRepo {
    /// The root directory of the repository (where .git is).
    root: PathBuf,
}

impl GitRepo {
    /// Discovers the Git repository from the current directory.
    ///
    /// Walks up the directory tree looking for a `.git` directory.
    ///
    /// # Errors
    ///
    /// Returns `GitError::NotGitRepo` if not inside a Git repository.
    /// Returns `GitError::DiscoveryFailed` if the current directory cannot be determined.
    pub fn discover() -> Result<Self, GitError> {
        Self::discover_from(
            std::env::current_dir()
                .map_err(|e| GitError::DiscoveryFailed(e.to_string()))?,
        )
    }

    /// Discovers the Git repository starting from the given path.
    ///
    /// Uses `git rev-parse --show-toplevel` to find the repository root.
    ///
    /// # Errors
    ///
    /// Returns `GitError::NotGitRepo` if the path is not inside a Git repository.
    /// Returns `GitError::CommandFailed` if the git command fails to execute.
    pub fn discover_from(start: impl AsRef<Path>) -> Result<Self, GitError> {
        let output = Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .current_dir(start.as_ref())
            .output()
            .map_err(GitError::CommandFailed)?;

        if !output.status.success() {
            return Err(GitError::NotGitRepo);
        }

        let root = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());

        Ok(Self { root })
    }

    /// Returns the root directory of the repository.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns a list of files with merge conflicts.
    ///
    /// Uses `git status --porcelain=v1` to detect unmerged paths.
    ///
    /// # Errors
    ///
    /// Returns `GitError::CommandFailed` if the git command fails to execute.
    /// Returns `GitError::CommandError` if git returns a non-zero exit status.
    pub fn conflicted_files(&self) -> Result<Vec<PathBuf>, GitError> {
        let entries = self.conflicted_entries()?;
        Ok(entries.into_iter().map(|e| e.path).collect())
    }

    /// Returns detailed conflict information for all conflicted files.
    ///
    /// # Errors
    ///
    /// Returns `GitError::CommandFailed` if the git command fails to execute.
    /// Returns `GitError::CommandError` if git returns a non-zero exit status.
    pub fn conflicted_entries(&self) -> Result<Vec<ConflictEntry>, GitError> {
        let output = self.run_git(&["status", "--porcelain=v1"])?;
        Ok(parse_porcelain_v1(&output))
    }

    /// Stages a resolved file.
    ///
    /// # Errors
    ///
    /// Returns `GitError::CommandFailed` if the git command fails to execute.
    /// Returns `GitError::CommandError` if git returns a non-zero exit status.
    pub fn stage_file(&self, path: &Path) -> Result<(), GitError> {
        self.run_git(&["add", &path.to_string_lossy()])?;
        Ok(())
    }

    /// Returns true if a merge is in progress.
    #[must_use]
    pub fn is_in_merge(&self) -> bool {
        self.root.join(".git/MERGE_HEAD").exists()
    }

    /// Returns true if a rebase is in progress.
    #[must_use]
    pub fn is_in_rebase(&self) -> bool {
        self.root.join(".git/rebase-merge").exists()
            || self.root.join(".git/rebase-apply").exists()
    }

    /// Returns true if a cherry-pick is in progress.
    #[must_use]
    pub fn is_in_cherry_pick(&self) -> bool {
        self.root.join(".git/CHERRY_PICK_HEAD").exists()
    }

    /// Returns true if a revert is in progress.
    #[must_use]
    pub fn is_in_revert(&self) -> bool {
        self.root.join(".git/REVERT_HEAD").exists()
    }

    /// Returns the current Git operation in progress, if any.
    #[must_use]
    pub fn current_operation(&self) -> GitOperation {
        if self.is_in_merge() {
            GitOperation::Merge
        } else if self.is_in_rebase() {
            GitOperation::Rebase
        } else if self.is_in_cherry_pick() {
            GitOperation::CherryPick
        } else if self.is_in_revert() {
            GitOperation::Revert
        } else {
            GitOperation::None
        }
    }

    /// Runs a git command and returns stdout as a string.
    fn run_git(&self, args: &[&str]) -> Result<String, GitError> {
        let output = Command::new("git")
            .args(args)
            .current_dir(&self.root)
            .output()
            .map_err(GitError::CommandFailed)?;

        if !output.status.success() {
            return Err(GitError::CommandError {
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }

        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}
