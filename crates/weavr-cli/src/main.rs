//! weavr CLI - Command-line interface for merge conflict resolution
//!
//! This binary provides:
//! - Interactive mode (launches TUI)
//! - Headless mode (applies rules automatically)
//! - File discovery and orchestration

#![forbid(unsafe_code)]

mod check;
mod cli;
mod config;
mod discovery;
mod error;
mod headless;
mod init;
mod inspect;
mod merge_driver;
mod output;
mod resolve;
mod tui;

use clap::Parser;

use cli::{Cli, OutputFormat};
use config::WeavrConfig;
use error::{exit_codes, CliError};

/// Runs `jj squash` in the repo root when squash-after-resolve is enabled.
#[cfg(feature = "jj")]
fn run_jj_squash(backend: &dyn weavr_vcs::VcsBackend) {
    run_jj_squash_impl(backend, false);
}

/// Runs `jj squash` without printing success messages (for JSON mode).
#[cfg(feature = "jj")]
fn run_jj_squash_quiet(backend: &dyn weavr_vcs::VcsBackend) {
    run_jj_squash_impl(backend, true);
}

#[cfg(feature = "jj")]
fn run_jj_squash_impl(backend: &dyn weavr_vcs::VcsBackend, quiet: bool) {
    if backend.name() != "jj" {
        return;
    }
    let root = backend.root();
    match std::process::Command::new("jj")
        .arg("squash")
        .current_dir(root)
        .output()
    {
        Ok(output) if output.status.success() => {
            if !quiet {
                println!("jj: squashed resolved changes");
            }
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("jj squash failed: {stderr}");
        }
        Err(e) => {
            eprintln!("jj squash failed: {e}");
        }
    }
}

#[allow(clippy::too_many_lines)]
fn run(cli: &Cli) -> Result<i32, CliError> {
    let format = cli.format;

    // Handle subcommands first
    if let Some(ref command) = cli.command {
        match command {
            cli::Command::MergeDriver(args) => {
                let raw_config = config::load_config(cli.config.as_deref())?;
                let cfg = WeavrConfig::from_raw(&raw_config)?;
                return merge_driver::run(args, &cfg, format);
            }
            cli::Command::Init(args) => {
                return init::run(args);
            }
            cli::Command::Inspect(args) => {
                return inspect::run(args);
            }
            cli::Command::Resolve(args) => {
                return resolve::run(args);
            }
        }
    }

    // Reject --format=json in TUI mode (no --list, --check, or --headless)
    if format == OutputFormat::Json && !cli.list && !cli.check && !cli.headless {
        return Err(CliError::InvalidArgs(
            "--format=json is not supported in TUI mode; use --list, --check, or --headless"
                .to_string(),
        ));
    }

    let backend = discovery::discover_backend(cli.vcs);

    // Mode: List conflicted files
    if cli.list {
        let backend = backend
            .as_deref()
            .ok_or(CliError::Vcs(weavr_vcs::VcsError::NotInRepo))?;
        discovery::list_conflicted_files(backend, format)?;
        return Ok(exit_codes::SUCCESS);
    }

    // Mode: Check for conflicts
    if cli.check {
        let files = if cli.files.is_empty() {
            let backend = backend
                .as_deref()
                .ok_or(CliError::Vcs(weavr_vcs::VcsError::NotInRepo))?;
            discovery::discover_conflicted_files(backend)?
        } else {
            cli.files.clone()
        };

        let results: Vec<check::CheckResult> = files
            .iter()
            .map(|p| check::check_file(p))
            .collect::<Result<_, _>>()?;
        let has_conflicts = results.iter().any(|r| r.conflict_count > 0);

        if !cli.quiet {
            if results.is_empty() {
                match format {
                    OutputFormat::Json => {
                        let json = output::JsonCheckOutput {
                            files: vec![],
                            total_conflicts: 0,
                        };
                        output::print_json(&json)?;
                    }
                    OutputFormat::Text => println!("No conflicted files found"),
                }
            } else {
                check::print_summary(&results, format)?;
            }
        }

        return Ok(if has_conflicts {
            exit_codes::UNRESOLVED
        } else {
            exit_codes::SUCCESS
        });
    }

    // Load and resolve configuration (layers 1-4)
    let raw_config = config::load_config(cli.config.as_deref())?;
    let mut config = WeavrConfig::from_raw(&raw_config)?;

    // Layer 5: CLI flag overrides
    if let Some(ref theme_name) = cli.theme {
        config.theme = config::parse_theme_name(theme_name)?;
    }
    if let Some(strategy) = cli.strategy {
        config.default_strategy = strategy;
    }
    if cli.dedupe {
        config.deduplicate = true;
    }
    if cli.fail_on_ambiguous {
        config.fail_on_ambiguous = true;
    }
    if cli.auto_stage {
        config.auto_stage = true;
    }
    if cli.no_stage {
        config.auto_stage = false;
        config.stage_prompt = false;
    }

    if backend.is_none() && (config.auto_stage || config.stage_prompt) {
        eprintln!("weavr: VCS backend not found, staging disabled");
    }

    // Resolve which files to process
    let files = discovery::resolve_files(cli.files.clone(), backend.as_deref())?;

    // Mode: Headless
    if cli.headless {
        let strategy = config.default_strategy;

        // Build AST handle for headless mode
        #[cfg(feature = "ast")]
        let ast_strategy;
        #[cfg(feature = "ast")]
        let ast_handle = {
            ast_strategy = tui::build_ast_strategy(&config.ast);
            headless::AstHandle::some(&ast_strategy)
        };
        #[cfg(not(feature = "ast"))]
        let ast_handle = headless::AstHandle::none();

        let is_json = format == OutputFormat::Json;
        let mut json_results: Vec<output::JsonHeadlessFile> = Vec::new();

        let strategy_name = match strategy {
            cli::Strategy::Left => "left",
            cli::Strategy::Right => "right",
            cli::Strategy::Both => "both",
            cli::Strategy::Ast => "ast",
        };

        for path in &files {
            let result = headless::process_file(path, strategy, config.deduplicate, &ast_handle)?;
            let written = !cli.dry_run;

            if is_json {
                json_results.push(output::JsonHeadlessFile {
                    path: path.clone(),
                    hunks_resolved: result.hunks_resolved,
                    strategy: strategy_name.to_string(),
                    written,
                });
                // Still write the file (or print for dry-run), just suppress text output
                if written {
                    std::fs::write(&result.path, &result.output)?;
                }
            } else {
                headless::write_or_print(&result, cli.dry_run)?;
            }

            if config.auto_stage && !cli.dry_run {
                if let Some(ref backend) = backend {
                    match backend.stage_file(path) {
                        Ok(()) => {
                            if !is_json {
                                println!("{}: staged", path.display());
                            }
                        }
                        Err(e) => eprintln!("{}: staging failed: {e}", path.display()),
                    }
                }
            }
        }

        if is_json {
            output::print_json(&output::JsonHeadlessOutput {
                results: json_results,
            })?;
        }

        #[cfg(feature = "jj")]
        if config.squash_after_resolve {
            if let Some(ref backend) = backend {
                if is_json {
                    // Still squash, but suppress stdout message
                    run_jj_squash_quiet(backend.as_ref());
                } else {
                    run_jj_squash(backend.as_ref());
                }
            }
        }

        return Ok(exit_codes::SUCCESS);
    }

    // Mode: Interactive (TUI)
    let results = tui::process_files(&files, &config, raw_config.keybindings.as_ref())?;
    let mut any_unresolved = false;

    for result in &results {
        if let Some(ref content) = result.content {
            std::fs::write(&result.path, content)?;

            if result.is_partial {
                // Partial write — conflict markers remain
                let remaining = result.total_hunks - result.hunks_resolved;
                println!(
                    "{}: saved with {} unresolved hunks (markers preserved)",
                    result.path.display(),
                    remaining
                );
                any_unresolved = true;
                // Do NOT auto-stage — file still has conflict markers
            } else {
                println!(
                    "{}: {} hunks resolved",
                    result.path.display(),
                    result.hunks_resolved
                );

                let should_stage = config.auto_stage || result.stage_requested;
                if should_stage {
                    if let Some(ref backend) = backend {
                        match backend.stage_file(&result.path) {
                            Ok(()) => println!("{}: staged", result.path.display()),
                            Err(e) => eprintln!("{}: staging failed: {e}", result.path.display()),
                        }
                    } else if result.stage_requested {
                        eprintln!(
                            "{}: staging requested but VCS backend not available",
                            result.path.display()
                        );
                    }
                }
            }
        } else {
            any_unresolved = true;
            eprintln!(
                "{}: exited with {}/{} hunks unresolved",
                result.path.display(),
                result.total_hunks - result.hunks_resolved,
                result.total_hunks
            );
        }
    }

    if any_unresolved {
        Ok(exit_codes::UNRESOLVED)
    } else {
        #[cfg(feature = "jj")]
        if config.squash_after_resolve {
            if let Some(ref backend) = backend {
                run_jj_squash(backend.as_ref());
            }
        }
        Ok(exit_codes::SUCCESS)
    }
}

fn main() {
    let cli = Cli::parse();

    let exit_code = match run(&cli) {
        Ok(code) => code,
        Err(e) => {
            if cli.format == OutputFormat::Json {
                output::print_json_error(&e.to_string());
            } else {
                eprintln!("weavr: {e}");
            }
            e.exit_code()
        }
    };

    std::process::exit(exit_code);
}
