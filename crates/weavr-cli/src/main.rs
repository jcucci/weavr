//! weavr CLI - Command-line interface for merge conflict resolution
//!
//! This binary provides:
//! - Interactive mode (launches TUI)
//! - Headless mode (applies rules automatically)
//! - File discovery and orchestration

#![forbid(unsafe_code)]

mod cli;
mod discovery;
mod error;
mod headless;

use clap::Parser;

use cli::{Cli, Strategy};
use error::{exit_codes, CliError};

fn run(cli: &Cli) -> Result<i32, CliError> {
    // Mode: List conflicted files
    if cli.list {
        discovery::list_conflicted_files()?;
        return Ok(exit_codes::SUCCESS);
    }

    // Resolve which files to process
    let files = discovery::resolve_files(cli.files.clone())?;

    // Mode: Headless
    if cli.headless {
        let strategy = cli.strategy.unwrap_or(Strategy::Left);

        for path in &files {
            let result = headless::process_file(path, strategy, cli.dedupe)?;
            headless::write_or_print(&result, cli.dry_run)?;
        }

        return Ok(exit_codes::SUCCESS);
    }

    // Mode: Interactive (TUI)
    println!("weavr: interactive mode");
    println!("Files to resolve:");
    for file in &files {
        println!("  {}", file.display());
    }
    println!("\nTUI integration pending (see weavr-tui crate)");

    Ok(exit_codes::SUCCESS)
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
