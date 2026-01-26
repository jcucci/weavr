//! Git conflict file discovery.

use std::path::{Path, PathBuf};

use weavr_git::GitRepo;

use crate::error::CliError;

/// Discovers files with Git merge conflicts in the current repository.
pub fn discover_conflicted_files() -> Result<Vec<PathBuf>, CliError> {
    let repo = GitRepo::discover()?;
    let files = repo.conflicted_files()?;
    Ok(files)
}

/// Checks if a file contains conflict markers.
pub fn has_conflict_markers(path: &Path) -> Result<bool, CliError> {
    let content = std::fs::read_to_string(path)?;
    Ok(content.contains("<<<<<<<") && content.contains("=======") && content.contains(">>>>>>>"))
}

/// Filters provided paths to only those with conflicts, or discovers all.
pub fn resolve_files(provided: Vec<PathBuf>) -> Result<Vec<PathBuf>, CliError> {
    if provided.is_empty() {
        let files = discover_conflicted_files()?;
        if files.is_empty() {
            return Err(CliError::NoConflictedFiles);
        }
        Ok(files)
    } else {
        let mut valid = Vec::new();
        for path in provided {
            if !path.exists() {
                return Err(CliError::FileNotFound(path));
            }
            if has_conflict_markers(&path)? {
                valid.push(path);
            }
        }
        if valid.is_empty() {
            Err(CliError::NoConflictedFiles)
        } else {
            Ok(valid)
        }
    }
}

/// Lists conflicted files to stdout.
pub fn list_conflicted_files() -> Result<(), CliError> {
    let files = discover_conflicted_files()?;

    if files.is_empty() {
        println!("No conflicted files found");
    } else {
        for file in files {
            println!("{}", file.display());
        }
    }

    Ok(())
}
