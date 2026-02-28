//! Language-specific AST merger implementations.
//!
//! Each merger is gated behind a feature flag and provides language-aware
//! structural merging for a specific language.
//!
//! Shared infrastructure (set merging, confidence scoring, test helpers) lives
//! in `common`, `confidence`, and `test_utils` — these are compiled when any
//! language feature is active, not gated to a single language.

#[cfg(any(feature = "rust", feature = "csharp", feature = "typescript"))]
pub(crate) mod common;
#[cfg(any(feature = "rust", feature = "csharp", feature = "typescript"))]
pub(crate) mod confidence;
#[cfg(all(
    test,
    any(feature = "rust", feature = "csharp", feature = "typescript")
))]
pub(crate) mod test_utils;

#[cfg(feature = "rust")]
pub mod rust_merger;

#[cfg(feature = "csharp")]
pub mod csharp_merger;

#[cfg(feature = "typescript")]
pub mod typescript_merger;

// Future language mergers will be added here behind feature gates:
//
// #[cfg(feature = "go")]
// pub mod go_merger;
