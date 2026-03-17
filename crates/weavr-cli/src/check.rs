//! Conflict-checking logic for `--check` mode.

use std::path::{Path, PathBuf};

use crate::cli::OutputFormat;
use crate::error::CliError;
use crate::output;

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
    let has_markers =
        content.contains("<<<<<<<") && content.contains("=======") && content.contains(">>>>>>>");
    let conflict_count = if has_markers {
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
pub fn print_summary(results: &[CheckResult], format: OutputFormat) -> Result<(), CliError> {
    match format {
        OutputFormat::Json => {
            let total_conflicts = results.iter().map(|r| r.conflict_count).sum();
            let json = output::JsonCheckOutput {
                files: results
                    .iter()
                    .map(|r| output::JsonCheckFile {
                        path: r.path.clone(),
                        conflict_count: r.conflict_count,
                    })
                    .collect(),
                total_conflicts,
            };
            output::print_json(&json)?;
        }
        OutputFormat::Text => {
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
    }
    Ok(())
}
