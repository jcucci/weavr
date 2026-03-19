//! Headless mode implementation.

use std::path::{Path, PathBuf};

use crate::cli::{FallbackStrategy, Strategy};
use crate::error::CliError;

/// Metadata for a single hunk resolution, regardless of strategy.
pub struct HunkMeta {
    pub hunk_id: u32,
    pub strategy: String,
    pub provider: Option<String>,
    pub confidence: Option<u8>,
    pub explanation: Option<String>,
    pub note: Option<String>,
}

/// Result of headless processing for a single file.
pub struct HeadlessResult {
    /// Path to the processed file.
    pub path: PathBuf,
    /// Number of hunks that were resolved.
    pub hunks_resolved: usize,
    /// The merged output content.
    pub output: String,
    /// Per-hunk resolution metadata.
    pub hunk_metadata: Vec<HunkMeta>,
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

/// Optional AI strategy handle for headless mode.
///
/// When the `ai` feature is enabled, this wraps an `Option` containing
/// the AI strategy and a tokio runtime for blocking calls.
/// When disabled, it's a zero-size type.
pub struct AiHandle<'a> {
    #[cfg(feature = "ai")]
    inner: Option<(&'a weavr_ai::AiStrategy, &'a tokio::runtime::Runtime)>,
    #[cfg(not(feature = "ai"))]
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl AiHandle<'_> {
    /// Creates a handle with no AI strategy.
    #[must_use]
    #[allow(dead_code)]
    pub fn none() -> Self {
        Self {
            #[cfg(feature = "ai")]
            inner: None,
            #[cfg(not(feature = "ai"))]
            _phantom: std::marker::PhantomData,
        }
    }
}

#[cfg(feature = "ai")]
#[allow(clippy::elidable_lifetime_names)]
impl<'a> AiHandle<'a> {
    /// Creates a handle wrapping an AI strategy and runtime.
    #[must_use]
    pub fn some(strategy: &'a weavr_ai::AiStrategy, runtime: &'a tokio::runtime::Runtime) -> Self {
        Self {
            inner: Some((strategy, runtime)),
        }
    }

    /// Synchronously calls the AI strategy's suggest method.
    pub fn suggest_blocking(
        &self,
        hunk: &weavr_core::ConflictHunk,
    ) -> Result<Option<weavr_core::Resolution>, weavr_ai::AiError> {
        match self.inner {
            Some((strategy, runtime)) => runtime.block_on(strategy.suggest(hunk)),
            None => Ok(None),
        }
    }

    /// Returns the provider name, if available.
    pub fn provider_name(&self) -> &str {
        match self.inner {
            Some((strategy, _)) => strategy.provider_name(),
            None => "none",
        }
    }
}

/// Runs headless merge on a single file.
pub fn process_file(
    path: &Path,
    strategy: Strategy,
    dedupe: bool,
    ast: &AstHandle<'_>,
    ai: &AiHandle<'_>,
    fallback_strategy: Option<FallbackStrategy>,
    fail_on_ambiguous: bool,
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
            hunk_metadata: Vec::new(),
        });
    }

    let mut hunk_metadata = Vec::new();

    for hunk in &hunks {
        let resolution = match strategy {
            Strategy::Left => {
                hunk_metadata.push(HunkMeta {
                    hunk_id: hunk.id.0,
                    strategy: "left".into(),
                    provider: None,
                    confidence: None,
                    explanation: None,
                    note: None,
                });
                weavr_core::Resolution::accept_left(hunk)
            }
            Strategy::Right => {
                hunk_metadata.push(HunkMeta {
                    hunk_id: hunk.id.0,
                    strategy: "right".into(),
                    provider: None,
                    confidence: None,
                    explanation: None,
                    note: None,
                });
                weavr_core::Resolution::accept_right(hunk)
            }
            Strategy::Both => {
                hunk_metadata.push(HunkMeta {
                    hunk_id: hunk.id.0,
                    strategy: "both".into(),
                    provider: None,
                    confidence: None,
                    explanation: None,
                    note: None,
                });
                let options = weavr_core::AcceptBothOptions {
                    order: weavr_core::BothOrder::LeftThenRight,
                    deduplicate: dedupe,
                    trim_whitespace: false,
                };
                weavr_core::Resolution::accept_both(hunk, &options)
            }
            Strategy::Ast => {
                let (resolution, meta) = try_ast_resolve(path, hunk, ast);
                hunk_metadata.push(meta);
                resolution
            }
            Strategy::Ai => try_ai_resolve(
                hunk,
                ai,
                fallback_strategy,
                dedupe,
                fail_on_ambiguous,
                &mut hunk_metadata,
            )?,
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
        hunk_metadata,
    })
}

/// Attempts AI resolution with fallback support.
#[cfg(feature = "ai")]
pub(crate) fn try_ai_resolve(
    hunk: &weavr_core::ConflictHunk,
    ai: &AiHandle<'_>,
    fallback_strategy: Option<FallbackStrategy>,
    dedupe: bool,
    fail_on_ambiguous: bool,
    metadata: &mut Vec<HunkMeta>,
) -> Result<weavr_core::Resolution, CliError> {
    let provider_name = ai.provider_name().to_string();

    match ai.suggest_blocking(hunk) {
        Ok(Some(resolution)) => {
            metadata.push(HunkMeta {
                hunk_id: hunk.id.0,
                strategy: "ai_suggested".into(),
                provider: Some(provider_name),
                confidence: resolution.metadata.confidence,
                explanation: resolution.metadata.notes.clone(),
                note: None,
            });
            Ok(resolution)
        }
        Ok(None) => {
            // AI declined (below confidence threshold)
            apply_fallback(
                hunk,
                &provider_name,
                fallback_strategy,
                dedupe,
                fail_on_ambiguous,
                metadata,
            )
        }
        Err(e) => {
            // AI errored — try fallback or propagate
            if let Some(_fallback) = fallback_strategy {
                eprintln!("weavr: AI error for hunk {}, falling back: {e}", hunk.id.0);
                apply_fallback(
                    hunk,
                    &provider_name,
                    fallback_strategy,
                    dedupe,
                    fail_on_ambiguous,
                    metadata,
                )
            } else {
                Err(CliError::Ai(e))
            }
        }
    }
}

/// Applies the fallback strategy for a hunk the AI could not resolve.
#[cfg(feature = "ai")]
fn apply_fallback(
    hunk: &weavr_core::ConflictHunk,
    provider_name: &str,
    fallback_strategy: Option<FallbackStrategy>,
    dedupe: bool,
    fail_on_ambiguous: bool,
    metadata: &mut Vec<HunkMeta>,
) -> Result<weavr_core::Resolution, CliError> {
    match fallback_strategy {
        Some(fallback) => {
            let (resolution, fallback_name) = match fallback {
                FallbackStrategy::Left => (weavr_core::Resolution::accept_left(hunk), "left"),
                FallbackStrategy::Right => (weavr_core::Resolution::accept_right(hunk), "right"),
                FallbackStrategy::Both => {
                    let options = weavr_core::AcceptBothOptions {
                        order: weavr_core::BothOrder::LeftThenRight,
                        deduplicate: dedupe,
                        trim_whitespace: false,
                    };
                    (
                        weavr_core::Resolution::accept_both(hunk, &options),
                        "both",
                    )
                }
            };
            metadata.push(HunkMeta {
                hunk_id: hunk.id.0,
                strategy: fallback_name.into(),
                provider: Some(provider_name.to_string()),
                confidence: None,
                explanation: None,
                note: Some("AI declined, used fallback".into()),
            });
            Ok(resolution)
        }
        None => {
            if fail_on_ambiguous {
                Err(CliError::AmbiguousHunks(1))
            } else {
                // Default fallback: accept left
                metadata.push(HunkMeta {
                    hunk_id: hunk.id.0,
                    strategy: "left".into(),
                    provider: Some(provider_name.to_string()),
                    confidence: None,
                    explanation: None,
                    note: Some("AI declined, used fallback".into()),
                });
                Ok(weavr_core::Resolution::accept_left(hunk))
            }
        }
    }
}

/// Stub when `ai` feature is disabled — returns a clear error.
#[cfg(not(feature = "ai"))]
pub(crate) fn try_ai_resolve(
    _hunk: &weavr_core::ConflictHunk,
    _ai: &AiHandle<'_>,
    _fallback_strategy: Option<FallbackStrategy>,
    _dedupe: bool,
    _fail_on_ambiguous: bool,
    _metadata: &mut Vec<HunkMeta>,
) -> Result<weavr_core::Resolution, CliError> {
    Err(CliError::InvalidArgs(
        "--strategy=ai requires the 'ai' feature (compile with --features ai-claude, ai-openai, or ai-local)".into(),
    ))
}

/// Attempts AST resolution, falling back to accept-left.
#[cfg(feature = "ast")]
fn try_ast_resolve(
    path: &Path,
    hunk: &weavr_core::ConflictHunk,
    ast: &AstHandle<'_>,
) -> (weavr_core::Resolution, HunkMeta) {
    if let Some(strategy) = ast.inner {
        let language = weavr_core::detect_language(path);
        if let Ok(Some(resolution)) = strategy.try_resolve(hunk, path, language) {
            let meta = HunkMeta {
                hunk_id: hunk.id.0,
                strategy: "ast_merged".into(),
                provider: None,
                confidence: resolution.metadata.confidence,
                explanation: resolution.metadata.notes.clone(),
                note: None,
            };
            return (resolution, meta);
        }
    }
    // Fallback: accept left
    let meta = HunkMeta {
        hunk_id: hunk.id.0,
        strategy: "left".into(),
        provider: None,
        confidence: None,
        explanation: None,
        note: Some("AST unavailable, used fallback".into()),
    };
    (weavr_core::Resolution::accept_left(hunk), meta)
}

/// Stub when `ast` feature is disabled — always falls back to accept-left.
#[cfg(not(feature = "ast"))]
fn try_ast_resolve(
    _path: &Path,
    hunk: &weavr_core::ConflictHunk,
    _ast: &AstHandle<'_>,
) -> (weavr_core::Resolution, HunkMeta) {
    let meta = HunkMeta {
        hunk_id: hunk.id.0,
        strategy: "left".into(),
        provider: None,
        confidence: None,
        explanation: None,
        note: Some("AST unavailable, used fallback".into()),
    };
    (weavr_core::Resolution::accept_left(hunk), meta)
}

impl From<&HunkMeta> for crate::output::JsonHunkResult {
    fn from(m: &HunkMeta) -> Self {
        Self {
            hunk_id: m.hunk_id,
            strategy: m.strategy.clone(),
            provider: m.provider.clone(),
            confidence: m.confidence,
            explanation: m.explanation.clone(),
            note: m.note.clone(),
        }
    }
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
