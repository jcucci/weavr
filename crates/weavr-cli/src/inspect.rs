//! Inspect subcommand — dumps structured conflict data as JSON.

use crate::cli::InspectArgs;
use crate::error::{exit_codes, CliError};
use crate::output::{
    self, JsonInspectContext, JsonInspectFile, JsonInspectHunk, JsonInspectOutput,
};

use weavr_core::MergeSession;

/// Runs the `inspect` subcommand.
pub fn run(args: &InspectArgs) -> Result<i32, CliError> {
    let mut files = Vec::new();

    for path in &args.files {
        let content = std::fs::read_to_string(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                CliError::FileNotFound(path.clone())
            } else {
                CliError::Io(e)
            }
        })?;
        let session = MergeSession::from_conflicted(&content, path.clone())?;

        let hunks: Vec<JsonInspectHunk> = session
            .hunks()
            .iter()
            .map(|h| JsonInspectHunk {
                id: h.id.0,
                left: h.left.text.clone(),
                right: h.right.text.clone(),
                base: h.base.as_ref().map(|b| b.text.clone()),
                context: JsonInspectContext {
                    before: h.context.before.clone(),
                    after: h.context.after.clone(),
                    start_line_left: h.context.start_line_left,
                    start_line_right: h.context.start_line_right,
                },
            })
            .collect();

        files.push(JsonInspectFile {
            file: path.clone(),
            hunks,
        });
    }

    output::print_json(&JsonInspectOutput { files })?;
    Ok(exit_codes::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn conflict_content() -> &'static str {
        "before\n<<<<<<< HEAD\nfn foo() { 1 }\n=======\nfn foo() { 2 }\n>>>>>>> branch\nafter\n"
    }

    fn conflict_content_diff3() -> &'static str {
        "before\n<<<<<<< HEAD\nfn foo() { 1 }\n||||||| base\nfn foo() { 0 }\n=======\nfn foo() { 2 }\n>>>>>>> branch\nafter\n"
    }

    #[test]
    fn inspect_single_file() {
        let mut tmp = NamedTempFile::new().unwrap();
        write!(tmp, "{}", conflict_content()).unwrap();

        let args = InspectArgs {
            files: vec![tmp.path().to_path_buf()],
        };
        let code = run(&args).unwrap();
        assert_eq!(code, exit_codes::SUCCESS);
    }

    #[test]
    fn inspect_hunk_fields() {
        let mut tmp = NamedTempFile::new().unwrap();
        write!(tmp, "{}", conflict_content()).unwrap();

        let content = std::fs::read_to_string(tmp.path()).unwrap();
        let session = MergeSession::from_conflicted(&content, tmp.path().to_path_buf()).unwrap();
        let hunks = session.hunks();

        assert_eq!(hunks.len(), 1);
        assert_eq!(hunks[0].left.text, "fn foo() { 1 }");
        assert_eq!(hunks[0].right.text, "fn foo() { 2 }");
        assert!(hunks[0].base.is_none());
    }

    #[test]
    fn inspect_diff3_has_base() {
        let mut tmp = NamedTempFile::new().unwrap();
        write!(tmp, "{}", conflict_content_diff3()).unwrap();

        let content = std::fs::read_to_string(tmp.path()).unwrap();
        let session = MergeSession::from_conflicted(&content, tmp.path().to_path_buf()).unwrap();
        let hunks = session.hunks();

        assert_eq!(hunks.len(), 1);
        assert!(hunks[0].base.is_some());
        assert_eq!(hunks[0].base.as_ref().unwrap().text, "fn foo() { 0 }");
    }

    #[test]
    fn inspect_multiple_files() {
        let mut tmp1 = NamedTempFile::new().unwrap();
        write!(tmp1, "{}", conflict_content()).unwrap();

        let mut tmp2 = NamedTempFile::new().unwrap();
        write!(tmp2, "{}", conflict_content()).unwrap();

        let args = InspectArgs {
            files: vec![tmp1.path().to_path_buf(), tmp2.path().to_path_buf()],
        };
        let code = run(&args).unwrap();
        assert_eq!(code, exit_codes::SUCCESS);
    }

    #[test]
    fn inspect_no_conflicts() {
        let mut tmp = NamedTempFile::new().unwrap();
        writeln!(tmp, "fn main() {{}}").unwrap();

        let args = InspectArgs {
            files: vec![tmp.path().to_path_buf()],
        };
        let code = run(&args).unwrap();
        assert_eq!(code, exit_codes::SUCCESS);
    }

    #[test]
    fn inspect_file_not_found() {
        let args = InspectArgs {
            files: vec![std::path::PathBuf::from("/nonexistent/file.rs")],
        };
        let result = run(&args);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CliError::FileNotFound(_)));
    }
}
