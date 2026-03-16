//! `weavr init` command — project initialization.
//!
//! Creates `.weavr.toml`, configures the git merge driver, and sets up
//! `.gitattributes` entries for weavr-managed file patterns.

use std::path::PathBuf;

use crate::cli::InitArgs;
use crate::error::{exit_codes, CliError};

/// Default `.weavr.toml` template with all options commented out.
const DEFAULT_CONFIG_TEMPLATE: &str = r#"# weavr configuration
# See: https://github.com/jcucci/weavr for full documentation
#
# All values shown below are defaults. Uncomment and modify as needed.

# [theme]
# name = "dark"           # Options: dark, light, nord, dracula

# [strategies]
# default = "left"        # Options: left, right, both, ast
# deduplicate = false

# [headless]
# fail_on_ambiguous = false

# [git]
# auto_stage = false
# stage_prompt = true

# [keybindings]
# next_hunk = ["j", "<Down>"]
# prev_hunk = ["k", "<Up>"]
# resolve_left = "a"
# resolve_right = "d"
# resolve_both = "b"
# quit = "q"
"#;

/// Tracks what actions were performed for the summary output.
struct Actions {
    config_created: bool,
    config_overwritten: bool,
    driver_configured: bool,
    patterns_added: Vec<String>,
}

impl Actions {
    fn new() -> Self {
        Self {
            config_created: false,
            config_overwritten: false,
            driver_configured: false,
            patterns_added: Vec::new(),
        }
    }

    fn did_something(&self) -> bool {
        self.config_created
            || self.config_overwritten
            || self.driver_configured
            || !self.patterns_added.is_empty()
    }
}

/// Entry point for `weavr init`.
pub fn run(args: &InitArgs) -> Result<i32, CliError> {
    let mut actions = Actions::new();

    write_config_file(args.force, &mut actions)?;

    if !args.no_git {
        setup_git_merge_driver(args.global, &mut actions)?;
        setup_gitattributes(&args.patterns, &mut actions)?;
    }

    if actions.did_something() {
        println!("weavr: initialized successfully");
        if actions.config_created {
            println!("  created .weavr.toml");
        }
        if actions.config_overwritten {
            println!("  overwrote .weavr.toml");
        }
        if actions.driver_configured {
            if args.global {
                println!("  configured merge driver in ~/.gitconfig");
            } else {
                println!("  configured merge driver in .git/config");
            }
        }
        for pattern in &actions.patterns_added {
            println!("  added {pattern} merge=weavr to .gitattributes");
        }
    } else {
        println!("weavr: nothing to do (already initialized)");
    }

    Ok(exit_codes::SUCCESS)
}

/// Writes `.weavr.toml` with documented defaults. Skips if it exists unless `--force`.
fn write_config_file(force: bool, actions: &mut Actions) -> Result<(), CliError> {
    let config_path = PathBuf::from(".weavr.toml");

    if config_path.exists() && !force {
        return Ok(());
    }

    let overwriting = config_path.exists();
    std::fs::write(&config_path, DEFAULT_CONFIG_TEMPLATE)
        .map_err(|e| CliError::Init(format!("failed to write .weavr.toml: {e}")))?;

    if overwriting {
        actions.config_overwritten = true;
    } else {
        actions.config_created = true;
    }

    Ok(())
}

/// Configures the git merge driver via `git config`.
fn setup_git_merge_driver(global: bool, actions: &mut Actions) -> Result<(), CliError> {
    let scope_flag: &[&str] = if global { &["--global"] } else { &[] };

    let configs = [
        ("merge.weavr.name", "weavr merge driver"),
        ("merge.weavr.driver", "weavr merge-driver %O %A %B %L %P"),
    ];

    // Check which keys need updating
    let mut needs_update = Vec::new();
    for (key, value) in &configs {
        let already_set = git_config_get(key, scope_flag).is_some_and(|current| current == *value);
        if !already_set {
            needs_update.push((*key, *value));
        }
    }

    if needs_update.is_empty() {
        return Ok(());
    }

    for (key, value) in &needs_update {
        let mut cmd = std::process::Command::new("git");
        cmd.arg("config");
        for flag in scope_flag {
            cmd.arg(flag);
        }
        cmd.args([key, value]);

        let output = cmd
            .output()
            .map_err(|e| CliError::Init(format!("failed to run git config: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CliError::Init(format!(
                "git config {key} failed: {}",
                stderr.trim()
            )));
        }
    }

    actions.driver_configured = true;
    Ok(())
}

/// Reads a git config value, returning `None` if not set.
fn git_config_get(key: &str, scope_flags: &[&str]) -> Option<String> {
    let mut cmd = std::process::Command::new("git");
    cmd.arg("config");
    for flag in scope_flags {
        cmd.arg(flag);
    }
    cmd.args(["--get", key]);

    let output = cmd.output().ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

/// Finds the repo root and appends missing `<pattern> merge=weavr` lines to `.gitattributes`.
fn setup_gitattributes(patterns: &[String], actions: &mut Actions) -> Result<(), CliError> {
    let repo_root = git_repo_root()?;
    let gitattributes_path = repo_root.join(".gitattributes");

    let existing_content = if gitattributes_path.exists() {
        std::fs::read_to_string(&gitattributes_path)
            .map_err(|e| CliError::Init(format!("failed to read .gitattributes: {e}")))?
    } else {
        String::new()
    };

    let mut lines_to_add = Vec::new();
    for pattern in patterns {
        let entry = format!("{pattern} merge=weavr");
        let already_present = existing_content.lines().any(|line| line.trim() == entry);

        if !already_present {
            lines_to_add.push(entry);
            actions.patterns_added.push(pattern.clone());
        }
    }

    if !lines_to_add.is_empty() {
        let mut content = existing_content;
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        for line in &lines_to_add {
            content.push_str(line);
            content.push('\n');
        }
        std::fs::write(&gitattributes_path, content)
            .map_err(|e| CliError::Init(format!("failed to write .gitattributes: {e}")))?;
    }

    Ok(())
}

/// Returns the git repository root via `git rev-parse --show-toplevel`.
fn git_repo_root() -> Result<PathBuf, CliError> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| CliError::Init(format!("failed to run git rev-parse: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(CliError::Init(format!(
            "not a git repository: {}",
            stderr.trim()
        )));
    }

    let root = String::from_utf8(output.stdout)
        .map_err(|e| CliError::Init(format!("invalid UTF-8 from git rev-parse: {e}")))?;

    Ok(PathBuf::from(root.trim()))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn config_file_created_in_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let mut actions = Actions::new();
        write_config_file(false, &mut actions).unwrap();

        std::env::set_current_dir(&prev).unwrap();

        assert!(actions.config_created);
        assert!(!actions.config_overwritten);
        let content = std::fs::read_to_string(dir.path().join(".weavr.toml")).unwrap();
        assert!(content.contains("[theme]"));
        assert!(content.contains("[strategies]"));
    }

    #[test]
    fn config_file_skipped_when_exists() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".weavr.toml"), "existing").unwrap();
        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let mut actions = Actions::new();
        write_config_file(false, &mut actions).unwrap();

        std::env::set_current_dir(&prev).unwrap();

        assert!(!actions.config_created);
        assert!(!actions.config_overwritten);
        let content = std::fs::read_to_string(dir.path().join(".weavr.toml")).unwrap();
        assert_eq!(content, "existing");
    }

    #[test]
    fn config_file_overwritten_with_force() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".weavr.toml"), "old").unwrap();
        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let mut actions = Actions::new();
        write_config_file(true, &mut actions).unwrap();

        std::env::set_current_dir(&prev).unwrap();

        assert!(!actions.config_created);
        assert!(actions.config_overwritten);
        let content = std::fs::read_to_string(dir.path().join(".weavr.toml")).unwrap();
        assert!(content.contains("[theme]"));
    }

    #[test]
    fn gitattributes_no_duplicates() {
        let dir = tempfile::tempdir().unwrap();
        let gitattributes = dir.path().join(".gitattributes");
        std::fs::write(&gitattributes, "*.rs merge=weavr\n").unwrap();

        let mut actions = Actions::new();
        setup_gitattributes_in_dir(
            &["*.rs".to_string(), "*.ts".to_string()],
            &mut actions,
            dir.path(),
        );

        assert_eq!(actions.patterns_added, vec!["*.ts"]);
        let content = std::fs::read_to_string(&gitattributes).unwrap();
        assert_eq!(content.matches("*.rs merge=weavr").count(), 1);
        assert!(content.contains("*.ts merge=weavr"));
    }

    #[test]
    fn gitattributes_created_when_missing() {
        let dir = tempfile::tempdir().unwrap();

        let mut actions = Actions::new();
        setup_gitattributes_in_dir(&["*.rs".to_string()], &mut actions, dir.path());

        assert_eq!(actions.patterns_added, vec!["*.rs"]);
        let content = std::fs::read_to_string(dir.path().join(".gitattributes")).unwrap();
        assert_eq!(content, "*.rs merge=weavr\n");
    }

    #[test]
    fn actions_did_something() {
        let mut actions = Actions::new();
        assert!(!actions.did_something());

        actions.config_created = true;
        assert!(actions.did_something());
    }

    /// Helper that exercises gitattributes logic without requiring a real git repo.
    fn setup_gitattributes_in_dir(patterns: &[String], actions: &mut Actions, repo_root: &Path) {
        let gitattributes_path = repo_root.join(".gitattributes");

        let existing_content = if gitattributes_path.exists() {
            std::fs::read_to_string(&gitattributes_path).unwrap()
        } else {
            String::new()
        };

        let mut lines_to_add = Vec::new();
        for pattern in patterns {
            let entry = format!("{pattern} merge=weavr");
            let already_present = existing_content.lines().any(|line| line.trim() == entry);

            if !already_present {
                lines_to_add.push(entry);
                actions.patterns_added.push(pattern.clone());
            }
        }

        if !lines_to_add.is_empty() {
            let mut content = existing_content;
            if !content.is_empty() && !content.ends_with('\n') {
                content.push('\n');
            }
            for line in &lines_to_add {
                content.push_str(line);
                content.push('\n');
            }
            std::fs::write(&gitattributes_path, content).unwrap();
        }
    }
}
