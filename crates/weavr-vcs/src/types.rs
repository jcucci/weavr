//! VCS-agnostic types for conflict detection and operation tracking.

use std::path::PathBuf;

/// The type of VCS operation currently in progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VcsOperation {
    /// Normal state, no operation in progress.
    None,
    /// Merge in progress.
    Merge,
    /// Rebase in progress.
    Rebase,
    /// Cherry-pick in progress.
    CherryPick,
    /// Revert in progress.
    Revert,
    /// An operation not covered by other variants.
    Other,
}

impl VcsOperation {
    /// Returns true if any conflict-producing operation is in progress.
    #[must_use]
    pub fn has_conflicts(&self) -> bool {
        !matches!(self, VcsOperation::None)
    }
}

/// The kind of conflict detected in a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictKind {
    /// Both sides modified the same file.
    BothModified,
    /// Both sides added the same file.
    BothAdded,
    /// Both sides deleted the same file.
    BothDeleted,
    /// Local side added the file, incoming side deleted it.
    AddDelete,
    /// Local side deleted the file, incoming side added it.
    DeleteAdd,
    /// File was renamed differently on each side.
    Rename,
    /// A conflict kind not covered by other variants.
    Other,
}

/// A file with a detected conflict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictedFile {
    /// The path to the conflicted file, relative to the repository root.
    pub path: PathBuf,
    /// The kind of conflict.
    pub kind: ConflictKind,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn none_has_no_conflicts() {
        assert!(!VcsOperation::None.has_conflicts());
    }

    #[test]
    fn merge_has_conflicts() {
        assert!(VcsOperation::Merge.has_conflicts());
    }

    #[test]
    fn rebase_has_conflicts() {
        assert!(VcsOperation::Rebase.has_conflicts());
    }

    #[test]
    fn cherry_pick_has_conflicts() {
        assert!(VcsOperation::CherryPick.has_conflicts());
    }

    #[test]
    fn revert_has_conflicts() {
        assert!(VcsOperation::Revert.has_conflicts());
    }

    #[test]
    fn other_has_conflicts() {
        assert!(VcsOperation::Other.has_conflicts());
    }

    #[test]
    fn conflicted_file_stores_path_and_kind() {
        let file = ConflictedFile {
            path: PathBuf::from("src/main.rs"),
            kind: ConflictKind::BothModified,
        };
        assert_eq!(file.path, PathBuf::from("src/main.rs"));
        assert_eq!(file.kind, ConflictKind::BothModified);
    }

    #[test]
    fn conflict_kind_equality() {
        assert_eq!(ConflictKind::AddDelete, ConflictKind::AddDelete);
        assert_ne!(ConflictKind::AddDelete, ConflictKind::DeleteAdd);
    }
}
