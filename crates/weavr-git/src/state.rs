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
}
