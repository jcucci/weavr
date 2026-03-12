//! Configuration file loading and merging.
//!
//! Supports layered configuration (lowest to highest priority):
//! 1. Compiled-in defaults
//! 2. User config: `~/.config/weavr/config.toml` (XDG)
//! 3. Project config: `.weavr.toml` in cwd
//! 4. `--config PATH` explicit file
//! 5. CLI flags (applied after `from_raw`)

use std::collections::BTreeMap;
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

/// Raw git integration configuration section.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawGitConfig {
    pub auto_stage: Option<bool>,
    pub stage_prompt: Option<bool>,
}

/// A keybinding value in config: either a single key string or multiple.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum RawKeybindingValue {
    /// A single key notation string (e.g., `"j"`).
    Single(String),
    /// Multiple key notation strings (e.g., `["j", "<Down>"]`).
    Multiple(Vec<String>),
}

impl RawKeybindingValue {
    /// Converts to a `Vec<String>` regardless of the variant.
    #[must_use]
    pub fn into_vec(self) -> Vec<String> {
        match self {
            Self::Single(s) => vec![s],
            Self::Multiple(v) => v,
        }
    }
}

/// Raw keybindings configuration section.
///
/// Action names are `snake_case` keys mapping to key notation strings.
/// Example: `next_hunk = "j"` or `next_hunk = ["j", "<Down>"]`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RawKeybindingsConfig {
    /// Action-to-key mappings.
    #[serde(flatten)]
    pub bindings: BTreeMap<String, RawKeybindingValue>,
}

impl RawKeybindingsConfig {
    /// Converts bindings into the format expected by
    /// [`weavr_tui::keybindings::build_from_config`].
    #[must_use]
    pub fn into_key_lists(self) -> BTreeMap<String, Vec<String>> {
        self.bindings
            .into_iter()
            .map(|(k, v)| (k, v.into_vec()))
            .collect()
    }
}

/// Raw jj integration configuration section.
#[cfg(feature = "jj")]
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawJjConfig {
    /// Override jj's conflict marker style (`"diff"` or `"snapshot"`).
    pub conflict_marker_style: Option<String>,
    /// Run `jj squash` after all hunks in a file are resolved.
    pub squash_after_resolve: Option<bool>,
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
    pub git: Option<RawGitConfig>,
    pub keybindings: Option<RawKeybindingsConfig>,

    #[cfg(feature = "jj")]
    pub jj: Option<RawJjConfig>,

    #[cfg(feature = "ai")]
    pub ai: Option<weavr_ai::AiConfig>,

    #[cfg(feature = "ast")]
    pub ast: Option<weavr_ast::AstConfig>,
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
            git: merge_option(self.git, lower.git, |hi, lo| RawGitConfig {
                auto_stage: hi.auto_stage.or(lo.auto_stage),
                stage_prompt: hi.stage_prompt.or(lo.stage_prompt),
            }),
            keybindings: merge_option(self.keybindings, lower.keybindings, |hi, lo| {
                // Action-level overlay: higher priority replaces bindings
                // for the same action, lower fills in the rest.
                let mut merged = lo.bindings;
                merged.extend(hi.bindings);
                RawKeybindingsConfig { bindings: merged }
            }),
            #[cfg(feature = "jj")]
            jj: merge_option(self.jj, lower.jj, |hi, lo| RawJjConfig {
                conflict_marker_style: hi.conflict_marker_style.or(lo.conflict_marker_style),
                squash_after_resolve: hi.squash_after_resolve.or(lo.squash_after_resolve),
            }),
            #[cfg(feature = "ai")]
            ai: self.ai.or(lower.ai),
            #[cfg(feature = "ast")]
            ast: self.ast.or(lower.ast),
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
#[allow(clippy::struct_excessive_bools)] // Config structs are naturally boolean
pub struct WeavrConfig {
    pub theme: weavr_tui::theme::ThemeName,
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
                hint: "valid strategies: left, right, both, ast".into(),
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

        let auto_stage = raw.git.as_ref().and_then(|g| g.auto_stage).unwrap_or(false);

        let stage_prompt = raw
            .git
            .as_ref()
            .and_then(|g| g.stage_prompt)
            .unwrap_or(true);

        #[cfg(feature = "jj")]
        let squash_after_resolve = raw
            .jj
            .as_ref()
            .and_then(|j| j.squash_after_resolve)
            .unwrap_or(false);

        Ok(Self {
            theme,
            default_strategy,
            deduplicate,
            fail_on_ambiguous,
            auto_stage,
            stage_prompt,
            #[cfg(feature = "jj")]
            squash_after_resolve,
            #[cfg(feature = "ai")]
            ai: raw.ai.clone().unwrap_or_default(),
            #[cfg(feature = "ast")]
            ast: raw.ast.clone().unwrap_or_default(),
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
        "ast" => Some(Strategy::Ast),
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
        assert!(config.git.is_none());
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
        assert!(!config.auto_stage);
        assert!(config.stage_prompt);
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

    #[test]
    fn parse_keybindings_single_key() {
        let toml_str = r#"
[keybindings]
next_hunk = "n"
quit = "q"
"#;
        let raw: RawConfig = toml::from_str(toml_str).unwrap();
        let kb = raw.keybindings.unwrap();
        assert!(matches!(
            kb.bindings.get("next_hunk"),
            Some(RawKeybindingValue::Single(s)) if s == "n"
        ));
    }

    #[test]
    fn parse_keybindings_multiple_keys() {
        let toml_str = r#"
[keybindings]
next_hunk = ["j", "<Down>"]
"#;
        let raw: RawConfig = toml::from_str(toml_str).unwrap();
        let kb = raw.keybindings.unwrap();
        assert!(matches!(
            kb.bindings.get("next_hunk"),
            Some(RawKeybindingValue::Multiple(v)) if v.len() == 2
        ));
    }

    #[test]
    fn parse_keybindings_with_other_sections() {
        let toml_str = r#"
[theme]
name = "nord"

[keybindings]
resolve_left = "a"
"#;
        let raw: RawConfig = toml::from_str(toml_str).unwrap();
        assert!(raw.theme.is_some());
        assert!(raw.keybindings.is_some());
    }

    #[test]
    fn merge_keybindings_action_level_overlay() {
        let higher = RawConfig {
            keybindings: Some(RawKeybindingsConfig {
                bindings: {
                    let mut m = BTreeMap::new();
                    m.insert("quit".into(), RawKeybindingValue::Single("Q".into()));
                    m
                },
            }),
            ..RawConfig::default()
        };
        let lower = RawConfig {
            keybindings: Some(RawKeybindingsConfig {
                bindings: {
                    let mut m = BTreeMap::new();
                    m.insert("quit".into(), RawKeybindingValue::Single("q".into()));
                    m.insert("next_hunk".into(), RawKeybindingValue::Single("j".into()));
                    m
                },
            }),
            ..RawConfig::default()
        };

        let merged = higher.merge(lower);
        let kb = merged.keybindings.unwrap();

        // Higher priority wins for same action
        assert!(matches!(
            kb.bindings.get("quit"),
            Some(RawKeybindingValue::Single(s)) if s == "Q"
        ));
        // Lower fills in missing actions
        assert!(matches!(
            kb.bindings.get("next_hunk"),
            Some(RawKeybindingValue::Single(s)) if s == "j"
        ));
    }

    #[test]
    fn parse_toml_git_section() {
        let toml_str = r"
[git]
auto_stage = true
stage_prompt = false
";
        let raw: RawConfig = toml::from_str(toml_str).unwrap();
        let config = WeavrConfig::from_raw(&raw).unwrap();
        assert!(config.auto_stage);
        assert!(!config.stage_prompt);
    }

    #[test]
    fn merge_git_config() {
        let higher = RawConfig {
            git: Some(RawGitConfig {
                auto_stage: Some(true),
                stage_prompt: None,
            }),
            ..RawConfig::default()
        };
        let lower = RawConfig {
            git: Some(RawGitConfig {
                auto_stage: None,
                stage_prompt: Some(false),
            }),
            ..RawConfig::default()
        };

        let merged = higher.merge(lower);
        let git = merged.git.unwrap();
        assert_eq!(git.auto_stage, Some(true));
        assert_eq!(git.stage_prompt, Some(false));
    }

    #[cfg(feature = "jj")]
    #[test]
    fn parse_toml_jj_section() {
        let toml_str = r#"
[jj]
conflict_marker_style = "snapshot"
squash_after_resolve = true
"#;
        let raw: RawConfig = toml::from_str(toml_str).unwrap();
        let jj = raw.jj.unwrap();
        assert_eq!(jj.conflict_marker_style.as_deref(), Some("snapshot"));
        assert_eq!(jj.squash_after_resolve, Some(true));
    }

    #[cfg(feature = "jj")]
    #[test]
    fn merge_jj_config() {
        let higher = RawConfig {
            jj: Some(RawJjConfig {
                conflict_marker_style: Some("snapshot".into()),
                squash_after_resolve: None,
            }),
            ..RawConfig::default()
        };
        let lower = RawConfig {
            jj: Some(RawJjConfig {
                conflict_marker_style: None,
                squash_after_resolve: Some(true),
            }),
            ..RawConfig::default()
        };

        let merged = higher.merge(lower);
        let jj = merged.jj.unwrap();
        assert_eq!(jj.conflict_marker_style.as_deref(), Some("snapshot"));
        assert_eq!(jj.squash_after_resolve, Some(true));
    }

    #[cfg(feature = "jj")]
    #[test]
    fn from_raw_jj_defaults() {
        let config = WeavrConfig::from_raw(&RawConfig::default()).unwrap();
        assert!(!config.squash_after_resolve);
    }

    #[cfg(feature = "jj")]
    #[test]
    fn from_raw_jj_squash_enabled() {
        let raw = RawConfig {
            jj: Some(RawJjConfig {
                conflict_marker_style: None,
                squash_after_resolve: Some(true),
            }),
            ..RawConfig::default()
        };
        let config = WeavrConfig::from_raw(&raw).unwrap();
        assert!(config.squash_after_resolve);
    }

    #[test]
    fn no_keybindings_section_is_none() {
        let toml_str = r#"
[theme]
name = "dark"
"#;
        let raw: RawConfig = toml::from_str(toml_str).unwrap();
        assert!(raw.keybindings.is_none());
    }
}
