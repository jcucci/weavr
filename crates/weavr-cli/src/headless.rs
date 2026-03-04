//! Headless mode implementation.

use std::path::{Path, PathBuf};

use crate::cli::Strategy;
use crate::error::CliError;

/// Result of headless processing for a single file.
pub struct HeadlessResult {
    /// Path to the processed file.
    pub path: PathBuf,
    /// Number of hunks that were resolved.
    pub hunks_resolved: usize,
    /// The merged output content.
    pub output: String,
}

/// Optional AST strategy handle for headless mode.
///
/// When the `ast` feature is enabled, this wraps an `Option<&AstStrategy>`.
/// When disabled, it's a zero-size type that always returns `None`.
pub struct AstHandle<'a> {
    #[cfg(feature = "ast")]
    inner: Option<&'a weavr_ast::AstStrategy>,
    #[cfg(not(feature = "ast"))]
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl AstHandle<'_> {
    /// Creates a handle with no AST strategy.
    #[must_use]
    #[allow(dead_code)]
    pub fn none() -> Self {
        Self {
            #[cfg(feature = "ast")]
            inner: None,
            #[cfg(not(feature = "ast"))]
            _phantom: std::marker::PhantomData,
        }
    }
}

#[cfg(feature = "ast")]
#[allow(clippy::elidable_lifetime_names)]
impl<'a> AstHandle<'a> {
    /// Creates a handle wrapping an AST strategy.
    #[must_use]
    pub fn some(strategy: &'a weavr_ast::AstStrategy) -> Self {
        Self {
            inner: Some(strategy),
        }
    }
}

/// Runs headless merge on a single file.
pub fn process_file(
    path: &Path,
    strategy: Strategy,
    dedupe: bool,
    ast: &AstHandle<'_>,
) -> Result<HeadlessResult, CliError> {
    let content = std::fs::read_to_string(path)?;
    let mut session = weavr_core::MergeSession::from_conflicted(&content, path.to_path_buf())?;

    let hunks: Vec<_> = session.hunks().to_vec();

    // Handle files without conflicts (already clean)
    if hunks.is_empty() {
        return Ok(HeadlessResult {
            path: path.to_path_buf(),
            hunks_resolved: 0,
            output: content,
        });
    }

    for hunk in &hunks {
        let resolution = match strategy {
            Strategy::Left => weavr_core::Resolution::accept_left(hunk),
            Strategy::Right => weavr_core::Resolution::accept_right(hunk),
            Strategy::Both => {
                let options = weavr_core::AcceptBothOptions {
                    order: weavr_core::BothOrder::LeftThenRight,
                    deduplicate: dedupe,
                    trim_whitespace: false,
                };
                weavr_core::Resolution::accept_both(hunk, &options)
            }
            Strategy::Ast => try_ast_resolve(path, hunk, ast),
        };

        session.set_resolution(hunk.id, resolution)?;
    }

    session.apply()?;
    session.validate()?;
    let result = session.complete()?;

    Ok(HeadlessResult {
        path: path.to_path_buf(),
        hunks_resolved: result.summary.resolved_hunks,
        output: result.content,
    })
}

/// Attempts AST resolution, falling back to accept-left.
#[cfg(feature = "ast")]
fn try_ast_resolve(
    path: &Path,
    hunk: &weavr_core::ConflictHunk,
    ast: &AstHandle<'_>,
) -> weavr_core::Resolution {
    if let Some(strategy) = ast.inner {
        let language = weavr_core::detect_language(path);
        if let Ok(Some(resolution)) = strategy.try_resolve(hunk, path, language) {
            return resolution;
        }
    }
    // Fallback: accept left
    weavr_core::Resolution::accept_left(hunk)
}

/// Stub when `ast` feature is disabled — always falls back to accept-left.
#[cfg(not(feature = "ast"))]
fn try_ast_resolve(
    _path: &Path,
    hunk: &weavr_core::ConflictHunk,
    _ast: &AstHandle<'_>,
) -> weavr_core::Resolution {
    weavr_core::Resolution::accept_left(hunk)
}

/// Writes the result to the file or prints it for dry-run.
pub fn write_or_print(result: &HeadlessResult, dry_run: bool) -> Result<(), CliError> {
    if dry_run {
        println!("=== {} ===", result.path.display());
        print!("{}", result.output);
    } else {
        std::fs::write(&result.path, &result.output)?;
        println!(
            "{}: {} hunks resolved",
            result.path.display(),
            result.hunks_resolved
        );
    }
    Ok(())
}
