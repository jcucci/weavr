//! Local LLM provider implementation (e.g., Ollama).

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use weavr_core::{ConflictHunk, Resolution};

use crate::error::AiError;
use crate::AiProvider;

/// Local LLM provider configuration.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct LocalConfig {
    /// Endpoint URL (e.g., `http://localhost:11434/api/generate` for Ollama).
    pub endpoint: Option<String>,

    /// Model name.
    pub model: Option<String>,
}

/// Local LLM provider.
pub struct LocalProvider {
    #[allow(dead_code)]
    endpoint: String,
    #[allow(dead_code)]
    model: String,
    #[allow(dead_code)]
    client: reqwest::Client,
}

impl LocalProvider {
    /// Creates a new local LLM provider from configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the endpoint or model is not configured.
    pub fn new(config: &LocalConfig) -> Result<Self, AiError> {
        let endpoint = config.endpoint.clone().ok_or_else(|| {
            AiError::ProviderNotAvailable("local LLM endpoint not configured".into())
        })?;

        let model = config.model.clone().ok_or_else(|| {
            AiError::ProviderNotAvailable("local LLM model not configured".into())
        })?;

        Ok(Self {
            endpoint,
            model,
            client: reqwest::Client::new(),
        })
    }
}

#[async_trait]
impl AiProvider for LocalProvider {
    fn name(&self) -> &'static str {
        "local"
    }

    async fn suggest(&self, _hunk: &ConflictHunk) -> Result<Option<Resolution>, AiError> {
        // TODO: Implement local LLM API integration (Ollama, llama.cpp, etc.)
        Err(AiError::ProviderNotAvailable(
            "Local LLM provider not yet implemented".into(),
        ))
    }

    async fn explain(&self, _hunk: &ConflictHunk) -> Result<Option<String>, AiError> {
        // TODO: Implement local LLM API integration
        Err(AiError::ProviderNotAvailable(
            "Local LLM provider not yet implemented".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = LocalConfig::default();
        assert!(config.endpoint.is_none());
        assert!(config.model.is_none());
    }

    #[test]
    fn deserialize_config() {
        let toml = r#"
            endpoint = "http://localhost:11434/api/generate"
            model = "codellama"
        "#;

        let config: LocalConfig = toml::from_str(toml).unwrap();
        assert_eq!(
            config.endpoint,
            Some("http://localhost:11434/api/generate".into())
        );
        assert_eq!(config.model, Some("codellama".into()));
    }
}
