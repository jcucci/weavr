//! Git integration for weavr.
//!
//! This crate provides Git repository operations for weavr:
//! - Discovering repositories from any subdirectory
//! - Detecting conflicted files during merge/rebase/cherry-pick
//! - Staging resolved files
//! - Detecting the current Git operation state
//!
//! # Example
//!
//! ```no_run
//! use weavr_git::GitRepo;
//!
//! let repo = GitRepo::discover()?;
//!
//! if repo.is_in_merge() || repo.is_in_rebase() {
//!     for path in repo.conflicted_files()? {
//!         println!("Conflict: {}", path.display());
//!     }
//! }
//! # Ok::<(), weavr_git::GitError>(())
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod error;
mod porcelain;
mod repo;
mod state;

pub use error::GitError;
pub use porcelain::{ConflictEntry, ConflictType};
pub use repo::GitRepo;
pub use state::GitOperation;
