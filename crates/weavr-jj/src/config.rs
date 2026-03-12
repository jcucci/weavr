//! Jujutsu configuration reading.

use std::path::Path;
use std::process::Command;

/// The conflict marker style configured in jj.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictMarkerStyle {
    /// Diff-style conflict markers (`%%%%%%%` for diffs).
    Diff,
    /// Snapshot-style conflict markers (`+++++++` for sides, `-------` for base).
    Snapshot,
}

/// Reads the `ui.conflict-marker-style` setting from jj config.
///
/// Runs `jj config get ui.conflict-marker-style` from the given repo root.
/// Returns [`ConflictMarkerStyle::Diff`] as the default if the config key
/// is unset or the command fails.
#[must_use]
pub fn conflict_marker_style(root: &Path) -> ConflictMarkerStyle {
    parse_marker_style(&read_config_value(root))
}

/// Runs `jj config get ui.conflict-marker-style` and returns the raw output.
fn read_config_value(root: &Path) -> String {
    Command::new("jj")
        .args(["config", "get", "ui.conflict-marker-style"])
        .current_dir(root)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

/// Parses a raw config value into a [`ConflictMarkerStyle`].
///
/// Returns [`ConflictMarkerStyle::Diff`] for unrecognized or empty values,
/// matching jj's default behavior.
#[must_use]
pub fn parse_marker_style(value: &str) -> ConflictMarkerStyle {
    match value.trim() {
        "snapshot" => ConflictMarkerStyle::Snapshot,
        _ => ConflictMarkerStyle::Diff,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_diff_style() {
        assert_eq!(parse_marker_style("diff"), ConflictMarkerStyle::Diff);
    }

    #[test]
    fn parse_snapshot_style() {
        assert_eq!(
            parse_marker_style("snapshot"),
            ConflictMarkerStyle::Snapshot
        );
    }

    #[test]
    fn parse_empty_defaults_to_diff() {
        assert_eq!(parse_marker_style(""), ConflictMarkerStyle::Diff);
    }

    #[test]
    fn parse_unknown_defaults_to_diff() {
        assert_eq!(parse_marker_style("unknown"), ConflictMarkerStyle::Diff);
    }

    #[test]
    fn parse_with_whitespace() {
        assert_eq!(
            parse_marker_style("  snapshot  "),
            ConflictMarkerStyle::Snapshot
        );
    }
}
