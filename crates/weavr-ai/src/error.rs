//! Error types for AI provider operations.

use std::time::Duration;
use thiserror::Error;

/// Errors that can occur during AI provider operations.
#[derive(Debug, Error)]
pub enum AiError {
    /// API key not configured or invalid.
    #[error("API key error: {0}")]
    ApiKeyError(String),

    /// Network request failed.
    #[cfg(any(feature = "ai-claude", feature = "ai-openai", feature = "ai-local"))]
    #[error("network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    /// Provider returned an error response.
    #[error("provider error: {provider} returned {status}: {message}")]
    ProviderError {
        /// The provider that returned the error.
        provider: String,
        /// HTTP status code.
        status: u16,
        /// Error message from the provider.
        message: String,
    },

    /// Rate limit exceeded.
    #[error("rate limit exceeded for {provider}, retry after {retry_after_secs:?}s")]
    RateLimited {
        /// The provider that rate limited.
        provider: String,
        /// Seconds to wait before retrying.
        retry_after_secs: Option<u64>,
    },

    /// Failed to parse provider response.
    #[error("failed to parse response: {0}")]
    ParseError(String),

    /// Request timed out.
    #[error("request timed out after {0:?}")]
    Timeout(Duration),

    /// Provider is not configured or feature not enabled.
    #[error("provider '{0}' is not available")]
    ProviderNotAvailable(String),

    /// Context too large for provider.
    #[error("conflict too large: {size} bytes exceeds {max} byte limit")]
    ContextTooLarge {
        /// Actual size in bytes.
        size: usize,
        /// Maximum allowed size.
        max: usize,
    },
}
