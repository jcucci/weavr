//! Parser for `git status --porcelain=v1` output.

use std::path::PathBuf;

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

/// Parses `git status --porcelain=v1` output and extracts conflicted files.
///
/// The porcelain format uses two characters for status:
/// - `UU` = both modified (merge conflict)
/// - `AA` = both added
/// - `DD` = both deleted
/// - `AU`, `UD` = added by us, deleted by them
/// - `UA`, `DU` = added by them, deleted by us
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
            let path = PathBuf::from(line[3..].to_string());

            Some(ConflictEntry { path, conflict_type })
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
        assert_eq!(entries[0].conflict_type, ConflictType::AddedByUsDeletedByThem);
    }

    #[test]
    fn parse_ua_conflict() {
        let output = "UA added_by_them.rs\n";
        let entries = parse_porcelain_v1(output);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].conflict_type, ConflictType::AddedByThemDeletedByUs);
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
}
