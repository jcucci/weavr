//! Parser for `git status --porcelain=v1` output.

use std::path::PathBuf;

use weavr_vcs::ConflictKind;

/// Conflict type from git status porcelain output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictType {
    /// Both sides modified the same file (UU).
    BothModified,
    /// Both sides added the same file (AA).
    BothAdded,
    /// Both sides deleted the same file (DD).
    BothDeleted,
    /// Added by us, deleted by them (AU or UD).
    AddedByUsDeletedByThem,
    /// Added by them, deleted by us (UA or DU).
    AddedByThemDeletedByUs,
}

/// A conflicted file entry from porcelain output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictEntry {
    /// The path to the conflicted file.
    pub path: PathBuf,
    /// The type of conflict.
    pub conflict_type: ConflictType,
}

impl From<ConflictType> for ConflictKind {
    fn from(ct: ConflictType) -> Self {
        match ct {
            ConflictType::BothModified => Self::BothModified,
            ConflictType::BothAdded => Self::BothAdded,
            ConflictType::BothDeleted => Self::BothDeleted,
            ConflictType::AddedByUsDeletedByThem => Self::AddDelete,
            ConflictType::AddedByThemDeletedByUs => Self::DeleteAdd,
        }
    }
}

/// Parses `git status --porcelain=v1` output and extracts conflicted files.
///
/// The porcelain format uses two characters for status:
/// - `UU` = both modified (merge conflict)
/// - `AA` = both added
/// - `DD` = both deleted
/// - `AU`, `UD` = added by us, deleted by them
/// - `UA`, `DU` = added by them, deleted by us
///
/// Handles quoted filenames with C-style escape sequences.
#[must_use]
pub fn parse_porcelain_v1(output: &str) -> Vec<ConflictEntry> {
    output
        .lines()
        .filter_map(|line| {
            if line.len() < 4 {
                return None;
            }

            let xy = &line[0..2];
            let conflict_type = is_unmerged(xy)?;

            // Path starts at position 3 (after "XY ")
            let raw_path = &line[3..];
            let path = PathBuf::from(unquote_path(raw_path));

            Some(ConflictEntry {
                path,
                conflict_type,
            })
        })
        .collect()
}

/// Checks if a porcelain status code indicates an unmerged state.
fn is_unmerged(xy: &str) -> Option<ConflictType> {
    match xy {
        "UU" => Some(ConflictType::BothModified),
        "AA" => Some(ConflictType::BothAdded),
        "DD" => Some(ConflictType::BothDeleted),
        "AU" | "UD" => Some(ConflictType::AddedByUsDeletedByThem),
        "UA" | "DU" => Some(ConflictType::AddedByThemDeletedByUs),
        _ => None,
    }
}

/// Unquotes a Git-quoted path string.
///
/// Git quotes filenames containing special characters (spaces, quotes, newlines,
/// non-ASCII) using C-style escaping. This function handles:
/// - `\\` -> `\`
/// - `\"` -> `"`
/// - `\n` -> newline
/// - `\t` -> tab
/// - `\xxx` -> octal escape sequences
fn unquote_path(s: &str) -> String {
    // If not quoted, return as-is
    if !s.starts_with('"') {
        return s.to_string();
    }

    // Remove surrounding quotes
    let inner = s
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(s);

    let mut result = String::with_capacity(inner.len());
    let mut chars = inner.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('\\') | None => result.push('\\'),
                Some('"') => result.push('"'),
                Some('n') => result.push('\n'),
                Some('t') => result.push('\t'),
                Some('r') => result.push('\r'),
                // Octal escape sequence (e.g., \302\240 for non-breaking space)
                Some(d1) if d1.is_ascii_digit() => {
                    let mut octal = String::new();
                    octal.push(d1);
                    // Collect up to 2 more octal digits
                    for _ in 0..2 {
                        if let Some(&d) = chars.peek() {
                            if d.is_ascii_digit() && d < '8' {
                                octal.push(chars.next().unwrap());
                            } else {
                                break;
                            }
                        }
                    }
                    if let Ok(byte) = u8::from_str_radix(&octal, 8) {
                        result.push(byte as char);
                    }
                }
                Some(other) => {
                    // Unknown escape, preserve literally
                    result.push('\\');
                    result.push(other);
                }
            }
        } else {
            result.push(c);
        }
    }

    result
}

/// Parses `git status --porcelain=v1` output and extracts modified (non-unmerged) file paths.
///
/// Returns paths for entries with status codes like ` M`, `M `, `MM`, `A `, `??`, etc. —
/// anything that represents a dirty file but is NOT an unmerged conflict entry.
#[must_use]
pub fn parse_modified_v1(output: &str) -> Vec<PathBuf> {
    output
        .lines()
        .filter_map(|line| {
            if line.len() < 4 {
                return None;
            }

            let xy = &line[0..2];
            // Skip unmerged entries — those are handled by parse_porcelain_v1
            if is_unmerged(xy).is_some() {
                return None;
            }

            // Skip entries with no status (shouldn't happen, but be safe)
            if xy == "  " {
                return None;
            }

            let raw_path = &line[3..];
            // For rename/copy entries, path contains " -> new_path"; use the new path
            let path_str = raw_path.split(" -> ").last().unwrap_or(raw_path);
            Some(PathBuf::from(unquote_path(path_str)))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_output() {
        let entries = parse_porcelain_v1("");
        assert!(entries.is_empty());
    }

    #[test]
    fn parse_no_conflicts() {
        let output = " M src/modified.rs\n?? untracked.txt\nA  staged.rs\n";
        let entries = parse_porcelain_v1(output);
        assert!(entries.is_empty());
    }

    #[test]
    fn parse_uu_conflict() {
        let output = "UU src/conflict.rs\n";
        let entries = parse_porcelain_v1(output);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, PathBuf::from("src/conflict.rs"));
        assert_eq!(entries[0].conflict_type, ConflictType::BothModified);
    }

    #[test]
    fn parse_aa_conflict() {
        let output = "AA both_added.rs\n";
        let entries = parse_porcelain_v1(output);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].conflict_type, ConflictType::BothAdded);
    }

    #[test]
    fn parse_dd_conflict() {
        let output = "DD both_deleted.rs\n";
        let entries = parse_porcelain_v1(output);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].conflict_type, ConflictType::BothDeleted);
    }

    #[test]
    fn parse_au_conflict() {
        let output = "AU added_by_us.rs\n";
        let entries = parse_porcelain_v1(output);
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].conflict_type,
            ConflictType::AddedByUsDeletedByThem
        );
    }

    #[test]
    fn parse_ua_conflict() {
        let output = "UA added_by_them.rs\n";
        let entries = parse_porcelain_v1(output);
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].conflict_type,
            ConflictType::AddedByThemDeletedByUs
        );
    }

    #[test]
    fn parse_multiple_conflicts() {
        let output = "UU file1.rs\nAA file2.rs\nDD file3.rs\n M normal.rs\n";
        let entries = parse_porcelain_v1(output);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].path, PathBuf::from("file1.rs"));
        assert_eq!(entries[1].path, PathBuf::from("file2.rs"));
        assert_eq!(entries[2].path, PathBuf::from("file3.rs"));
    }

    #[test]
    fn parse_path_with_spaces() {
        let output = "UU path with spaces/file.rs\n";
        let entries = parse_porcelain_v1(output);
        assert_eq!(entries[0].path, PathBuf::from("path with spaces/file.rs"));
    }

    #[test]
    fn parse_mixed_with_non_conflicts() {
        let output = " M modified.rs\nUU conflict.rs\n?? untracked.rs\n";
        let entries = parse_porcelain_v1(output);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, PathBuf::from("conflict.rs"));
    }

    #[test]
    fn parse_short_line_ignored() {
        let output = "UU\n"; // Too short, no path
        let entries = parse_porcelain_v1(output);
        assert!(entries.is_empty());
    }

    #[test]
    fn parse_nested_path() {
        let output = "UU src/deep/nested/file.rs\n";
        let entries = parse_porcelain_v1(output);
        assert_eq!(entries[0].path, PathBuf::from("src/deep/nested/file.rs"));
    }

    #[test]
    fn unquote_simple_path() {
        assert_eq!(unquote_path("simple.rs"), "simple.rs");
    }

    #[test]
    fn unquote_quoted_path_with_spaces() {
        assert_eq!(
            unquote_path("\"path with spaces.rs\""),
            "path with spaces.rs"
        );
    }

    #[test]
    fn unquote_escaped_quotes() {
        assert_eq!(unquote_path("\"file\\\"name\\\".rs\""), "file\"name\".rs");
    }

    #[test]
    fn unquote_escaped_backslash() {
        assert_eq!(unquote_path("\"path\\\\file.rs\""), "path\\file.rs");
    }

    #[test]
    fn unquote_escaped_newline() {
        assert_eq!(unquote_path("\"file\\nname.rs\""), "file\nname.rs");
    }

    #[test]
    fn unquote_escaped_tab() {
        assert_eq!(unquote_path("\"file\\tname.rs\""), "file\tname.rs");
    }

    #[test]
    fn unquote_octal_escape() {
        // \101 is octal for 'A' (65 decimal)
        assert_eq!(unquote_path("\"\\101.rs\""), "A.rs");
    }

    #[test]
    fn parse_quoted_conflict() {
        let output = "UU \"file with \\\"quotes\\\".rs\"\n";
        let entries = parse_porcelain_v1(output);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, PathBuf::from("file with \"quotes\".rs"));
    }

    #[test]
    fn both_modified_converts_to_conflict_kind() {
        assert_eq!(
            ConflictKind::from(ConflictType::BothModified),
            ConflictKind::BothModified
        );
    }

    #[test]
    fn both_added_converts_to_conflict_kind() {
        assert_eq!(
            ConflictKind::from(ConflictType::BothAdded),
            ConflictKind::BothAdded
        );
    }

    #[test]
    fn both_deleted_converts_to_conflict_kind() {
        assert_eq!(
            ConflictKind::from(ConflictType::BothDeleted),
            ConflictKind::BothDeleted
        );
    }

    #[test]
    fn added_by_us_converts_to_add_delete() {
        assert_eq!(
            ConflictKind::from(ConflictType::AddedByUsDeletedByThem),
            ConflictKind::AddDelete
        );
    }

    #[test]
    fn added_by_them_converts_to_delete_add() {
        assert_eq!(
            ConflictKind::from(ConflictType::AddedByThemDeletedByUs),
            ConflictKind::DeleteAdd
        );
    }

    #[test]
    fn modified_empty_output() {
        let result = parse_modified_v1("");
        assert!(result.is_empty());
    }

    #[test]
    fn modified_picks_up_worktree_modified() {
        let output = " M src/modified.rs\n";
        let result = parse_modified_v1(output);
        assert_eq!(result, vec![PathBuf::from("src/modified.rs")]);
    }

    #[test]
    fn modified_picks_up_index_modified() {
        let output = "M  src/staged.rs\n";
        let result = parse_modified_v1(output);
        assert_eq!(result, vec![PathBuf::from("src/staged.rs")]);
    }

    #[test]
    fn modified_picks_up_both_modified() {
        let output = "MM src/both.rs\n";
        let result = parse_modified_v1(output);
        assert_eq!(result, vec![PathBuf::from("src/both.rs")]);
    }

    #[test]
    fn modified_picks_up_added() {
        let output = "A  src/new.rs\n";
        let result = parse_modified_v1(output);
        assert_eq!(result, vec![PathBuf::from("src/new.rs")]);
    }

    #[test]
    fn modified_picks_up_untracked() {
        let output = "?? untracked.txt\n";
        let result = parse_modified_v1(output);
        assert_eq!(result, vec![PathBuf::from("untracked.txt")]);
    }

    #[test]
    fn modified_skips_unmerged_entries() {
        let output = "UU conflict.rs\n M modified.rs\nAA both_added.rs\n";
        let result = parse_modified_v1(output);
        assert_eq!(result, vec![PathBuf::from("modified.rs")]);
    }

    #[test]
    fn modified_mixed_entries() {
        let output = " M modified.rs\nUU conflict.rs\n?? untracked.rs\nA  staged.rs\n";
        let result = parse_modified_v1(output);
        assert_eq!(
            result,
            vec![
                PathBuf::from("modified.rs"),
                PathBuf::from("untracked.rs"),
                PathBuf::from("staged.rs"),
            ]
        );
    }

    #[test]
    fn modified_handles_rename() {
        let output = "R  old_name.rs -> new_name.rs\n";
        let result = parse_modified_v1(output);
        assert_eq!(result, vec![PathBuf::from("new_name.rs")]);
    }
}
