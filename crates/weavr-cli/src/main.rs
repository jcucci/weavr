//! weavr CLI - Command-line interface for merge conflict resolution
//!
//! This binary provides:
//! - Interactive mode (launches TUI)
//! - Headless mode (applies rules automatically)
//! - File discovery and orchestration

#![forbid(unsafe_code)]

mod cli;
mod config;
mod discovery;
mod error;
mod headless;
mod tui;

use clap::Parser;

use cli::Cli;
use config::WeavrConfig;
use error::{exit_codes, CliError};

fn run(cli: &Cli) -> Result<i32, CliError> {
    // Mode: List conflicted files
    if cli.list {
        discovery::list_conflicted_files()?;
        return Ok(exit_codes::SUCCESS);
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

    // Resolve which files to process
    let files = discovery::resolve_files(cli.files.clone())?;

    // Git repo discovery (needed for auto-staging, prompts, and explicit :wa)
    let repo = match weavr_git::GitRepo::discover() {
        Ok(r) => Some(r),
        Err(e) => {
            if config.auto_stage || config.stage_prompt {
                eprintln!("weavr: git repo not found, staging disabled: {e}");
            }
            None
        }
    };

    // Mode: Headless
    if cli.headless {
        let strategy = config.default_strategy;

        for path in &files {
            let result = headless::process_file(path, strategy, config.deduplicate)?;
            headless::write_or_print(&result, cli.dry_run)?;

            if config.auto_stage && !cli.dry_run {
                if let Some(ref repo) = repo {
                    match repo.stage_file(path) {
                        Ok(()) => println!("{}: staged", path.display()),
                        Err(e) => eprintln!("{}: staging failed: {e}", path.display()),
                    }
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
            println!(
                "{}: {} hunks resolved",
                result.path.display(),
                result.hunks_resolved
            );

            let should_stage = config.auto_stage || result.stage_requested;
            if should_stage {
                if let Some(ref repo) = repo {
                    match repo.stage_file(&result.path) {
                        Ok(()) => println!("{}: staged", result.path.display()),
                        Err(e) => eprintln!("{}: staging failed: {e}", result.path.display()),
                    }
                } else if result.stage_requested {
                    eprintln!(
                        "{}: staging requested but git repo not available",
                        result.path.display()
                    );
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
        Ok(exit_codes::SUCCESS)
    }
}

fn main() {
    let cli = Cli::parse();

    let exit_code = match run(&cli) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("weavr: {e}");
            e.exit_code()
        }
    };

    std::process::exit(exit_code);
}
