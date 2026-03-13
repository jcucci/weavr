//! CLI argument definitions.

use std::path::PathBuf;

use clap::{Parser, ValueEnum};

/// VCS backend selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum VcsChoice {
    /// Auto-detect (tries jj first, then git)
    #[default]
    Auto,
    /// Force Git backend
    Git,
    /// Force Jujutsu (jj) backend
    Jj,
}

/// Resolution strategy for headless mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Strategy {
    /// Accept left (`ours/HEAD`) content
    Left,
    /// Accept right (`theirs/MERGE_HEAD`) content
    Right,
    /// Accept both sides (combine left then right)
    Both,
    /// AST-based structural merge (falls back to left when unavailable)
    Ast,
}

/// A terminal-first merge conflict resolver
#[derive(Parser, Debug)]
#[command(name = "weavr")]
#[command(author, version, about, long_about = None)]
#[allow(clippy::struct_excessive_bools)] // CLI flags are naturally boolean
pub struct Cli {
    /// VCS backend to use (auto-detects by default)
    #[arg(long, value_enum, default_value_t = VcsChoice::Auto)]
    pub vcs: VcsChoice,

    /// Files to resolve (defaults to all conflicted files)
    #[arg(value_name = "FILE")]
    pub files: Vec<PathBuf>,

    /// Run in headless mode (no TUI, apply rules automatically)
    #[arg(long)]
    pub headless: bool,

    /// Default resolution strategy for headless mode
    #[arg(long, value_enum, requires = "headless")]
    pub strategy: Option<Strategy>,

    /// Enable deduplication for accept-both strategy
    #[arg(long, requires = "headless")]
    pub dedupe: bool,

    /// Print result without writing to file
    #[arg(long, requires = "headless")]
    pub dry_run: bool,

    /// Exit with code 1 if any hunk cannot be auto-resolved
    #[arg(long, requires = "headless")]
    pub fail_on_ambiguous: bool,

    /// List conflicted files and exit
    #[arg(long)]
    pub list: bool,

    /// Configuration file path
    #[arg(long, value_name = "PATH")]
    pub config: Option<PathBuf>,

    /// Theme name (overrides config file)
    #[arg(long, value_name = "THEME")]
    pub theme: Option<String>,

    /// Automatically stage resolved files after writing
    #[arg(long)]
    pub auto_stage: bool,

    /// Disable staging (no auto-stage, no prompt)
    #[arg(long, conflicts_with = "auto_stage")]
    pub no_stage: bool,
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
        assert_eq!(cli.vcs, VcsChoice::Auto);
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
        let cli = Cli::parse_from(["weavr", "--headless", "--strategy=left"]);
        assert!(cli.headless);
        assert_eq!(cli.strategy, Some(Strategy::Left));
    }

    #[test]
    fn cli_parse_strategy_right() {
        let cli = Cli::parse_from(["weavr", "--headless", "--strategy=right"]);
        assert!(cli.headless);
        assert_eq!(cli.strategy, Some(Strategy::Right));
    }

    #[test]
    fn cli_strategy_requires_headless() {
        let result = Cli::try_parse_from(["weavr", "--strategy=left"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_dedupe_requires_headless() {
        let result = Cli::try_parse_from(["weavr", "--dedupe"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_parse_theme() {
        let cli = Cli::parse_from(["weavr", "--theme", "dracula"]);
        assert_eq!(cli.theme.as_deref(), Some("dracula"));
    }

    #[test]
    fn cli_parse_theme_default_is_none() {
        let cli = Cli::parse_from(["weavr"]);
        assert!(cli.theme.is_none());
    }

    #[test]
    fn cli_parse_auto_stage() {
        let cli = Cli::parse_from(["weavr", "--auto-stage"]);
        assert!(cli.auto_stage);
        assert!(!cli.no_stage);
    }

    #[test]
    fn cli_parse_no_stage() {
        let cli = Cli::parse_from(["weavr", "--no-stage"]);
        assert!(cli.no_stage);
        assert!(!cli.auto_stage);
    }

    #[test]
    fn cli_auto_stage_conflicts_with_no_stage() {
        let result = Cli::try_parse_from(["weavr", "--auto-stage", "--no-stage"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_auto_stage_defaults() {
        let cli = Cli::parse_from(["weavr"]);
        assert!(!cli.auto_stage);
        assert!(!cli.no_stage);
    }

    #[test]
    fn cli_parse_vcs_git() {
        let cli = Cli::parse_from(["weavr", "--vcs", "git"]);
        assert_eq!(cli.vcs, VcsChoice::Git);
    }

    #[test]
    fn cli_parse_vcs_jj() {
        let cli = Cli::parse_from(["weavr", "--vcs", "jj"]);
        assert_eq!(cli.vcs, VcsChoice::Jj);
    }

    #[test]
    fn cli_parse_vcs_auto() {
        let cli = Cli::parse_from(["weavr", "--vcs", "auto"]);
        assert_eq!(cli.vcs, VcsChoice::Auto);
    }
}
