//! Parser for `jj status` output.

use std::path::PathBuf;

/// Parses `jj status` output and extracts conflicted file paths.
///
/// Jujutsu marks conflicted files with a `C` prefix in status output, e.g.:
/// ```text
/// Working copy changes:
/// C src/main.rs
/// M src/lib.rs
/// ```
///
/// Only lines starting with `C ` are extracted as conflicts.
#[must_use]
pub fn parse_jj_status(output: &str) -> Vec<PathBuf> {
    output
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            trimmed
                .strip_prefix("C ")
                .map(|path| PathBuf::from(path.trim()))
        })
        .collect()
}

/// Parses `jj status` output and extracts modified (non-conflicted) file paths.
///
/// Returns paths for entries with status prefixes like `M `, `A `, `D `, `R ` —
/// anything that represents a dirty file but is NOT a conflict (`C `).
#[must_use]
pub fn parse_jj_modified(output: &str) -> Vec<PathBuf> {
    output
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            // Must be a single-letter status followed by a space
            if trimmed.len() < 3 {
                return None;
            }
            let first = trimmed.as_bytes()[0];
            let second = trimmed.as_bytes()[1];
            // Skip conflicts (handled separately) and non-status lines
            if second != b' ' || first == b'C' {
                return None;
            }
            // Accept known status letters: M(odified), A(dded), D(eleted), R(enamed)
            if matches!(first, b'M' | b'A' | b'D' | b'R') {
                Some(PathBuf::from(trimmed[2..].trim()))
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_output() {
        let result = parse_jj_status("");
        assert!(result.is_empty());
    }

    #[test]
    fn parse_no_conflicts() {
        let output = "Working copy changes:\nM src/lib.rs\nA new_file.rs\n";
        let result = parse_jj_status(output);
        assert!(result.is_empty());
    }

    #[test]
    fn parse_single_conflict() {
        let output = "Working copy changes:\nC src/main.rs\nM src/lib.rs\n";
        let result = parse_jj_status(output);
        assert_eq!(result, vec![PathBuf::from("src/main.rs")]);
    }

    #[test]
    fn parse_multiple_conflicts() {
        let output = "Working copy changes:\nC src/main.rs\nC src/lib.rs\nC tests/test.rs\n";
        let result = parse_jj_status(output);
        assert_eq!(
            result,
            vec![
                PathBuf::from("src/main.rs"),
                PathBuf::from("src/lib.rs"),
                PathBuf::from("tests/test.rs"),
            ]
        );
    }

    #[test]
    fn parse_mixed_status_lines() {
        let output = "\
Working copy changes:
M modified.rs
C conflicted.rs
A added.rs
D deleted.rs
C another_conflict.rs
";
        let result = parse_jj_status(output);
        assert_eq!(
            result,
            vec![
                PathBuf::from("conflicted.rs"),
                PathBuf::from("another_conflict.rs"),
            ]
        );
    }

    #[test]
    fn parse_with_leading_whitespace() {
        let output = "  C src/main.rs\n";
        let result = parse_jj_status(output);
        assert_eq!(result, vec![PathBuf::from("src/main.rs")]);
    }

    #[test]
    fn parse_nested_path() {
        let output = "C deep/nested/path/file.rs\n";
        let result = parse_jj_status(output);
        assert_eq!(result, vec![PathBuf::from("deep/nested/path/file.rs")]);
    }

    #[test]
    fn ignores_lines_without_c_prefix() {
        let output = "The working copy is clean\nNothing to see here\n";
        let result = parse_jj_status(output);
        assert!(result.is_empty());
    }

    #[test]
    fn parse_path_with_spaces() {
        let output = "C path with spaces/file.rs\n";
        let result = parse_jj_status(output);
        assert_eq!(result, vec![PathBuf::from("path with spaces/file.rs")]);
    }

    #[test]
    fn modified_empty_output() {
        let result = parse_jj_modified("");
        assert!(result.is_empty());
    }

    #[test]
    fn modified_picks_up_modified_files() {
        let output = "Working copy changes:\nM src/lib.rs\nA new_file.rs\n";
        let result = parse_jj_modified(output);
        assert_eq!(
            result,
            vec![PathBuf::from("src/lib.rs"), PathBuf::from("new_file.rs")]
        );
    }

    #[test]
    fn modified_skips_conflicts() {
        let output = "C conflict.rs\nM modified.rs\n";
        let result = parse_jj_modified(output);
        assert_eq!(result, vec![PathBuf::from("modified.rs")]);
    }

    #[test]
    fn modified_skips_non_status_lines() {
        let output = "Working copy changes:\nM src/lib.rs\nThe working copy is clean\n";
        let result = parse_jj_modified(output);
        assert_eq!(result, vec![PathBuf::from("src/lib.rs")]);
    }

    #[test]
    fn modified_picks_up_deleted() {
        let output = "D removed.rs\n";
        let result = parse_jj_modified(output);
        assert_eq!(result, vec![PathBuf::from("removed.rs")]);
    }
}
