//! Conflict-checking logic for `--check` mode.

use std::path::{Path, PathBuf};

use crate::error::CliError;

/// Result of checking a single file for conflicts.
pub struct CheckResult {
    pub path: PathBuf,
    pub conflict_count: usize,
}

/// Reads a file and counts conflict hunks using the core parser.
pub fn check_file(path: &Path) -> Result<CheckResult, CliError> {
    if !path.exists() {
        return Err(CliError::FileNotFound(path.to_path_buf()));
    }
    let content = std::fs::read_to_string(path)?;
    let conflict_count = if content.contains("<<<<<<<") {
        let parsed = weavr_core::parse_conflict_markers(&content)?;
        parsed.hunks.len()
    } else {
        0
    };
    Ok(CheckResult {
        path: path.to_path_buf(),
        conflict_count,
    })
}

/// Prints a per-file summary and totals line.
pub fn print_summary(results: &[CheckResult]) {
    let mut total_files_with_conflicts = 0;
    let mut total_conflicts = 0;

    for result in results {
        println!(
            "{}: {} conflict(s)",
            result.path.display(),
            result.conflict_count
        );
        if result.conflict_count > 0 {
            total_files_with_conflicts += 1;
            total_conflicts += result.conflict_count;
        }
    }

    if results.len() > 1 {
        println!("\n{total_conflicts} conflict(s) in {total_files_with_conflicts} file(s)");
    }
}
