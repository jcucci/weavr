//! CLI argument definitions.

use std::path::PathBuf;

use clap::{Parser, ValueEnum};

/// Resolution strategy for headless mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Strategy {
    /// Accept left (ours/HEAD) content
    Left,
    /// Accept right (`theirs/MERGE_HEAD`) content
    Right,
    /// Accept both sides (combine left then right)
    Both,
}

/// A terminal-first merge conflict resolver
#[derive(Parser, Debug)]
#[command(name = "weavr")]
#[command(author, version, about, long_about = None)]
#[allow(clippy::struct_excessive_bools)] // CLI flags are naturally boolean
pub struct Cli {
    /// Files to resolve (defaults to all conflicted files)
    #[arg(value_name = "FILE")]
    pub files: Vec<PathBuf>,

    /// Run in headless mode (no TUI, apply rules automatically)
    #[arg(long)]
    pub headless: bool,

    /// Default resolution strategy for headless mode
    #[arg(long, value_enum)]
    pub strategy: Option<Strategy>,

    /// Enable deduplication for accept-both strategy
    #[arg(long)]
    pub dedupe: bool,

    /// Print result without writing to file
    #[arg(long)]
    pub dry_run: bool,

    /// Exit with code 1 if any hunk cannot be auto-resolved
    #[arg(long)]
    pub fail_on_ambiguous: bool,

    /// List conflicted files and exit
    #[arg(long)]
    pub list: bool,

    /// Configuration file path
    #[arg(long, value_name = "PATH")]
    pub config: Option<PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_parse_defaults() {
        let cli = Cli::parse_from(["weavr"]);
        assert!(cli.files.is_empty());
        assert!(!cli.headless);
        assert!(cli.strategy.is_none());
        assert!(!cli.dedupe);
        assert!(!cli.dry_run);
        assert!(!cli.fail_on_ambiguous);
        assert!(!cli.list);
    }

    #[test]
    fn cli_parse_headless_with_strategy() {
        let cli = Cli::parse_from(["weavr", "--headless", "--strategy=both", "--dedupe"]);
        assert!(cli.headless);
        assert_eq!(cli.strategy, Some(Strategy::Both));
        assert!(cli.dedupe);
    }

    #[test]
    fn cli_parse_files() {
        let cli = Cli::parse_from(["weavr", "file1.rs", "file2.rs"]);
        assert_eq!(cli.files.len(), 2);
    }

    #[test]
    fn cli_parse_list() {
        let cli = Cli::parse_from(["weavr", "--list"]);
        assert!(cli.list);
    }

    #[test]
    fn cli_parse_dry_run() {
        let cli = Cli::parse_from(["weavr", "--headless", "--dry-run"]);
        assert!(cli.headless);
        assert!(cli.dry_run);
    }

    #[test]
    fn cli_parse_fail_on_ambiguous() {
        let cli = Cli::parse_from(["weavr", "--headless", "--fail-on-ambiguous"]);
        assert!(cli.headless);
        assert!(cli.fail_on_ambiguous);
    }

    #[test]
    fn cli_parse_strategy_left() {
        let cli = Cli::parse_from(["weavr", "--strategy=left"]);
        assert_eq!(cli.strategy, Some(Strategy::Left));
    }

    #[test]
    fn cli_parse_strategy_right() {
        let cli = Cli::parse_from(["weavr", "--strategy=right"]);
        assert_eq!(cli.strategy, Some(Strategy::Right));
    }
}
