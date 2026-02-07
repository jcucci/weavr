//! AI provider implementations.
//!
//! Each provider is feature-gated and contains both its configuration
//! and implementation in the same module.

#[cfg(feature = "ai-claude")]
pub mod claude;

#[cfg(feature = "ai-openai")]
pub mod openai;

#[cfg(feature = "ai-local")]
pub mod local;

// Re-export provider types for convenience
#[cfg(feature = "ai-claude")]
pub use claude::{ClaudeConfig, ClaudeProvider};

#[cfg(feature = "ai-openai")]
pub use openai::{OpenAiConfig, OpenAiProvider};

#[cfg(feature = "ai-local")]
pub use local::{LocalConfig, LocalProvider};
