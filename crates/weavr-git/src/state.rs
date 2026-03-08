//! Git repository state detection.

/// The type of Git operation currently in progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitOperation {
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
}

impl GitOperation {
    /// Returns true if any conflict-producing operation is in progress.
    #[must_use]
    pub fn has_conflicts(&self) -> bool {
        !matches!(self, GitOperation::None)
    }
}

impl From<GitOperation> for weavr_vcs::VcsOperation {
    fn from(op: GitOperation) -> Self {
        match op {
            GitOperation::None => Self::None,
            GitOperation::Merge => Self::Merge,
            GitOperation::Rebase => Self::Rebase,
            GitOperation::CherryPick => Self::CherryPick,
            GitOperation::Revert => Self::Revert,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn none_has_no_conflicts() {
        assert!(!GitOperation::None.has_conflicts());
    }

    #[test]
    fn merge_has_conflicts() {
        assert!(GitOperation::Merge.has_conflicts());
    }

    #[test]
    fn rebase_has_conflicts() {
        assert!(GitOperation::Rebase.has_conflicts());
    }

    #[test]
    fn cherry_pick_has_conflicts() {
        assert!(GitOperation::CherryPick.has_conflicts());
    }

    #[test]
    fn revert_has_conflicts() {
        assert!(GitOperation::Revert.has_conflicts());
    }

    #[test]
    fn none_converts_to_vcs_none() {
        assert_eq!(
            weavr_vcs::VcsOperation::from(GitOperation::None),
            weavr_vcs::VcsOperation::None
        );
    }

    #[test]
    fn merge_converts_to_vcs_merge() {
        assert_eq!(
            weavr_vcs::VcsOperation::from(GitOperation::Merge),
            weavr_vcs::VcsOperation::Merge
        );
    }

    #[test]
    fn rebase_converts_to_vcs_rebase() {
        assert_eq!(
            weavr_vcs::VcsOperation::from(GitOperation::Rebase),
            weavr_vcs::VcsOperation::Rebase
        );
    }

    #[test]
    fn cherry_pick_converts_to_vcs_cherry_pick() {
        assert_eq!(
            weavr_vcs::VcsOperation::from(GitOperation::CherryPick),
            weavr_vcs::VcsOperation::CherryPick
        );
    }

    #[test]
    fn revert_converts_to_vcs_revert() {
        assert_eq!(
            weavr_vcs::VcsOperation::from(GitOperation::Revert),
            weavr_vcs::VcsOperation::Revert
        );
    }
}
