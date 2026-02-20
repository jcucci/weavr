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

    // Resolve which files to process
    let files = discovery::resolve_files(cli.files.clone())?;

    // Mode: Headless
    if cli.headless {
        let strategy = config.default_strategy;

        for path in &files {
            let result = headless::process_file(path, strategy, config.deduplicate)?;
            headless::write_or_print(&result, cli.dry_run)?;
        }

        return Ok(exit_codes::SUCCESS);
    }

    // Mode: Interactive (TUI)
    let mut any_unresolved = false;

    for path in &files {
        let result = tui::process_file(path, &config)?;

        if let Some(ref content) = result.content {
            std::fs::write(path, content)?;
            println!(
                "{}: {} hunks resolved",
                path.display(),
                result.hunks_resolved
            );
        } else {
            any_unresolved = true;
            eprintln!(
                "{}: exited with {}/{} hunks unresolved",
                path.display(),
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
