//! Merge result types.
//!
//! All types in this module are **stable** and covered by semantic versioning.

use serde::{Deserialize, Serialize};

use crate::HunkId;

/// A merge warning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeWarning {
    /// Warning message.
    pub message: String,
    /// Associated hunk, if any.
    pub hunk_id: Option<HunkId>,
}

/// Summary statistics for a merge.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MergeSummary {
    /// Total number of conflict hunks.
    pub total_hunks: usize,
    /// Number of resolved hunks.
    pub resolved_hunks: usize,
}

/// Final output of a merge session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeResult {
    /// The merged file content.
    pub content: String,
    /// Any hunks that remain unresolved.
    pub unresolved_hunks: Vec<HunkId>,
    /// Warnings generated during merge.
    pub warnings: Vec<MergeWarning>,
    /// Statistics summary.
    pub summary: MergeSummary,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_summary_default() {
        let summary = MergeSummary::default();
        assert_eq!(summary.total_hunks, 0);
        assert_eq!(summary.resolved_hunks, 0);
    }

    #[test]
    fn merge_warning_with_hunk() {
        let warning = MergeWarning {
            message: String::from("potential conflict"),
            hunk_id: Some(HunkId(1)),
        };
        assert_eq!(warning.hunk_id, Some(HunkId(1)));
    }

    #[test]
    fn merge_warning_without_hunk() {
        let warning = MergeWarning {
            message: String::from("general warning"),
            hunk_id: None,
        };
        assert!(warning.hunk_id.is_none());
    }

    #[test]
    fn merge_result_creation() {
        let result = MergeResult {
            content: String::from("merged content"),
            unresolved_hunks: vec![],
            warnings: vec![],
            summary: MergeSummary {
                total_hunks: 2,
                resolved_hunks: 2,
            },
        };
        assert_eq!(result.summary.total_hunks, 2);
        assert!(result.unresolved_hunks.is_empty());
    }
}
