//! weavr-ast: AST-based merge strategies for language-aware conflict resolution.
//!
//! This crate provides the [`AstMerger`] trait contract and [`AstStrategy`] coordinator
//! for language-specific structural merging. Individual language implementations are
//! gated behind feature flags.
//!
//! # Architecture
//!
//! - [`AstMerger`] — trait that language-specific mergers implement
//! - [`AstStrategy`] — wraps registered mergers with config filtering and confidence thresholds
//! - Returns `Option<Resolution>` — caller controls fallback to text-based strategies
//!
//! # Feature Flags
//!
//! Language mergers are opt-in:
//! - `rust` — Rust AST merger (stub)
//! - `csharp` — C# AST merger (stub)
//! - `typescript` — TypeScript AST merger (stub)
//! - `go` — Go AST merger (stub)
//! - `all-languages` — enables all language mergers

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod config;
mod error;
pub mod mergers;
mod strategy;

pub use config::AstConfig;
pub use error::AstError;
pub use strategy::AstStrategy;

use std::path::Path;

use weavr_core::{ConflictHunk, Language};

/// Result of a successful AST-based merge operation.
#[derive(Debug, Clone)]
pub struct AstMergeResult {
    /// The merged content.
    pub content: String,
    /// Confidence score from 0.0 to 1.0.
    pub confidence: f32,
    /// Human-readable description of the merge (e.g., "Merged 3 imports").
    pub description: String,
}

/// Trait for language-specific AST merge implementations.
///
/// Implementors provide structural merging for one or more languages.
/// The merge operation is synchronous (CPU-bound work, no I/O).
pub trait AstMerger: Send + Sync {
    /// Returns the languages this merger can handle.
    fn supported_languages(&self) -> &[Language];

    /// Returns whether this merger can handle the given file and language.
    fn supports(&self, path: &Path, language: Language) -> bool;

    /// Attempts to merge a conflict hunk using AST analysis.
    ///
    /// Returns `Ok(Some(result))` on success, `Ok(None)` if the merger
    /// cannot handle this particular conflict (e.g., too complex),
    /// or `Err` on parse/internal errors.
    ///
    /// # Errors
    ///
    /// Returns [`AstError::ParseError`] if the source cannot be parsed,
    /// or [`AstError::Internal`] on unexpected merge failures.
    fn try_merge(&self, hunk: &ConflictHunk) -> Result<Option<AstMergeResult>, AstError>;
}
