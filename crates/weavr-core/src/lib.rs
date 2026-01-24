//! weavr-core: Pure merge logic engine
//!
//! This crate contains the core domain model and merge logic for weavr.
//! It is intentionally pure and has no dependencies on:
//! - Filesystem operations
//! - Git commands
//! - UI/terminal rendering
//! - Network/AI providers
//!
//! All merge decisions are explicit and deterministic.
//!
//! # Stability
//!
//! This crate follows [Semantic Versioning](https://semver.org/). All public types
//! and methods are considered **stable** unless documented otherwise.
//!
//! See the [README](https://github.com/jcucci/weavr/blob/main/crates/weavr-core/README.md)
//! for the full API stability policy.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod error;
mod hunk;
mod input;
mod parser;
mod resolution;
mod result;
mod session;

pub use error::*;
pub use hunk::*;
pub use input::*;
pub use parser::*;
pub use resolution::*;
pub use result::*;
pub use session::*;
