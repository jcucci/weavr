use std::collections::BTreeMap;

use serde::Deserialize;

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
        let config = crate::config::WeavrConfig::from_raw(&raw).unwrap();
        assert_eq!(config.theme, weavr_tui::theme::ThemeName::Dracula);
        assert_eq!(config.default_strategy, crate::cli::Strategy::Both);
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
        let config = crate::config::WeavrConfig::from_raw(&raw).unwrap();
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

    #[test]
    fn no_keybindings_section_is_none() {
        let toml_str = r#"
[theme]
name = "dark"
"#;
        let raw: RawConfig = toml::from_str(toml_str).unwrap();
        assert!(raw.keybindings.is_none());
    }

    #[cfg(feature = "jj")]
    #[test]
    fn parse_toml_jj_section() {
        let toml_str = r"
[jj]
squash_after_resolve = true
";
        let raw: RawConfig = toml::from_str(toml_str).unwrap();
        let jj = raw.jj.unwrap();
        assert_eq!(jj.squash_after_resolve, Some(true));
    }

    #[cfg(feature = "jj")]
    #[test]
    fn merge_jj_config() {
        let higher = RawConfig {
            jj: Some(RawJjConfig {
                squash_after_resolve: None,
            }),
            ..RawConfig::default()
        };
        let lower = RawConfig {
            jj: Some(RawJjConfig {
                squash_after_resolve: Some(true),
            }),
            ..RawConfig::default()
        };

        let merged = higher.merge(lower);
        let jj = merged.jj.unwrap();
        assert_eq!(jj.squash_after_resolve, Some(true));
    }
}
