use crate::cli::Strategy;

use super::raw::RawConfig;
use super::ConfigError;
use super::WeavrConfig;

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
                hint: "valid strategies: left, right, both, ast, ai".into(),
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
        "ai" => Some(Strategy::Ai),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::raw::{RawConfig, RawStrategiesConfig, RawThemeConfig};

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
    fn parse_strategy_valid() {
        assert_eq!(parse_strategy("left"), Some(Strategy::Left));
        assert_eq!(parse_strategy("RIGHT"), Some(Strategy::Right));
        assert_eq!(parse_strategy("Both"), Some(Strategy::Both));
    }

    #[test]
    fn parse_strategy_invalid() {
        assert_eq!(parse_strategy("unknown"), None);
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
        use crate::config::raw::RawJjConfig;

        let raw = RawConfig {
            jj: Some(RawJjConfig {
                squash_after_resolve: Some(true),
            }),
            ..RawConfig::default()
        };
        let config = WeavrConfig::from_raw(&raw).unwrap();
        assert!(config.squash_after_resolve);
    }
}
