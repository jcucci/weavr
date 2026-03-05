//! weavr-vcs: VCS abstraction layer for backend-agnostic version control operations.
//!
//! This crate defines the [`VcsBackend`] trait and supporting types that allow
//! weavr's CLI and TUI to work with any version control system (Git, Jujutsu, etc.)
//! through a common interface.
//!
//! # Architecture
//!
//! - [`VcsBackend`] — trait that VCS backends implement
//! - [`VcsOperation`] — the type of operation in progress (merge, rebase, etc.)
//! - [`ConflictedFile`] — a file with a detected conflict and its [`ConflictKind`]
//! - [`VcsError`] — error type for VCS operations
//!
//! This crate contains **only trait definitions and types** — no I/O, no concrete
//! implementations. Backend crates (e.g., `weavr-git`, `weavr-jj`) provide the
//! implementations.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod backend;
mod error;
mod types;

pub use backend::VcsBackend;
pub use error::VcsError;
pub use types::{ConflictKind, ConflictedFile, VcsOperation};
