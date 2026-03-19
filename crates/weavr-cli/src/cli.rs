//! CLI argument definitions.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

/// Output format selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum OutputFormat {
    /// Human-readable text output
    #[default]
    Text,
    /// Machine-readable JSON output
    Json,
}

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
    /// AI-assisted resolution (requires the `ai` feature)
    Ai,
}

/// Fallback strategy for AI resolution failures.
///
/// Unlike [`Strategy`], this only includes strategies that make sense as
/// fallbacks — `ast` and `ai` are excluded because falling back to the
/// primary strategy (or another non-deterministic strategy) is not useful.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum FallbackStrategy {
    /// Accept left (`ours/HEAD`) content
    Left,
    /// Accept right (`theirs/MERGE_HEAD`) content
    Right,
    /// Accept both sides (combine left then right)
    Both,
}

/// Subcommands for weavr.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Run as a git merge driver
    MergeDriver(MergeDriverArgs),
    /// Initialize weavr in the current repository
    Init(InitArgs),
    /// Dump structured conflict data (JSON)
    Inspect(InspectArgs),
    /// Apply per-hunk resolutions from a JSON map
    Resolve(ResolveArgs),
}

/// Arguments for the `resolve` subcommand.
#[derive(Debug, Args)]
#[allow(clippy::struct_excessive_bools)] // CLI flags are naturally boolean
pub struct ResolveArgs {
    /// Files to resolve conflicts in
    #[arg(required = true, value_name = "FILE")]
    pub files: Vec<PathBuf>,

    /// Path to JSON resolutions file, or `-` for stdin
    #[arg(long, required = true, value_name = "PATH")]
    pub resolutions: PathBuf,

    /// Preview resolved output without writing files
    #[arg(long)]
    pub dry_run: bool,

    /// Error if any hunk has no resolution in the map
    #[arg(long)]
    pub fail_on_ambiguous: bool,

    /// Enable deduplication for `both` strategy entries
    #[arg(long)]
    pub dedupe: bool,

    /// Automatically stage resolved files
    #[arg(long)]
    pub auto_stage: bool,

    /// VCS backend to use
    #[arg(long, value_enum, default_value_t = VcsChoice::Auto)]
    pub vcs: VcsChoice,

    /// Output format
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
}

/// Arguments for the `inspect` subcommand.
#[derive(Debug, Args)]
pub struct InspectArgs {
    /// Files to inspect for conflicts
    #[arg(required = true, value_name = "FILE")]
    pub files: Vec<PathBuf>,
}

/// Scope for jj configuration.
#[cfg(feature = "jj")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum JjScope {
    /// Repository-local config
    #[default]
    Repo,
    /// User-level config
    User,
}

/// Arguments for the `init` subcommand.
#[derive(Debug, Args)]
#[allow(clippy::struct_excessive_bools)] // CLI flags are naturally boolean
pub struct InitArgs {
    /// Overwrite existing config files
    #[arg(long)]
    pub force: bool,
    /// Skip git merge driver and .gitattributes setup
    #[arg(long)]
    pub no_git: bool,
    /// Configure merge driver globally (~/.gitconfig) instead of repo-local
    #[arg(long, conflicts_with = "no_git")]
    pub global: bool,
    /// File patterns for .gitattributes (comma-separated, e.g. "*.rs,*.ts")
    #[arg(long, default_value = "*.rs", value_delimiter = ',')]
    pub patterns: Vec<String>,
    /// Skip jj merge tool setup
    #[cfg(feature = "jj")]
    #[arg(long)]
    pub no_jj: bool,
    /// Scope for jj config (repo or user)
    #[cfg(feature = "jj")]
    #[arg(long, value_enum, default_value_t = JjScope::Repo, conflicts_with = "no_jj")]
    pub jj_scope: JjScope,
}

/// Arguments for the `merge-driver` subcommand.
#[derive(Debug, Args)]
pub struct MergeDriverArgs {
    /// Base (ancestor) version (%O)
    pub base: PathBuf,
    /// Current (ours) version (%A) — result is written here unless --output is set
    pub ours: PathBuf,
    /// Other (theirs) version (%B)
    pub theirs: PathBuf,
    /// Conflict marker size (%L)
    pub marker_size: Option<usize>,
    /// Pathname of the merged file (%P)
    pub path: Option<PathBuf>,
    /// Resolution strategy (overrides config)
    #[arg(long, value_enum)]
    pub strategy: Option<Strategy>,
    /// Write result to a separate output file instead of overwriting ours
    #[arg(long)]
    pub output: Option<PathBuf>,
    /// Fallback strategy when AI declines or errors (defaults to left when omitted)
    #[arg(long, value_enum)]
    pub fallback_strategy: Option<FallbackStrategy>,
}

/// A terminal-first merge conflict resolver
#[derive(Parser, Debug)]
#[command(name = "weavr")]
#[command(author, version, about, long_about = None)]
#[allow(clippy::struct_excessive_bools)] // CLI flags are naturally boolean
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

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

    /// Fallback strategy when AI declines or errors (requires --headless)
    #[arg(long, value_enum, requires = "headless")]
    pub fallback_strategy: Option<FallbackStrategy>,

    /// Exit with code 1 if any hunk cannot be auto-resolved
    #[arg(long, requires = "headless")]
    pub fail_on_ambiguous: bool,

    /// List conflicted files and exit
    #[arg(long)]
    pub list: bool,

    /// Check files for conflicts and exit (no resolution)
    #[arg(long, conflicts_with_all = ["headless", "list"])]
    pub check: bool,

    /// Suppress output (exit code only); requires --check
    #[arg(long, requires = "check")]
    pub quiet: bool,

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

    /// Output format (text or json)
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
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
        assert!(cli.fallback_strategy.is_none());
        assert!(!cli.dedupe);
        assert!(!cli.dry_run);
        assert!(!cli.fail_on_ambiguous);
        assert!(!cli.list);
        assert!(!cli.check);
        assert!(!cli.quiet);
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
    fn cli_parse_strategy_ai() {
        let cli = Cli::parse_from(["weavr", "--headless", "--strategy=ai"]);
        assert!(cli.headless);
        assert_eq!(cli.strategy, Some(Strategy::Ai));
    }

    #[test]
    fn cli_parse_fallback_strategy() {
        let cli = Cli::parse_from([
            "weavr",
            "--headless",
            "--strategy=ai",
            "--fallback-strategy=left",
        ]);
        assert_eq!(cli.strategy, Some(Strategy::Ai));
        assert_eq!(cli.fallback_strategy, Some(FallbackStrategy::Left));
    }

    #[test]
    fn cli_fallback_strategy_requires_headless() {
        let result = Cli::try_parse_from(["weavr", "--fallback-strategy=left"]);
        assert!(result.is_err());
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

    #[test]
    fn cli_parse_check() {
        let cli = Cli::parse_from(["weavr", "--check", "file.rs"]);
        assert!(cli.check);
        assert!(!cli.quiet);
    }

    #[test]
    fn cli_parse_check_quiet() {
        let cli = Cli::parse_from(["weavr", "--check", "--quiet", "file.rs"]);
        assert!(cli.check);
        assert!(cli.quiet);
    }

    #[test]
    fn cli_check_conflicts_with_headless() {
        let result = Cli::try_parse_from(["weavr", "--check", "--headless"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_check_conflicts_with_list() {
        let result = Cli::try_parse_from(["weavr", "--check", "--list"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_quiet_requires_check() {
        let result = Cli::try_parse_from(["weavr", "--quiet"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_parse_init() {
        let cli = Cli::parse_from(["weavr", "init"]);
        assert!(matches!(cli.command, Some(Command::Init(ref args))
            if !args.force && !args.no_git && !args.global && args.patterns == vec!["*.rs"]));
    }

    #[test]
    fn cli_parse_init_force() {
        let cli = Cli::parse_from(["weavr", "init", "--force"]);
        if let Some(Command::Init(args)) = cli.command {
            assert!(args.force);
        } else {
            panic!("expected Init command");
        }
    }

    #[test]
    fn cli_parse_init_patterns() {
        let cli = Cli::parse_from(["weavr", "init", "--patterns", "*.rs,*.ts,*.go"]);
        if let Some(Command::Init(args)) = cli.command {
            assert_eq!(args.patterns, vec!["*.rs", "*.ts", "*.go"]);
        } else {
            panic!("expected Init command");
        }
    }

    #[test]
    fn cli_parse_init_global_conflicts_no_git() {
        let result = Cli::try_parse_from(["weavr", "init", "--global", "--no-git"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_parse_merge_driver_output() {
        let cli = Cli::parse_from([
            "weavr",
            "merge-driver",
            "base.txt",
            "ours.txt",
            "theirs.txt",
            "--output",
            "out.txt",
        ]);
        if let Some(Command::MergeDriver(args)) = cli.command {
            assert_eq!(args.output, Some(PathBuf::from("out.txt")));
        } else {
            panic!("expected MergeDriver command");
        }
    }

    #[test]
    fn cli_parse_merge_driver_no_output() {
        let cli = Cli::parse_from([
            "weavr",
            "merge-driver",
            "base.txt",
            "ours.txt",
            "theirs.txt",
        ]);
        if let Some(Command::MergeDriver(args)) = cli.command {
            assert!(args.output.is_none());
        } else {
            panic!("expected MergeDriver command");
        }
    }

    #[cfg(feature = "jj")]
    #[test]
    fn cli_parse_init_no_jj() {
        let cli = Cli::parse_from(["weavr", "init", "--no-jj"]);
        if let Some(Command::Init(args)) = cli.command {
            assert!(args.no_jj);
        } else {
            panic!("expected Init command");
        }
    }

    #[cfg(feature = "jj")]
    #[test]
    fn cli_parse_init_jj_scope_user() {
        let cli = Cli::parse_from(["weavr", "init", "--jj-scope", "user"]);
        if let Some(Command::Init(args)) = cli.command {
            assert_eq!(args.jj_scope, JjScope::User);
        } else {
            panic!("expected Init command");
        }
    }

    #[cfg(feature = "jj")]
    #[test]
    fn cli_parse_init_jj_scope_defaults_to_repo() {
        let cli = Cli::parse_from(["weavr", "init"]);
        if let Some(Command::Init(args)) = cli.command {
            assert_eq!(args.jj_scope, JjScope::Repo);
        } else {
            panic!("expected Init command");
        }
    }

    #[cfg(feature = "jj")]
    #[test]
    fn cli_parse_init_jj_scope_conflicts_with_no_jj() {
        let result = Cli::try_parse_from(["weavr", "init", "--no-jj", "--jj-scope", "user"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_parse_inspect() {
        let cli = Cli::parse_from(["weavr", "inspect", "file.rs"]);
        if let Some(Command::Inspect(args)) = cli.command {
            assert_eq!(args.files, vec![PathBuf::from("file.rs")]);
        } else {
            panic!("expected Inspect command");
        }
    }

    #[test]
    fn cli_parse_inspect_multiple_files() {
        let cli = Cli::parse_from(["weavr", "inspect", "a.rs", "b.rs"]);
        if let Some(Command::Inspect(args)) = cli.command {
            assert_eq!(args.files.len(), 2);
        } else {
            panic!("expected Inspect command");
        }
    }

    #[test]
    fn cli_parse_inspect_requires_file() {
        let result = Cli::try_parse_from(["weavr", "inspect"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_parse_format_json() {
        let cli = Cli::parse_from(["weavr", "--format=json", "--check", "file.rs"]);
        assert_eq!(cli.format, OutputFormat::Json);
    }

    #[test]
    fn cli_parse_format_text() {
        let cli = Cli::parse_from(["weavr", "--format=text", "--check", "file.rs"]);
        assert_eq!(cli.format, OutputFormat::Text);
    }

    #[test]
    fn cli_parse_format_default_is_text() {
        let cli = Cli::parse_from(["weavr"]);
        assert_eq!(cli.format, OutputFormat::Text);
    }

    #[test]
    fn cli_parse_resolve() {
        let cli = Cli::parse_from([
            "weavr",
            "resolve",
            "file.rs",
            "--resolutions",
            "decisions.json",
        ]);
        if let Some(Command::Resolve(args)) = cli.command {
            assert_eq!(args.files, vec![PathBuf::from("file.rs")]);
            assert_eq!(args.resolutions, PathBuf::from("decisions.json"));
            assert!(!args.dry_run);
            assert!(!args.fail_on_ambiguous);
            assert!(!args.dedupe);
        } else {
            panic!("expected Resolve command");
        }
    }

    #[test]
    fn cli_parse_resolve_requires_resolutions() {
        let result = Cli::try_parse_from(["weavr", "resolve", "file.rs"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_parse_resolve_requires_file() {
        let result = Cli::try_parse_from(["weavr", "resolve", "--resolutions", "r.json"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_parse_resolve_dry_run() {
        let cli = Cli::parse_from([
            "weavr",
            "resolve",
            "file.rs",
            "--resolutions",
            "r.json",
            "--dry-run",
        ]);
        if let Some(Command::Resolve(args)) = cli.command {
            assert!(args.dry_run);
        } else {
            panic!("expected Resolve command");
        }
    }

    #[test]
    fn cli_parse_resolve_fail_on_ambiguous() {
        let cli = Cli::parse_from([
            "weavr",
            "resolve",
            "file.rs",
            "--resolutions",
            "r.json",
            "--fail-on-ambiguous",
        ]);
        if let Some(Command::Resolve(args)) = cli.command {
            assert!(args.fail_on_ambiguous);
        } else {
            panic!("expected Resolve command");
        }
    }

    #[test]
    fn cli_parse_resolve_format_json() {
        let cli = Cli::parse_from([
            "weavr",
            "resolve",
            "file.rs",
            "--resolutions",
            "r.json",
            "--format=json",
        ]);
        if let Some(Command::Resolve(args)) = cli.command {
            assert_eq!(args.format, OutputFormat::Json);
        } else {
            panic!("expected Resolve command");
        }
    }

    #[test]
    fn cli_parse_resolve_stdin() {
        let cli = Cli::parse_from(["weavr", "resolve", "file.rs", "--resolutions", "-"]);
        if let Some(Command::Resolve(args)) = cli.command {
            assert_eq!(args.resolutions, PathBuf::from("-"));
        } else {
            panic!("expected Resolve command");
        }
    }

    #[test]
    fn cli_parse_resolve_multiple_files() {
        let cli = Cli::parse_from([
            "weavr",
            "resolve",
            "a.rs",
            "b.rs",
            "--resolutions",
            "r.json",
        ]);
        if let Some(Command::Resolve(args)) = cli.command {
            assert_eq!(args.files.len(), 2);
        } else {
            panic!("expected Resolve command");
        }
    }
}
