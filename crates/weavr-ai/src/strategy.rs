//! AI strategy wrapper for resolution workflows.

use weavr_core::ConflictHunk;
use weavr_core::Resolution;

use crate::config::AiConfig;
use crate::error::AiError;
use crate::AiProvider;

/// Wraps an `AiProvider` to produce `Resolution` objects.
///
/// This struct handles configuration-based filtering (e.g., minimum confidence)
/// and provides a consistent interface for the CLI/TUI to request AI suggestions.
pub struct AiStrategy {
    provider: Box<dyn AiProvider>,
    config: AiConfig,
}

impl AiStrategy {
    /// Creates a new `AiStrategy` with the given provider and configuration.
    #[must_use]
    pub fn new(provider: Box<dyn AiProvider>, config: AiConfig) -> Self {
        Self { provider, config }
    }

    /// Returns the provider name.
    #[must_use]
    pub fn provider_name(&self) -> &str {
        self.provider.name()
    }

    /// Returns whether AI is enabled in the configuration.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Asynchronously suggests a resolution for the hunk.
    ///
    /// Returns `Ok(None)` if:
    /// - AI is disabled in config
    /// - Provider returned no suggestion
    /// - Confidence is below the configured threshold
    ///
    /// # Errors
    ///
    /// Returns an error if the provider fails to generate a suggestion.
    pub async fn suggest(&self, hunk: &ConflictHunk) -> Result<Option<Resolution>, AiError> {
        if !self.config.enabled {
            return Ok(None);
        }

        let response = self.provider.suggest(hunk).await?;

        // Filter by confidence threshold
        match response {
            Some(resolution) => {
                if self.config.min_confidence > 0 {
                    match resolution.metadata.confidence {
                        Some(conf) if conf < self.config.min_confidence => {
                            return Ok(None);
                        }
                        None => {
                            // When min_confidence is configured, treat missing confidence
                            // as below threshold for predictable behavior.
                            return Ok(None);
                        }
                        _ => {}
                    }
                }
                Ok(Some(resolution))
            }
            None => Ok(None),
        }
    }

    /// Asynchronously explains the conflict.
    ///
    /// Returns `Ok(None)` if AI is disabled or the provider declines to explain.
    ///
    /// # Errors
    ///
    /// Returns an error if the provider fails to generate an explanation.
    pub async fn explain(&self, hunk: &ConflictHunk) -> Result<Option<String>, AiError> {
        if !self.config.enabled {
            return Ok(None);
        }
        self.provider.explain(hunk).await
    }
}
