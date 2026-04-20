//! Git repository abstraction.

use std::path::{Path, PathBuf};
use std::process::Command;

use weavr_vcs::{ConflictedFile, VcsBackend, VcsError, VcsOperation};

use crate::error::GitError;
use crate::porcelain::{parse_modified_v1, parse_porcelain_v1, ConflictEntry};
use crate::state::GitOperation;

/// A handle to a Git repository.
#[derive(Debug, Clone)]
pub struct GitRepo {
    /// The root directory of the repository's working tree.
    root: PathBuf,
    /// The git directory (usually .git, but different in worktrees).
    git_dir: PathBuf,
}

impl GitRepo {
    /// Discovers the Git repository from the current directory.
    ///
    /// Uses `git rev-parse --show-toplevel` (via [`Self::discover_from`]) to find the repository root.
    ///
    /// # Errors
    ///
    /// Returns `GitError::NotGitRepo` if not inside a Git repository.
    /// Returns `GitError::DiscoveryFailed` if the current directory cannot be determined.
    pub fn discover() -> Result<Self, GitError> {
        Self::discover_from(
            std::env::current_dir().map_err(|e| GitError::DiscoveryFailed(e.to_string()))?,
        )
    }

    /// Discovers the Git repository starting from the given path.
    ///
    /// Uses `git rev-parse --show-toplevel` to find the repository root
    /// and `git rev-parse --git-dir` to find the git directory (for worktree support).
    ///
    /// # Errors
    ///
    /// Returns `GitError::NotGitRepo` if the path is not inside a Git repository.
    /// Returns `GitError::CommandFailed` if the git command fails to execute.
    pub fn discover_from(start: impl AsRef<Path>) -> Result<Self, GitError> {
        let start_path = start.as_ref();

        // Get the working tree root
        let toplevel_output = Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .current_dir(start_path)
            .output()
            .map_err(GitError::CommandFailed)?;

        if !toplevel_output.status.success() {
            return Err(GitError::NotGitRepo);
        }

        let root = PathBuf::from(String::from_utf8_lossy(&toplevel_output.stdout).trim());

        // Get the git directory (handles worktrees correctly)
        let gitdir_output = Command::new("git")
            .args(["rev-parse", "--git-dir"])
            .current_dir(start_path)
            .output()
            .map_err(GitError::CommandFailed)?;

        let git_dir = if gitdir_output.status.success() {
            let git_dir_str = String::from_utf8_lossy(&gitdir_output.stdout)
                .trim()
                .to_string();
            let git_dir_path = PathBuf::from(&git_dir_str);
            // Make absolute if relative
            if git_dir_path.is_absolute() {
                git_dir_path
            } else {
                start_path.join(git_dir_path)
            }
        } else {
            // Fallback to .git in root (shouldn't happen if toplevel succeeded)
            root.join(".git")
        };

        Ok(Self { root, git_dir })
    }

    /// Returns the root directory of the repository's working tree.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns the git directory path.
    ///
    /// This is usually `.git` but may be different in worktrees.
    #[must_use]
    pub fn git_dir(&self) -> &Path {
        &self.git_dir
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

    /// Returns paths of all modified (dirty) files in the working tree.
    ///
    /// Uses `git status --porcelain=v1` to detect modified, added, and untracked files
    /// that are not in an unmerged state.
    ///
    /// # Errors
    ///
    /// Returns `GitError::CommandFailed` if the git command fails to execute.
    /// Returns `GitError::CommandError` if git returns a non-zero exit status.
    pub fn modified_files(&self) -> Result<Vec<PathBuf>, GitError> {
        let output = self.run_git(&["status", "--porcelain=v1"])?;
        Ok(parse_modified_v1(&output))
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
        self.git_dir.join("MERGE_HEAD").exists()
    }

    /// Returns true if a rebase is in progress.
    #[must_use]
    pub fn is_in_rebase(&self) -> bool {
        self.git_dir.join("rebase-merge").exists() || self.git_dir.join("rebase-apply").exists()
    }

    /// Returns true if a cherry-pick is in progress.
    #[must_use]
    pub fn is_in_cherry_pick(&self) -> bool {
        self.git_dir.join("CHERRY_PICK_HEAD").exists()
    }

    /// Returns true if a revert is in progress.
    #[must_use]
    pub fn is_in_revert(&self) -> bool {
        self.git_dir.join("REVERT_HEAD").exists()
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

impl VcsBackend for GitRepo {
    #[allow(clippy::unnecessary_literal_bound)]
    fn name(&self) -> &str {
        "git"
    }

    fn root(&self) -> &Path {
        &self.root
    }

    fn conflicted_files(&self) -> Result<Vec<ConflictedFile>, VcsError> {
        let entries = self.conflicted_entries().map_err(VcsError::from)?;
        Ok(entries
            .into_iter()
            .map(|e| ConflictedFile {
                path: e.path,
                kind: e.conflict_type.into(),
            })
            .collect())
    }

    fn stage_file(&self, path: &Path) -> Result<(), VcsError> {
        GitRepo::stage_file(self, path).map_err(VcsError::from)
    }

    fn current_operation(&self) -> Result<VcsOperation, VcsError> {
        Ok(GitRepo::current_operation(self).into())
    }

    fn modified_files(&self) -> Result<Vec<PathBuf>, VcsError> {
        GitRepo::modified_files(self).map_err(VcsError::from)
    }
}
