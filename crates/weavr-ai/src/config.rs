//! Top-level AI configuration.

use serde::{Deserialize, Serialize};
use std::time::Duration;

#[cfg(feature = "ai-claude")]
use crate::providers::claude::ClaudeConfig;
#[cfg(feature = "ai-local")]
use crate::providers::local::LocalConfig;
#[cfg(feature = "ai-openai")]
use crate::providers::openai::OpenAiConfig;

/// Top-level AI configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AiConfig {
    /// Whether AI features are enabled.
    #[serde(default)]
    pub enabled: bool,

    /// Which provider to use (e.g., "claude", "openai", "local").
    pub provider: Option<String>,

    /// Request timeout.
    #[serde(default = "default_timeout", with = "humantime_serde")]
    pub timeout: Duration,

    /// Minimum confidence threshold to show suggestions (0.0 to 1.0).
    #[serde(default = "default_min_confidence")]
    pub min_confidence: f32,

    /// Whether to automatically suggest resolutions (still requires user acceptance).
    #[serde(default)]
    pub auto_suggest: bool,

    /// Claude-specific configuration.
    #[cfg(feature = "ai-claude")]
    #[serde(default)]
    pub claude: ClaudeConfig,

    /// OpenAI-specific configuration.
    #[cfg(feature = "ai-openai")]
    #[serde(default)]
    pub openai: OpenAiConfig,

    /// Local LLM configuration.
    #[cfg(feature = "ai-local")]
    #[serde(default)]
    pub local: LocalConfig,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: None,
            timeout: default_timeout(),
            min_confidence: default_min_confidence(),
            auto_suggest: false,
            #[cfg(feature = "ai-claude")]
            claude: ClaudeConfig::default(),
            #[cfg(feature = "ai-openai")]
            openai: OpenAiConfig::default(),
            #[cfg(feature = "ai-local")]
            local: LocalConfig::default(),
        }
    }
}

fn default_timeout() -> Duration {
    Duration::from_secs(30)
}

fn default_min_confidence() -> f32 {
    0.7
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = AiConfig::default();
        assert!(!config.enabled);
        assert!(config.provider.is_none());
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert!((config.min_confidence - 0.7).abs() < f32::EPSILON);
    }
}
