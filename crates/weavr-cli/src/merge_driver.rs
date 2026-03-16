//! Git merge driver integration.
//!
//! Implements the protocol expected by `git merge-driver`:
//! the driver receives base, ours, and theirs file paths, merges them,
//! writes the result to the ours path, and exits 0 on success.

use std::path::Path;

use crate::cli::{MergeDriverArgs, Strategy};
use crate::config::WeavrConfig;
use crate::error::CliError;

/// Runs the merge driver with the given arguments and configuration.
pub fn run(args: &MergeDriverArgs, config: &WeavrConfig) -> Result<i32, CliError> {
    let strategy = resolve_strategy(args.strategy, config);
    let output_path = args.output.as_ref().unwrap_or(&args.ours);

    let marker_size = args.marker_size.unwrap_or(7);

    if marker_size != 7 {
        return Err(CliError::MergeDriver(format!(
            "unsupported marker size {marker_size}: weavr currently only supports the default marker size of 7"
        )));
    }

    // Run git merge-file to produce a 3-way merge
    let output = std::process::Command::new("git")
        .args(["merge-file", "--stdout"])
        .arg(&args.ours)
        .arg(&args.base)
        .arg(&args.theirs)
        .output()
        .map_err(|e| CliError::MergeDriver(format!("failed to run git merge-file: {e}")))?;

    let exit_code = output.status.code();

    let merged = String::from_utf8(output.stdout).map_err(|e| {
        CliError::MergeDriver(format!("git merge-file produced invalid UTF-8: {e}"))
    })?;

    match exit_code {
        Some(0) => {
            // Clean merge — no conflicts
            std::fs::write(output_path, &merged)?;
            Ok(0)
        }
        Some(1) => {
            // Exit code 1 means conflicts remain — resolve them
            let file_path = args.path.as_deref().unwrap_or(args.ours.as_path());
            resolve_conflicts(
                &merged,
                output_path,
                file_path,
                strategy,
                config.deduplicate,
            )
        }
        other => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(CliError::MergeDriver(format!(
                "git merge-file failed (exit code {}): {}",
                other.map_or("unknown".to_string(), |c| c.to_string()),
                stderr.trim()
            )))
        }
    }
}

/// Determines the strategy to use, in priority order:
/// CLI `--strategy` > `WEAVR_MERGE_STRATEGY` env var > config default
fn resolve_strategy(cli_strategy: Option<Strategy>, config: &WeavrConfig) -> Strategy {
    if let Some(s) = cli_strategy {
        return s;
    }

    if let Ok(env_val) = std::env::var("WEAVR_MERGE_STRATEGY") {
        match env_val.to_lowercase().as_str() {
            "left" => return Strategy::Left,
            "right" => return Strategy::Right,
            "both" => return Strategy::Both,
            "ast" => return Strategy::Ast,
            _ => {} // fall through to config default
        }
    }

    config.default_strategy
}

/// Parses conflicted content, applies the strategy, and writes the result.
fn resolve_conflicts(
    content: &str,
    ours_path: &Path,
    file_path: &Path,
    strategy: Strategy,
    deduplicate: bool,
) -> Result<i32, CliError> {
    let display_path = file_path.to_path_buf();

    let mut session = weavr_core::MergeSession::from_conflicted(content, display_path)?;
    let hunks: Vec<_> = session.hunks().to_vec();

    if hunks.is_empty() {
        // No conflict markers found — write as-is
        std::fs::write(ours_path, content)?;
        return Ok(0);
    }

    for hunk in &hunks {
        let resolution = match strategy {
            Strategy::Left => weavr_core::Resolution::accept_left(hunk),
            Strategy::Right => weavr_core::Resolution::accept_right(hunk),
            Strategy::Both => {
                let options = weavr_core::AcceptBothOptions {
                    order: weavr_core::BothOrder::LeftThenRight,
                    deduplicate,
                    trim_whitespace: false,
                };
                weavr_core::Resolution::accept_both(hunk, &options)
            }
            Strategy::Ast => {
                // AST not available in merge driver context — fall back to left
                weavr_core::Resolution::accept_left(hunk)
            }
        };

        session.set_resolution(hunk.id, resolution)?;
    }

    session.apply()?;
    session.validate()?;
    let result = session.complete()?;

    std::fs::write(ours_path, &result.content)?;
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFLICTED: &str = "\
before
<<<<<<< ours
left content
=======
right content
>>>>>>> theirs
after
";

    #[test]
    fn resolve_conflicts_with_left_strategy() {
        let dir = tempfile::tempdir().unwrap();
        let ours = dir.path().join("ours.txt");
        std::fs::write(&ours, "placeholder").unwrap();

        let code = resolve_conflicts(CONFLICTED, &ours, &ours, Strategy::Left, false).unwrap();

        assert_eq!(code, 0);
        let result = std::fs::read_to_string(&ours).unwrap();
        assert!(result.contains("left content"));
        assert!(!result.contains("right content"));
        assert!(!result.contains("<<<<<<<"));
    }

    #[test]
    fn resolve_conflicts_with_right_strategy() {
        let dir = tempfile::tempdir().unwrap();
        let ours = dir.path().join("ours.txt");
        std::fs::write(&ours, "placeholder").unwrap();

        let code = resolve_conflicts(CONFLICTED, &ours, &ours, Strategy::Right, false).unwrap();

        assert_eq!(code, 0);
        let result = std::fs::read_to_string(&ours).unwrap();
        assert!(result.contains("right content"));
        assert!(!result.contains("left content"));
    }

    #[test]
    fn resolve_conflicts_with_both_strategy() {
        let dir = tempfile::tempdir().unwrap();
        let ours = dir.path().join("ours.txt");
        std::fs::write(&ours, "placeholder").unwrap();

        let code = resolve_conflicts(CONFLICTED, &ours, &ours, Strategy::Both, false).unwrap();

        assert_eq!(code, 0);
        let result = std::fs::read_to_string(&ours).unwrap();
        assert!(result.contains("left content"));
        assert!(result.contains("right content"));
    }

    #[test]
    fn resolve_conflicts_clean_content() {
        let dir = tempfile::tempdir().unwrap();
        let ours = dir.path().join("ours.txt");
        std::fs::write(&ours, "placeholder").unwrap();

        let clean = "no conflicts here\n";
        let code = resolve_conflicts(clean, &ours, &ours, Strategy::Left, false).unwrap();

        assert_eq!(code, 0);
        let result = std::fs::read_to_string(&ours).unwrap();
        assert_eq!(result, clean);
    }

    #[test]
    fn resolve_strategy_cli_wins() {
        let config = test_config(Strategy::Right);
        let result = resolve_strategy(Some(Strategy::Left), &config);
        assert_eq!(result, Strategy::Left);
    }

    #[test]
    fn resolve_strategy_falls_back_to_config() {
        let config = test_config(Strategy::Right);
        // Clear env to avoid interference
        std::env::remove_var("WEAVR_MERGE_STRATEGY");
        let result = resolve_strategy(None, &config);
        assert_eq!(result, Strategy::Right);
    }

    fn test_config(strategy: Strategy) -> WeavrConfig {
        WeavrConfig::from_raw(&crate::config::RawConfig {
            strategies: Some(crate::config::RawStrategiesConfig {
                default: Some(
                    match strategy {
                        Strategy::Left => "left",
                        Strategy::Right => "right",
                        Strategy::Both => "both",
                        Strategy::Ast => "ast",
                    }
                    .into(),
                ),
                deduplicate: None,
            }),
            ..crate::config::RawConfig::default()
        })
        .unwrap()
    }
}
