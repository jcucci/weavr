//! `OpenAI` provider implementation.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use weavr_core::{ConflictHunk, Resolution};

use crate::error::AiError;
use crate::AiProvider;

/// `OpenAI` provider configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OpenAiConfig {
    /// Environment variable containing the API key.
    #[serde(default = "default_api_key_env")]
    pub api_key_env: String,

    /// Model to use.
    #[serde(default = "default_model")]
    pub model: String,

    /// Maximum tokens in response.
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
}

impl Default for OpenAiConfig {
    fn default() -> Self {
        Self {
            api_key_env: default_api_key_env(),
            model: default_model(),
            max_tokens: default_max_tokens(),
        }
    }
}

fn default_api_key_env() -> String {
    "OPENAI_API_KEY".into()
}

fn default_model() -> String {
    "gpt-4".into()
}

fn default_max_tokens() -> u32 {
    4096
}

/// `OpenAI` provider.
pub struct OpenAiProvider {
    #[allow(dead_code)]
    api_key: String,
    #[allow(dead_code)]
    model: String,
    #[allow(dead_code)]
    max_tokens: u32,
    #[allow(dead_code)]
    client: reqwest::Client,
}

impl OpenAiProvider {
    /// Creates a new `OpenAI` provider from configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the API key environment variable is not set.
    pub fn new(config: &OpenAiConfig) -> Result<Self, AiError> {
        let api_key = std::env::var(&config.api_key_env).map_err(|_| {
            AiError::ApiKeyError(format!(
                "environment variable {} not set",
                config.api_key_env
            ))
        })?;

        Ok(Self {
            api_key,
            model: config.model.clone(),
            max_tokens: config.max_tokens,
            client: reqwest::Client::new(),
        })
    }
}

#[async_trait]
impl AiProvider for OpenAiProvider {
    fn name(&self) -> &'static str {
        "openai"
    }

    async fn suggest(&self, _hunk: &ConflictHunk) -> Result<Option<Resolution>, AiError> {
        // TODO: Implement OpenAI API integration
        Err(AiError::ProviderNotAvailable(
            "OpenAI provider not yet implemented".into(),
        ))
    }

    async fn explain(&self, _hunk: &ConflictHunk) -> Result<Option<String>, AiError> {
        // TODO: Implement OpenAI API integration
        Err(AiError::ProviderNotAvailable(
            "OpenAI provider not yet implemented".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = OpenAiConfig::default();
        assert_eq!(config.api_key_env, "OPENAI_API_KEY");
        assert_eq!(config.model, "gpt-4");
        assert_eq!(config.max_tokens, 4096);
    }
}
