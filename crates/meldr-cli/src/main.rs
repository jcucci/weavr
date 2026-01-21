//! meldr CLI - Command-line interface for merge conflict resolution
//!
//! This binary provides:
//! - Interactive mode (launches TUI)
//! - Headless mode (applies rules automatically)
//! - File discovery and orchestration

#![forbid(unsafe_code)]

use std::path::PathBuf;

use clap::Parser;

/// A terminal-first merge conflict resolver
#[derive(Parser, Debug)]
#[command(name = "meldr")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Files to resolve (defaults to all conflicted files)
    #[arg(value_name = "FILE")]
    files: Vec<PathBuf>,

    /// Run in headless mode (no TUI, apply rules automatically)
    #[arg(long)]
    headless: bool,

    /// Configuration file path
    #[arg(long, value_name = "PATH")]
    config: Option<PathBuf>,
}

fn main() {
    let cli = Cli::parse();

    if cli.headless {
        println!("meldr: headless mode (not yet implemented)");
    } else {
        println!("meldr: interactive mode (not yet implemented)");
    }

    if !cli.files.is_empty() {
        println!("Files: {:?}", cli.files);
    }

    // Demonstrate that meldr-core is linked correctly
    let input = meldr_core::MergeInput {
        left: meldr_core::FileVersion {
            path: PathBuf::from("example.rs"),
            content: String::from("left"),
        },
        right: meldr_core::FileVersion {
            path: PathBuf::from("example.rs"),
            content: String::from("right"),
        },
        base: None,
    };

    match meldr_core::MergeSession::new(input) {
        Ok(session) => {
            println!("Session state: {:?}", session.state());
        }
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}
