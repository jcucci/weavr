//! Jujutsu (jj) integration for weavr.
//!
//! This crate provides Jujutsu repository operations for weavr:
//! - Discovering repositories from any subdirectory
//! - Detecting conflicted files from `jj status` output
//! - Reading conflict marker style configuration
//!
//! # Example
//!
//! ```no_run
//! use weavr_jj::JjRepo;
//!
//! let repo = JjRepo::discover()?;
//!
//! for file in repo.conflicted_files()? {
//!     println!("Conflict: {}", file.display());
//! }
//! # Ok::<(), weavr_jj::JjError>(())
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod config;
mod error;
mod repo;
mod status;

pub use config::{conflict_marker_style, parse_marker_style, ConflictMarkerStyle};
pub use error::JjError;
pub use repo::JjRepo;

// Re-export VcsBackend types for convenience.
pub use weavr_vcs::{ConflictKind, ConflictedFile, VcsBackend, VcsError, VcsOperation};
