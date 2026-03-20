//! JSON output types for `--format=json` mode.

use std::io::Write;
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
    pub hunks: Vec<JsonHunkResult>,
}

/// Per-hunk resolution metadata for JSON output.
#[derive(Debug, Serialize)]
pub struct JsonHunkResult {
    pub hunk_id: u32,
    pub strategy: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explanation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// JSON output for `merge-driver --format=json`.
#[derive(Debug, Serialize)]
pub struct JsonMergeDriverOutput {
    pub path: PathBuf,
    pub hunks_resolved: usize,
    pub clean_merge: bool,
    pub written: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy: Option<String>,
    pub hunks: Vec<JsonHunkResult>,
}

/// JSON output for `inspect` subcommand.
#[derive(Debug, Serialize)]
pub struct JsonInspectOutput {
    pub files: Vec<JsonInspectFile>,
}

/// Per-file entry in inspect JSON output.
#[derive(Debug, Serialize)]
pub struct JsonInspectFile {
    pub file: PathBuf,
    pub hunks: Vec<JsonInspectHunk>,
}

/// Per-hunk entry in inspect JSON output.
#[derive(Debug, Serialize)]
pub struct JsonInspectHunk {
    pub id: u32,
    pub left: String,
    pub right: String,
    pub base: Option<String>,
    pub context: JsonInspectContext,
}

/// Context surrounding a conflict hunk.
#[derive(Debug, Serialize)]
pub struct JsonInspectContext {
    pub before: Vec<String>,
    pub after: Vec<String>,
    pub start_line_left: usize,
    pub start_line_right: usize,
}

/// JSON output for `resolve` subcommand.
#[derive(Debug, Serialize)]
pub struct JsonResolveOutput {
    pub results: Vec<JsonResolveFile>,
}

/// Per-file entry in resolve JSON output.
#[derive(Debug, Serialize)]
pub struct JsonResolveFile {
    pub path: PathBuf,
    pub total_hunks: usize,
    pub resolved_hunks: usize,
    pub unresolved_hunks: Vec<u32>,
    pub written: bool,
    pub hunks: Vec<JsonHunkResult>,
}

/// JSON error wrapper.
#[derive(Debug, Serialize)]
pub struct JsonError {
    pub error: String,
}

/// Serializes the given value as pretty-printed JSON to the given writer.
fn print_json_to<T: Serialize>(writer: &mut impl Write, value: &T) -> Result<(), std::io::Error> {
    serde_json::to_writer_pretty(&mut *writer, value).map_err(std::io::Error::other)?;
    writeln!(writer)?;
    Ok(())
}

/// Serializes the given value as JSON to stdout.
pub fn print_json<T: Serialize>(value: &T) -> Result<(), std::io::Error> {
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    print_json_to(&mut handle, value)
}

/// Serializes the given value as JSON to stderr.
pub fn print_json_stderr<T: Serialize>(value: &T) -> Result<(), std::io::Error> {
    let stderr = std::io::stderr();
    let mut handle = stderr.lock();
    print_json_to(&mut handle, value)
}

/// Appends the given value as JSON to the specified file.
pub fn print_json_file<T: Serialize>(
    path: &std::path::Path,
    value: &T,
) -> Result<(), std::io::Error> {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    print_json_to(&mut file, value)
}

/// Prints a JSON error object to stderr.
pub fn print_json_error(message: &str) {
    let err = JsonError {
        error: message.to_string(),
    };
    let stderr = std::io::stderr();
    let mut handle = stderr.lock();
    if serde_json::to_writer_pretty(&mut handle, &err).is_ok() {
        let _ = writeln!(handle);
    }
}
