//! Jujutsu repository abstraction.

use std::path::{Path, PathBuf};
use std::process::Command;

use weavr_vcs::{ConflictKind, ConflictedFile, VcsBackend, VcsError, VcsOperation};

use crate::error::JjError;
use crate::status::parse_jj_status;

/// A handle to a Jujutsu repository.
#[derive(Debug, Clone)]
pub struct JjRepo {
    /// The root directory of the repository's working tree.
    root: PathBuf,
}

impl JjRepo {
    /// Discovers the Jujutsu repository from the current directory.
    ///
    /// Uses `jj root` to find the repository root.
    ///
    /// # Errors
    ///
    /// Returns `JjError::NotJjRepo` if not inside a Jujutsu repository.
    /// Returns `JjError::DiscoveryFailed` if the current directory cannot be determined.
    pub fn discover() -> Result<Self, JjError> {
        Self::discover_from(
            std::env::current_dir().map_err(|e| JjError::DiscoveryFailed(e.to_string()))?,
        )
    }

    /// Discovers the Jujutsu repository starting from the given path.
    ///
    /// Uses `jj root` with the given path as the working directory to find the repository root.
    ///
    /// # Errors
    ///
    /// Returns `JjError::NotJjRepo` if the path is not inside a Jujutsu repository.
    /// Returns `JjError::CommandFailed` if the jj command fails to execute.
    pub fn discover_from(start: impl AsRef<Path>) -> Result<Self, JjError> {
        let start_path = start.as_ref();

        let output = Command::new("jj")
            .arg("root")
            .current_dir(start_path)
            .output()
            .map_err(JjError::CommandFailed)?;

        if !output.status.success() {
            return Err(JjError::NotJjRepo);
        }

        let root = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());

        Ok(Self { root })
    }

    /// Returns the root directory of the repository's working tree.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns a list of files with conflicts.
    ///
    /// Uses `jj status` to detect conflicted files (lines prefixed with `C`).
    ///
    /// # Errors
    ///
    /// Returns `JjError::CommandFailed` if the jj command fails to execute.
    /// Returns `JjError::CommandError` if jj returns a non-zero exit status.
    pub fn conflicted_files(&self) -> Result<Vec<PathBuf>, JjError> {
        let output = self.run_jj(&["status"])?;
        Ok(parse_jj_status(&output))
    }

    /// Runs a jj command and returns stdout as a string.
    fn run_jj(&self, args: &[&str]) -> Result<String, JjError> {
        let output = Command::new("jj")
            .args(args)
            .current_dir(&self.root)
            .output()
            .map_err(JjError::CommandFailed)?;

        if !output.status.success() {
            return Err(JjError::CommandError {
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }

        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

impl VcsBackend for JjRepo {
    #[allow(clippy::unnecessary_literal_bound)]
    fn name(&self) -> &str {
        "jj"
    }

    fn root(&self) -> &Path {
        &self.root
    }

    fn conflicted_files(&self) -> Result<Vec<ConflictedFile>, VcsError> {
        let paths = JjRepo::conflicted_files(self).map_err(VcsError::from)?;
        Ok(paths
            .into_iter()
            .map(|path| ConflictedFile {
                path,
                kind: ConflictKind::Other,
            })
            .collect())
    }

    fn stage_file(&self, _path: &Path) -> Result<(), VcsError> {
        // jj auto-tracks the working copy — no explicit staging needed.
        Ok(())
    }

    fn current_operation(&self) -> Result<VcsOperation, VcsError> {
        let files = JjRepo::conflicted_files(self).map_err(VcsError::from)?;
        if files.is_empty() {
            Ok(VcsOperation::None)
        } else {
            Ok(VcsOperation::Other)
        }
    }
}
