//! Configuration file loading and merging.
//!
//! Supports layered configuration (lowest to highest priority):
//! 1. Compiled-in defaults
//! 2. User config: `~/.config/weavr/config.toml` (XDG)
//! 3. Project config: `.weavr.toml` in cwd
//! 4. `--config PATH` explicit file
//! 5. CLI flags (applied after `from_raw`)

mod raw;
mod validate;

use std::path::{Path, PathBuf};

use crate::cli::Strategy;

pub use raw::{RawConfig, RawKeybindingsConfig};
pub use validate::parse_theme_name;

// Re-export raw sub-types used by tests in other modules (e.g. merge_driver).
#[cfg(test)]
pub use raw::RawStrategiesConfig;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Error type for configuration loading and parsing.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read config file {path}: {source}")]
    ReadError {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to parse config file {path}: {source}")]
    ParseError {
        path: PathBuf,
        source: toml::de::Error,
    },

    #[error("invalid value for '{key}': '{value}' ({hint})")]
    InvalidValue {
        key: String,
        value: String,
        hint: String,
    },
}

// ---------------------------------------------------------------------------
// Resolved config
// ---------------------------------------------------------------------------

/// Fully resolved configuration with concrete, validated types.
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)] // Config structs are naturally boolean
pub struct WeavrConfig {
    pub theme: weavr_tui::theme::ThemeChoice,
    pub default_strategy: Strategy,
    pub deduplicate: bool,
    pub fail_on_ambiguous: bool,
    pub auto_stage: bool,
    pub stage_prompt: bool,
    #[cfg(feature = "jj")]
    pub squash_after_resolve: bool,
    #[cfg(feature = "ai")]
    pub ai: weavr_ai::AiConfig,
    #[cfg(feature = "ast")]
    pub ast: weavr_ast::AstConfig,
}

// ---------------------------------------------------------------------------
// Config loading
// ---------------------------------------------------------------------------

/// Returns the user-level config path: `~/.config/weavr/config.toml` (XDG).
#[must_use]
pub fn user_config_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "weavr")
        .map(|dirs| dirs.config_dir().join("config.toml"))
}

/// Loads and merges configuration layers 1-4.
///
/// 1. Compiled-in defaults (empty `RawConfig`)
/// 2. User config (`~/.config/weavr/config.toml`)
/// 3. Project config (`.weavr.toml` in cwd)
/// 4. Explicit `--config PATH` file
pub fn load_config(cli_path: Option<&Path>) -> Result<RawConfig, ConfigError> {
    let mut config = RawConfig::default();

    // Layer 2: User config
    if let Some(user_path) = user_config_path() {
        if user_path.exists() {
            let user_config = read_config_file(&user_path)?;
            config = user_config.merge(config);
        }
    }

    // Layer 3: Project config
    let project_path = PathBuf::from(".weavr.toml");
    if project_path.exists() {
        let project_config = read_config_file(&project_path)?;
        config = project_config.merge(config);
    }

    // Layer 4: Explicit --config file
    if let Some(path) = cli_path {
        let explicit_config = read_config_file(path)?;
        config = explicit_config.merge(config);
    }

    Ok(config)
}

/// Reads and parses a single TOML config file.
fn read_config_file(path: &Path) -> Result<RawConfig, ConfigError> {
    let content = std::fs::read_to_string(path).map_err(|source| ConfigError::ReadError {
        path: path.to_path_buf(),
        source,
    })?;

    toml::from_str(&content).map_err(|source| ConfigError::ParseError {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_config_path_returns_some() {
        assert!(user_config_path().is_some());
    }

    #[test]
    fn load_config_no_files() {
        // Run from a tempdir so neither ~/.config/weavr/config.toml nor
        // .weavr.toml can be accidentally picked up from the host.
        let dir = tempfile::tempdir().unwrap();
        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let raw = load_config(None).unwrap();

        std::env::set_current_dir(prev).unwrap();

        // User config may exist on the host, but project config won't.
        // With no explicit file, only the user layer (if any) contributes.
        // At minimum, strategies and headless should be None since no
        // .weavr.toml exists in the tempdir.
        assert!(raw.strategies.is_none());
        assert!(raw.headless.is_none());
    }

    #[test]
    fn read_config_file_missing() {
        let err = read_config_file(Path::new("/nonexistent/config.toml")).unwrap_err();
        assert!(matches!(err, ConfigError::ReadError { .. }));
    }

    #[test]
    fn read_config_file_invalid_toml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.toml");
        std::fs::write(&path, "this is not valid { toml").unwrap();

        let err = read_config_file(&path).unwrap_err();
        assert!(matches!(err, ConfigError::ParseError { .. }));
    }

    #[test]
    fn load_config_explicit_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(
            &path,
            r#"
[theme]
name = "nord"
"#,
        )
        .unwrap();

        let raw = load_config(Some(&path)).unwrap();
        assert_eq!(
            raw.theme.as_ref().and_then(|t| t.name.as_deref()),
            Some("nord")
        );
    }
}
