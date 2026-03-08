//! Conflict format detection.
//!
//! All types in this module are **stable** and covered by semantic versioning.

use serde::{Deserialize, Serialize};

/// The conflict marker format used in a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictFormat {
    /// Standard Git conflict markers (`<<<<<<<`/`=======`/`>>>>>>>`).
    Git,
    /// Jujutsu snapshot format (`+++++++` for sides, `-------` for base).
    JjSnapshot,
    /// Jujutsu diff format (not yet supported).
    JjDiff,
}

/// Detects the conflict marker format used in file content.
///
/// Scans for the first `<<<<<<<` block, then inspects the first inner marker
/// to determine the format:
/// - `+++++++` → `JjSnapshot`
/// - `|||||||` or `=======` → `Git`
///
/// Returns `None` if no conflict markers are found.
#[must_use]
pub fn detect_format(content: &str) -> Option<ConflictFormat> {
    let mut in_conflict = false;

    for line in content.lines() {
        if in_conflict {
            // First inner marker determines the format
            if line.starts_with("+++++++") {
                return Some(ConflictFormat::JjSnapshot);
            }
            if line.starts_with("|||||||") || line.starts_with("=======") {
                return Some(ConflictFormat::Git);
            }
            // For jj diff format, the marker after <<<<<<< would be different;
            // for now we keep scanning content lines until we hit a marker
        } else if line.starts_with("<<<<<<<") {
            in_conflict = true;
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_format_git_two_way() {
        let content = "before\n<<<<<<< HEAD\nleft\n=======\nright\n>>>>>>> feature\nafter";
        assert_eq!(detect_format(content), Some(ConflictFormat::Git));
    }

    #[test]
    fn detect_format_git_diff3() {
        let content = "<<<<<<< HEAD\nleft\n||||||| base\nbase\n=======\nright\n>>>>>>> feature";
        assert_eq!(detect_format(content), Some(ConflictFormat::Git));
    }

    #[test]
    fn detect_format_jj_snapshot() {
        let content = "<<<<<<< Conflict 1 of 1\n+++++++ Side #1\nleft\n------- Base\nbase\n+++++++ Side #2\nright\n>>>>>>> Conflict 1 of 1";
        assert_eq!(detect_format(content), Some(ConflictFormat::JjSnapshot));
    }

    #[test]
    fn detect_format_no_conflicts() {
        let content = "just normal content\nno conflicts here";
        assert_eq!(detect_format(content), None);
    }

    #[test]
    fn detect_format_unclosed_no_inner_marker() {
        let content = "<<<<<<< HEAD\nsome content but no inner markers";
        assert_eq!(detect_format(content), None);
    }
}
