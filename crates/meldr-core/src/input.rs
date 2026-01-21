//! Input types for merge operations.
//!
//! All types in this module are **stable** and covered by semantic versioning.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Represents a single version of a file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileVersion {
    /// Path to the file (for identification, not IO).
    pub path: PathBuf,
    /// File contents as a string.
    pub content: String,
}

/// Represents the raw inputs to a merge operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeInput {
    /// The left (`HEAD`) version.
    pub left: FileVersion,
    /// The right (`MERGE_HEAD`) version.
    pub right: FileVersion,
    /// Optional base for 3-way merge.
    pub base: Option<FileVersion>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_version_creation() {
        let version = FileVersion {
            path: PathBuf::from("test.rs"),
            content: String::from("fn main() {}"),
        };
        assert_eq!(version.path, PathBuf::from("test.rs"));
    }

    #[test]
    fn merge_input_two_way() {
        let input = MergeInput {
            left: FileVersion {
                path: PathBuf::from("test.rs"),
                content: String::from("left"),
            },
            right: FileVersion {
                path: PathBuf::from("test.rs"),
                content: String::from("right"),
            },
            base: None,
        };
        assert!(input.base.is_none());
    }

    #[test]
    fn merge_input_three_way() {
        let input = MergeInput {
            left: FileVersion {
                path: PathBuf::from("test.rs"),
                content: String::from("left"),
            },
            right: FileVersion {
                path: PathBuf::from("test.rs"),
                content: String::from("right"),
            },
            base: Some(FileVersion {
                path: PathBuf::from("test.rs"),
                content: String::from("base"),
            }),
        };
        assert!(input.base.is_some());
    }
}
