//! JSON output types for `--format=json` mode.

use std::path::PathBuf;

use serde::Serialize;

/// JSON output for `--check --format=json`.
#[derive(Debug, Serialize)]
pub struct JsonCheckOutput {
    pub files: Vec<JsonCheckFile>,
    pub total_conflicts: usize,
}

/// Per-file entry in check JSON output.
#[derive(Debug, Serialize)]
pub struct JsonCheckFile {
    pub path: PathBuf,
    pub conflict_count: usize,
}

/// JSON output for `--list --format=json`.
#[derive(Debug, Serialize)]
pub struct JsonListOutput {
    pub conflicted_files: Vec<PathBuf>,
}

/// JSON output for `--headless --format=json`.
#[derive(Debug, Serialize)]
pub struct JsonHeadlessOutput {
    pub results: Vec<JsonHeadlessFile>,
}

/// Per-file entry in headless JSON output.
#[derive(Debug, Serialize)]
pub struct JsonHeadlessFile {
    pub path: PathBuf,
    pub hunks_resolved: usize,
    pub strategy: String,
    pub written: bool,
}

/// JSON output for `merge-driver --format=json`.
#[derive(Debug, Serialize)]
pub struct JsonMergeDriverOutput {
    pub path: PathBuf,
    pub hunks_resolved: usize,
    pub clean_merge: bool,
    pub written: bool,
}

/// JSON error wrapper.
#[derive(Debug, Serialize)]
pub struct JsonError {
    pub error: String,
}

/// Serializes the given value as JSON to stdout.
pub fn print_json<T: Serialize>(value: &T) -> Result<(), std::io::Error> {
    let json = serde_json::to_string_pretty(value).map_err(std::io::Error::other)?;
    println!("{json}");
    Ok(())
}

/// Prints a JSON error object to stderr.
pub fn print_json_error(message: &str) {
    let err = JsonError {
        error: message.to_string(),
    };
    if let Ok(json) = serde_json::to_string_pretty(&err) {
        eprintln!("{json}");
    }
}
