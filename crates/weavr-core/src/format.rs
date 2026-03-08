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
            // First inner marker determines the format.
            // Use boundary-aware checks matching the parser logic.
            if is_marker_run(line, b'+') {
                return Some(ConflictFormat::JjSnapshot);
            }
            if is_marker_run(line, b'|') || is_separator_marker(line) {
                return Some(ConflictFormat::Git);
            }
            // For jj diff format, the marker after <<<<<<< would be different;
            // for now we keep scanning content lines until we hit a marker
        } else if is_marker_run(line, b'<') {
            in_conflict = true;
        }
    }

    None
}

/// Checks if a line is a run of 7+ identical `ch` characters, followed by
/// either end-of-line or a space. This matches the parser's marker validation.
fn is_marker_run(line: &str, ch: u8) -> bool {
    let bytes = line.as_bytes();
    if bytes.len() < 7 {
        return false;
    }

    let count = bytes.iter().take_while(|&&b| b == ch).count();
    if count < 7 {
        return false;
    }

    count == bytes.len() || bytes.get(count) == Some(&b' ')
}

/// Checks if a line is a Git `=======` separator (exactly 7 `=`, then only whitespace).
fn is_separator_marker(line: &str) -> bool {
    let bytes = line.as_bytes();
    bytes.len() >= 7
        && bytes.iter().take(7).all(|&b| b == b'=')
        && bytes[7..].iter().all(u8::is_ascii_whitespace)
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
