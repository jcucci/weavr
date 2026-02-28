//! Language-specific AST merger implementations.
//!
//! Each merger is gated behind a feature flag and provides language-aware
//! structural merging for a specific language.

#[cfg(feature = "rust")]
pub mod rust_merger;

#[cfg(feature = "csharp")]
pub mod csharp_merger;

// Future language mergers will be added here behind feature gates:
//
// #[cfg(feature = "typescript")]
// pub mod typescript_merger;
//
// #[cfg(feature = "go")]
// pub mod go_merger;
