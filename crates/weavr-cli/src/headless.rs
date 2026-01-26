//! Headless mode implementation.

use std::path::{Path, PathBuf};

use crate::cli::Strategy;
use crate::error::CliError;

/// Result of headless processing for a single file.
pub struct HeadlessResult {
    /// Path to the processed file.
    pub path: PathBuf,
    /// Number of hunks that were resolved.
    pub hunks_resolved: usize,
    /// The merged output content.
    pub output: String,
}

/// Runs headless merge on a single file.
pub fn process_file(
    path: &Path,
    strategy: Strategy,
    dedupe: bool,
) -> Result<HeadlessResult, CliError> {
    let content = std::fs::read_to_string(path)?;
    let mut session = weavr_core::MergeSession::from_conflicted(&content, path.to_path_buf())?;

    let hunks: Vec<_> = session.hunks().to_vec();

    // Handle files without conflicts (already clean)
    if hunks.is_empty() {
        return Ok(HeadlessResult {
            path: path.to_path_buf(),
            hunks_resolved: 0,
            output: content,
        });
    }

    for hunk in &hunks {
        let resolution = match strategy {
            Strategy::Left => weavr_core::Resolution::accept_left(hunk),
            Strategy::Right => weavr_core::Resolution::accept_right(hunk),
            Strategy::Both => {
                let options = weavr_core::AcceptBothOptions {
                    order: weavr_core::BothOrder::LeftThenRight,
                    deduplicate: dedupe,
                    trim_whitespace: false,
                };
                weavr_core::Resolution::accept_both(hunk, &options)
            }
        };

        session.set_resolution(hunk.id, resolution)?;
    }

    session.apply()?;
    session.validate()?;
    let result = session.complete()?;

    Ok(HeadlessResult {
        path: path.to_path_buf(),
        hunks_resolved: result.summary.resolved_hunks,
        output: result.content,
    })
}

/// Writes the result to the file or prints it for dry-run.
pub fn write_or_print(result: &HeadlessResult, dry_run: bool) -> Result<(), CliError> {
    if dry_run {
        println!("=== {} ===", result.path.display());
        print!("{}", result.output);
    } else {
        std::fs::write(&result.path, &result.output)?;
        println!(
            "{}: {} hunks resolved",
            result.path.display(),
            result.hunks_resolved
        );
    }
    Ok(())
}
