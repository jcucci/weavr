//! Configuration file loading and merging.
//!
//! Supports layered configuration (lowest to highest priority):
//! 1. Compiled-in defaults
//! 2. User config: `~/.config/weavr/config.toml` (XDG)
//! 3. Project config: `.weavr.toml` in cwd
//! 4. `--config PATH` explicit file
//! 5. CLI flags (applied after `from_raw`)

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::cli::Strategy;

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
// Raw (deserializable) config types — all Option for merge support
// ---------------------------------------------------------------------------

/// Raw theme configuration section.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawThemeConfig {
    pub name: Option<String>,
}

/// Raw strategies configuration section.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawStrategiesConfig {
    pub default: Option<String>,
    pub deduplicate: Option<bool>,
}

/// Raw headless configuration section.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawHeadlessConfig {
    pub fail_on_ambiguous: Option<bool>,
}

/// Raw TOML configuration. All fields optional for layered merging.
///
/// Top-level struct does NOT use `deny_unknown_fields` so that `[ai]`
/// is silently ignored when the `ai` feature is disabled.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RawConfig {
    pub theme: Option<RawThemeConfig>,
    pub strategies: Option<RawStrategiesConfig>,
    pub headless: Option<RawHeadlessConfig>,

    #[cfg(feature = "ai")]
    pub ai: Option<weavr_ai::AiConfig>,
}

impl RawConfig {
    /// Merges two configs. `self` (higher priority) wins for `Some` fields,
    /// falls back to `lower` for `None` fields.
    #[must_use]
    pub fn merge(self, lower: Self) -> Self {
        Self {
            theme: merge_option(self.theme, lower.theme, |hi, lo| RawThemeConfig {
                name: hi.name.or(lo.name),
            }),
            strategies: merge_option(self.strategies, lower.strategies, |hi, lo| {
                RawStrategiesConfig {
                    default: hi.default.or(lo.default),
                    deduplicate: hi.deduplicate.or(lo.deduplicate),
                }
            }),
            headless: merge_option(self.headless, lower.headless, |hi, lo| RawHeadlessConfig {
                fail_on_ambiguous: hi.fail_on_ambiguous.or(lo.fail_on_ambiguous),
            }),
            #[cfg(feature = "ai")]
            ai: self.ai.or(lower.ai),
        }
    }
}

/// Merges two `Option<T>` values with a field-level combiner.
fn merge_option<T>(
    higher: Option<T>,
    lower: Option<T>,
    combine: impl FnOnce(T, T) -> T,
) -> Option<T> {
    match (higher, lower) {
        (Some(hi), Some(lo)) => Some(combine(hi, lo)),
        (hi @ Some(_), None) => hi,
        (None, lo @ Some(_)) => lo,
        (None, None) => None,
    }
}

// ---------------------------------------------------------------------------
// Resolved config
// ---------------------------------------------------------------------------

/// Fully resolved configuration with concrete, validated types.
#[derive(Debug, Clone)]
pub struct WeavrConfig {
    pub theme: weavr_tui::theme::ThemeName,
    pub default_strategy: Strategy,
    pub deduplicate: bool,
    pub fail_on_ambiguous: bool,
    #[cfg(feature = "ai")]
    pub ai: weavr_ai::AiConfig,
}

impl WeavrConfig {
    /// Resolves a [`RawConfig`] into a validated [`WeavrConfig`].
    ///
    /// Returns a [`ConfigError::InvalidValue`] for unrecognized theme names
    /// or strategy names, with a hint listing valid values.
    pub fn from_raw(raw: &RawConfig) -> Result<Self, ConfigError> {
        let theme = match raw.theme.as_ref().and_then(|t| t.name.as_deref()) {
            Some(name) => parse_theme_name(name)?,
            None => weavr_tui::theme::ThemeName::default(),
        };

        let default_strategy = match raw.strategies.as_ref().and_then(|s| s.default.as_deref()) {
            Some(name) => parse_strategy(name).ok_or_else(|| ConfigError::InvalidValue {
                key: "strategies.default".into(),
                value: name.into(),
                hint: "valid strategies: left, right, both".into(),
            })?,
            None => Strategy::Left,
        };

        let deduplicate = raw
            .strategies
            .as_ref()
            .and_then(|s| s.deduplicate)
            .unwrap_or(false);

        let fail_on_ambiguous = raw
            .headless
            .as_ref()
            .and_then(|h| h.fail_on_ambiguous)
            .unwrap_or(false);

        Ok(Self {
            theme,
            default_strategy,
            deduplicate,
            fail_on_ambiguous,
            #[cfg(feature = "ai")]
            ai: raw.ai.clone().unwrap_or_default(),
        })
    }
}

/// Parses a theme name string, returning a helpful error with valid theme names on failure.
pub fn parse_theme_name(s: &str) -> Result<weavr_tui::theme::ThemeName, ConfigError> {
    s.parse::<weavr_tui::theme::ThemeName>().map_err(|_| {
        let valid: Vec<_> = weavr_tui::theme::ThemeName::all()
            .iter()
            .map(ToString::to_string)
            .collect();
        ConfigError::InvalidValue {
            key: "theme.name".into(),
            value: s.into(),
            hint: format!("valid themes: {}", valid.join(", ")),
        }
    })
}

/// Parses a strategy name string into a [`Strategy`] variant.
fn parse_strategy(s: &str) -> Option<Strategy> {
    match s.to_lowercase().as_str() {
        "left" => Some(Strategy::Left),
        "right" => Some(Strategy::Right),
        "both" => Some(Strategy::Both),
        _ => None,
    }
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
    fn default_raw_config_is_all_none() {
        let config = RawConfig::default();
        assert!(config.theme.is_none());
        assert!(config.strategies.is_none());
        assert!(config.headless.is_none());
    }

    #[test]
    fn merge_higher_wins() {
        let higher = RawConfig {
            theme: Some(RawThemeConfig {
                name: Some("dracula".into()),
            }),
            ..RawConfig::default()
        };
        let lower = RawConfig {
            theme: Some(RawThemeConfig {
                name: Some("nord".into()),
            }),
            ..RawConfig::default()
        };

        let merged = higher.merge(lower);
        assert_eq!(
            merged.theme.as_ref().and_then(|t| t.name.as_deref()),
            Some("dracula")
        );
    }

    #[test]
    fn merge_falls_back_to_lower() {
        let higher = RawConfig::default();
        let lower = RawConfig {
            theme: Some(RawThemeConfig {
                name: Some("nord".into()),
            }),
            strategies: Some(RawStrategiesConfig {
                default: Some("right".into()),
                deduplicate: Some(true),
            }),
            ..RawConfig::default()
        };

        let merged = higher.merge(lower);
        assert_eq!(
            merged.theme.as_ref().and_then(|t| t.name.as_deref()),
            Some("nord")
        );
        assert_eq!(
            merged
                .strategies
                .as_ref()
                .and_then(|s| s.default.as_deref()),
            Some("right")
        );
        assert_eq!(
            merged.strategies.as_ref().and_then(|s| s.deduplicate),
            Some(true)
        );
    }

    #[test]
    fn merge_field_level_granularity() {
        let higher = RawConfig {
            strategies: Some(RawStrategiesConfig {
                default: Some("both".into()),
                deduplicate: None,
            }),
            ..RawConfig::default()
        };
        let lower = RawConfig {
            strategies: Some(RawStrategiesConfig {
                default: None,
                deduplicate: Some(true),
            }),
            ..RawConfig::default()
        };

        let merged = higher.merge(lower);
        let strategies = merged.strategies.unwrap();
        assert_eq!(strategies.default.as_deref(), Some("both"));
        assert_eq!(strategies.deduplicate, Some(true));
    }

    #[test]
    fn from_raw_defaults() {
        let config = WeavrConfig::from_raw(&RawConfig::default()).unwrap();
        assert_eq!(config.theme, weavr_tui::theme::ThemeName::Dark);
        assert_eq!(config.default_strategy, Strategy::Left);
        assert!(!config.deduplicate);
        assert!(!config.fail_on_ambiguous);
    }

    #[test]
    fn from_raw_valid_theme() {
        let raw = RawConfig {
            theme: Some(RawThemeConfig {
                name: Some("nord".into()),
            }),
            ..RawConfig::default()
        };
        let config = WeavrConfig::from_raw(&raw).unwrap();
        assert_eq!(config.theme, weavr_tui::theme::ThemeName::Nord);
    }

    #[test]
    fn from_raw_invalid_theme() {
        let raw = RawConfig {
            theme: Some(RawThemeConfig {
                name: Some("nonexistent".into()),
            }),
            ..RawConfig::default()
        };
        let err = WeavrConfig::from_raw(&raw).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("theme.name"));
        assert!(msg.contains("nonexistent"));
        assert!(msg.contains("valid themes:"));
    }

    #[test]
    fn from_raw_valid_strategy() {
        let raw = RawConfig {
            strategies: Some(RawStrategiesConfig {
                default: Some("right".into()),
                deduplicate: None,
            }),
            ..RawConfig::default()
        };
        let config = WeavrConfig::from_raw(&raw).unwrap();
        assert_eq!(config.default_strategy, Strategy::Right);
    }

    #[test]
    fn from_raw_invalid_strategy() {
        let raw = RawConfig {
            strategies: Some(RawStrategiesConfig {
                default: Some("invalid".into()),
                deduplicate: None,
            }),
            ..RawConfig::default()
        };
        let err = WeavrConfig::from_raw(&raw).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("strategies.default"));
        assert!(msg.contains("invalid"));
    }

    #[test]
    fn parse_toml_roundtrip() {
        let toml_str = r#"
[theme]
name = "dracula"

[strategies]
default = "both"
deduplicate = true

[headless]
fail_on_ambiguous = true
"#;
        let raw: RawConfig = toml::from_str(toml_str).unwrap();
        let config = WeavrConfig::from_raw(&raw).unwrap();
        assert_eq!(config.theme, weavr_tui::theme::ThemeName::Dracula);
        assert_eq!(config.default_strategy, Strategy::Both);
        assert!(config.deduplicate);
        assert!(config.fail_on_ambiguous);
    }

    #[test]
    fn parse_toml_unknown_top_level_section_ignored() {
        let toml_str = r#"
[theme]
name = "dark"

[unknown_section]
foo = "bar"
"#;
        let raw: RawConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(
            raw.theme.as_ref().and_then(|t| t.name.as_deref()),
            Some("dark")
        );
    }

    #[test]
    fn parse_toml_empty() {
        let raw: RawConfig = toml::from_str("").unwrap();
        assert!(raw.theme.is_none());
    }

    #[test]
    fn user_config_path_returns_some() {
        assert!(user_config_path().is_some());
    }

    #[test]
    fn parse_strategy_valid() {
        assert_eq!(parse_strategy("left"), Some(Strategy::Left));
        assert_eq!(parse_strategy("RIGHT"), Some(Strategy::Right));
        assert_eq!(parse_strategy("Both"), Some(Strategy::Both));
    }

    #[test]
    fn parse_strategy_invalid() {
        assert_eq!(parse_strategy("unknown"), None);
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
