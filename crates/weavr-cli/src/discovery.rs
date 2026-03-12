//! VCS conflict file discovery and backend auto-detection.

use std::path::{Path, PathBuf};

use weavr_vcs::VcsBackend;

use crate::cli::VcsChoice;
use crate::error::CliError;

/// Discovers the VCS backend based on the user's choice.
///
/// - `Auto`: tries jj first (for colocated repos), then falls back to git.
/// - `Git`: only tries git.
/// - `Jj`: only tries jj.
///
/// Returns `None` if no backend can be discovered for the given choice.
pub fn discover_backend(vcs_choice: VcsChoice) -> Option<Box<dyn VcsBackend>> {
    match vcs_choice {
        VcsChoice::Git => try_git(),
        VcsChoice::Jj => try_jj(),
        VcsChoice::Auto => try_jj().or_else(try_git),
    }
}

/// Attempts to discover a Git repository.
fn try_git() -> Option<Box<dyn VcsBackend>> {
    weavr_git::GitRepo::discover()
        .ok()
        .map(|repo| Box::new(repo) as Box<dyn VcsBackend>)
}

/// Attempts to discover a Jujutsu repository.
#[cfg(feature = "jj")]
fn try_jj() -> Option<Box<dyn VcsBackend>> {
    weavr_jj::JjRepo::discover()
        .ok()
        .map(|repo| Box::new(repo) as Box<dyn VcsBackend>)
}

/// Stub when jj feature is disabled.
#[cfg(not(feature = "jj"))]
fn try_jj() -> Option<Box<dyn VcsBackend>> {
    None
}

/// Discovers files with merge conflicts using the given VCS backend.
pub fn discover_conflicted_files(backend: &dyn VcsBackend) -> Result<Vec<PathBuf>, CliError> {
    let files = backend.conflicted_files()?;
    Ok(files.into_iter().map(|f| f.path).collect())
}

/// Checks if a file contains conflict markers.
pub fn has_conflict_markers(path: &Path) -> Result<bool, CliError> {
    let content = std::fs::read_to_string(path)?;
    Ok(content.contains("<<<<<<<") && content.contains("=======") && content.contains(">>>>>>>"))
}

/// Filters provided paths to only those with conflicts, or discovers all via `backend`.
pub fn resolve_files(
    provided: Vec<PathBuf>,
    backend: Option<&dyn VcsBackend>,
) -> Result<Vec<PathBuf>, CliError> {
    if provided.is_empty() {
        let backend = backend.ok_or(CliError::Vcs(weavr_vcs::VcsError::NotInRepo))?;
        let files = discover_conflicted_files(backend)?;
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
pub fn list_conflicted_files(backend: &dyn VcsBackend) -> Result<(), CliError> {
    let files = discover_conflicted_files(backend)?;

    if files.is_empty() {
        println!("No conflicted files found");
    } else {
        for file in files {
            println!("{}", file.display());
        }
    }

    Ok(())
}
