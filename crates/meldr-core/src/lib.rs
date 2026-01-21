//! meldr-core: Pure merge logic engine
//!
//! This crate contains the core domain model and merge logic for meldr.
//! It is intentionally pure and has no dependencies on:
//! - Filesystem operations
//! - Git commands
//! - UI/terminal rendering
//! - Network/AI providers
//!
//! All merge decisions are explicit and deterministic.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod error;
mod hunk;
mod input;
mod resolution;
mod result;
mod session;

pub use error::*;
pub use hunk::*;
pub use input::*;
pub use resolution::*;
pub use result::*;
pub use session::*;
