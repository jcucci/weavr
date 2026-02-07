//! AI provider integrations for weavr merge conflict resolver.
//!
//! This crate provides AI-assisted conflict resolution through various providers.
//! All AI features are opt-in and suggestions **never auto-apply**.
//!
//! # Feature Flags
//!
//! - `ai-claude` - Enables the Claude provider
//! - `ai-openai` - Enables the `OpenAI` provider
//! - `ai-local` - Enables local LLM support (Ollama, etc.)
//! - `all-providers` - Enables all providers
//!
//! # Example
//!
//! ```ignore
//! use weavr_ai::{AiProvider, AiStrategy, AiConfig};
//! use weavr_ai::providers::ClaudeProvider;
//!
//! let config = AiConfig {
//!     enabled: true,
//!     provider: Some("claude".into()),
//!     ..Default::default()
//! };
//!
//! let provider = ClaudeProvider::new(&config.claude)?;
//! let strategy = AiStrategy::new(Box::new(provider), config);
//!
//! // Request a suggestion (async)
//! let suggestion = strategy.suggest(&hunk).await?;
//! ```

pub mod config;
pub mod error;
pub mod providers;
pub mod request;
pub mod strategy;

pub use config::AiConfig;
pub use error::AiError;
pub use request::{AiRequest, AiResponse, ConflictContext};
pub use strategy::AiStrategy;

use async_trait::async_trait;
use weavr_core::ConflictHunk;
use weavr_core::Resolution;

/// Trait for AI providers that can suggest conflict resolutions.
///
/// All methods are async and non-blocking. Providers must not auto-apply
/// any resolutions - they only suggest.
///
/// # Implementation Notes
///
/// - `suggest` should return a `Resolution` with `ResolutionStrategyKind::AiSuggested`
/// - `explain` provides natural language explanation without suggesting a resolution
/// - Both methods may return `Ok(None)` if the provider declines to respond
#[async_trait]
pub trait AiProvider: Send + Sync {
    /// Returns the provider name (e.g., "claude", "openai", "local").
    fn name(&self) -> &'static str;

    /// Suggests a resolution for the given conflict hunk.
    ///
    /// Returns `Ok(Some(resolution))` if a suggestion was generated,
    /// `Ok(None)` if the provider declined to suggest (e.g., low confidence),
    /// or `Err(AiError)` if an error occurred.
    async fn suggest(&self, hunk: &ConflictHunk) -> Result<Option<Resolution>, AiError>;

    /// Generates a natural-language explanation of the conflict.
    ///
    /// Returns `Ok(Some(explanation))` if generated successfully,
    /// `Ok(None)` if the provider declined to explain,
    /// or `Err(AiError)` if an error occurred.
    async fn explain(&self, hunk: &ConflictHunk) -> Result<Option<String>, AiError>;
}
