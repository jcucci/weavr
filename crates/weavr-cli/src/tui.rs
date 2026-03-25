//! TUI mode implementation.

use std::path::{Path, PathBuf};

use weavr_core::MergeSession;
use weavr_tui::App;

use crate::config::{RawKeybindingsConfig, WeavrConfig};
use crate::error::CliError;

/// Result of TUI processing for a single file.
pub struct TuiResult {
    /// Path to the file.
    pub path: PathBuf,
    /// The resolved content (if fully resolved and saved).
    pub content: Option<String>,
    /// Number of hunks that were resolved.
    pub hunks_resolved: usize,
    /// Total number of hunks in the file.
    pub total_hunks: usize,
    /// Whether the user requested staging (via `:wa` or staging prompt).
    pub stage_requested: bool,
    /// Whether this is a partial write (some hunks still have conflict markers).
    pub is_partial: bool,
}

/// Runs the TUI for a single file.
///
/// Returns the resolution result after the user quits the TUI.
pub fn process_file(
    path: &Path,
    config: &WeavrConfig,
    keybindings_config: Option<&RawKeybindingsConfig>,
) -> Result<TuiResult, CliError> {
    let content = std::fs::read_to_string(path)?;
    let session = MergeSession::from_conflicted(&content, path.to_path_buf())?;

    // Handle files without conflicts (already clean)
    if session.hunks().is_empty() {
        return Ok(TuiResult {
            path: path.to_path_buf(),
            content: Some(content),
            hunks_resolved: 0,
            total_hunks: 0,
            stage_requested: false,
            is_partial: false,
        });
    }

    let total_hunks = session.hunks().len();

    // Create and configure App
    let mut app = App::with_theme(config.theme.clone());
    app.set_session(session);

    // Wire up custom keybindings if configured
    if let Some(kb_config) = keybindings_config {
        let overrides = kb_config.clone().into_key_lists();
        match weavr_tui::keybindings::build_from_config(&overrides) {
            Ok((map, warnings)) => {
                for w in &warnings {
                    eprintln!("weavr: {w}");
                }
                app.set_keybindings(map);
            }
            Err(e) => {
                eprintln!("weavr: keybinding config error: {e}");
                eprintln!("       Falling back to default keybindings.");
            }
        }
    }

    // Wire up staging prompt from config
    app.set_stage_prompt(config.stage_prompt);

    // Wire up AI if configured
    #[cfg(feature = "ai")]
    if let Some(handle) = spawn_ai_worker(&config.ai) {
        app.set_ai_handle(handle);
    }

    // Wire up AST strategy if configured
    #[cfg(feature = "ast")]
    {
        let ast_strategy = build_ast_strategy(&config.ast);
        app.set_ast_strategy(ast_strategy);
    }

    // Run TUI event loop
    weavr_tui::run(&mut app)?;

    // Extract preferences before taking session
    let stage_requested = app.stage_requested();
    let partial_write = app.partial_write_requested();

    // Extract session and check resolution state
    let session = app
        .take_session()
        .ok_or_else(|| std::io::Error::other("merge session unexpectedly missing after TUI run"))?;
    let resolved_count = session
        .hunks()
        .iter()
        .filter(|h| matches!(h.state, weavr_core::HunkState::Resolved(_)))
        .count();

    if session.is_fully_resolved() {
        // Complete the lifecycle to get the merged content
        let mut session = session;
        session.apply()?;
        session.validate()?;
        let result = session.complete()?;

        Ok(TuiResult {
            path: path.to_path_buf(),
            content: Some(result.content),
            hunks_resolved: result.summary.resolved_hunks,
            total_hunks,
            stage_requested,
            is_partial: false,
        })
    } else if partial_write {
        // Partial write — use apply_partial to preserve unresolved markers
        let result = session.apply_partial()?;

        Ok(TuiResult {
            path: path.to_path_buf(),
            content: Some(result.content),
            hunks_resolved: result.summary.resolved_hunks,
            total_hunks,
            stage_requested: false,
            is_partial: true,
        })
    } else {
        // User quit without resolving all hunks
        Ok(TuiResult {
            path: path.to_path_buf(),
            content: None,
            hunks_resolved: resolved_count,
            total_hunks,
            stage_requested: false,
            is_partial: false,
        })
    }
}

/// Runs the TUI for multiple files in a single session.
///
/// Files without conflicts are returned immediately as resolved `TuiResult`s.
/// If only one file has conflicts, delegates to `process_file`.
/// Otherwise, creates a multi-file workspace for the TUI.
#[allow(clippy::too_many_lines)]
pub fn process_files(
    paths: &[PathBuf],
    config: &WeavrConfig,
    keybindings_config: Option<&RawKeybindingsConfig>,
) -> Result<Vec<TuiResult>, CliError> {
    use weavr_tui::workspace::{FileState, Workspace};

    let mut results: Vec<TuiResult> = Vec::new();
    let mut conflicted: Vec<(PathBuf, MergeSession)> = Vec::new();

    // Phase 1: Read all files, separate clean from conflicted
    for path in paths {
        let content = std::fs::read_to_string(path)?;
        let session = MergeSession::from_conflicted(&content, path.clone())?;

        if session.hunks().is_empty() {
            // Clean file — no TUI needed
            results.push(TuiResult {
                path: path.clone(),
                content: Some(content),
                hunks_resolved: 0,
                total_hunks: 0,
                stage_requested: false,
                is_partial: false,
            });
        } else {
            conflicted.push((path.clone(), session));
        }
    }

    // Phase 2: If 0 conflicted files, return clean results only
    if conflicted.is_empty() {
        return Ok(results);
    }

    // Phase 3: If only 1 conflicted file, delegate to existing single-file path
    if conflicted.len() == 1 {
        let (path, _session) = conflicted.into_iter().next().unwrap();
        let result = process_file(&path, config, keybindings_config)?;
        results.push(result);
        return Ok(results);
    }

    // Phase 4: Build workspace for multi-file TUI
    let file_states: Vec<FileState> = conflicted
        .into_iter()
        .map(|(path, session)| FileState::new(path, session))
        .collect();

    let workspace = Workspace::new(file_states);

    // Create and configure App
    let mut app = App::with_theme(config.theme.clone());

    // Wire up custom keybindings if configured
    if let Some(kb_config) = keybindings_config {
        let overrides = kb_config.clone().into_key_lists();
        match weavr_tui::keybindings::build_from_config(&overrides) {
            Ok((map, warnings)) => {
                for w in &warnings {
                    eprintln!("weavr: {w}");
                }
                app.set_keybindings(map);
            }
            Err(e) => {
                eprintln!("weavr: keybinding config error: {e}");
                eprintln!("       Falling back to default keybindings.");
            }
        }
    }

    // Wire up staging prompt from config
    app.set_stage_prompt(config.stage_prompt);

    // Wire up AI if configured
    #[cfg(feature = "ai")]
    if let Some(handle) = spawn_ai_worker(&config.ai) {
        app.set_ai_handle(handle);
    }

    // Wire up AST strategy if configured
    #[cfg(feature = "ast")]
    {
        let ast_strategy = build_ast_strategy(&config.ast);
        app.set_ast_strategy(ast_strategy);
    }

    // Set workspace and run TUI
    app.set_workspace(workspace);
    weavr_tui::run(&mut app)?;

    // Phase 5: Extract workspace and build results
    if let Some(mut workspace) = app.take_workspace() {
        for file_state in workspace.files_mut().drain(..) {
            let total_hunks = file_state.session.hunks().len();
            let resolved_count = file_state
                .session
                .hunks()
                .iter()
                .filter(|h| matches!(h.state, weavr_core::HunkState::Resolved(_)))
                .count();

            if file_state.written && file_state.session.is_fully_resolved() {
                // Complete the lifecycle
                let mut session = file_state.session;
                session.apply()?;
                session.validate()?;
                let merge_result = session.complete()?;

                results.push(TuiResult {
                    path: file_state.path,
                    content: Some(merge_result.content),
                    hunks_resolved: merge_result.summary.resolved_hunks,
                    total_hunks,
                    stage_requested: file_state.stage_requested,
                    is_partial: false,
                });
            } else if file_state.written && file_state.partial {
                // Partial write — use apply_partial
                let result = file_state.session.apply_partial()?;

                results.push(TuiResult {
                    path: file_state.path,
                    content: Some(result.content),
                    hunks_resolved: result.summary.resolved_hunks,
                    total_hunks,
                    stage_requested: false,
                    is_partial: true,
                });
            } else {
                // User didn't complete this file
                results.push(TuiResult {
                    path: file_state.path,
                    content: None,
                    hunks_resolved: resolved_count,
                    total_hunks,
                    stage_requested: false,
                    is_partial: false,
                });
            }
        }
    }

    Ok(results)
}

// ---------------------------------------------------------------------------
// AST strategy builder (feature-gated)
// ---------------------------------------------------------------------------

/// Builds an `AstStrategy` with all available language mergers.
#[cfg(feature = "ast")]
#[allow(clippy::vec_init_then_push)] // pushes are conditionally compiled per language feature
pub(crate) fn build_ast_strategy(config: &weavr_ast::AstConfig) -> weavr_ast::AstStrategy {
    let mut mergers: Vec<Box<dyn weavr_ast::AstMerger>> = Vec::new();

    #[cfg(feature = "ast-rust")]
    mergers.push(Box::new(weavr_ast::mergers::rust_merger::RustMerger::new()));

    #[cfg(feature = "ast-csharp")]
    mergers.push(Box::new(
        weavr_ast::mergers::csharp_merger::CSharpMerger::new(),
    ));

    #[cfg(feature = "ast-typescript")]
    mergers.push(Box::new(
        weavr_ast::mergers::typescript_merger::TypeScriptMerger::new(),
    ));

    #[cfg(feature = "ast-go")]
    mergers.push(Box::new(weavr_ast::mergers::go_merger::GoMerger::new()));

    weavr_ast::AstStrategy::new(mergers, config.clone())
}

// ---------------------------------------------------------------------------
// AI background worker (feature-gated)
// ---------------------------------------------------------------------------

/// Spawns the AI background worker and returns an `AiHandle`.
///
/// Returns `None` if the provider cannot be initialized (e.g., missing API key).
#[cfg(feature = "ai")]
fn spawn_ai_worker(ai_config: &weavr_ai::AiConfig) -> Option<weavr_tui::ai::AiHandle> {
    use std::sync::mpsc;
    use weavr_tui::ai::{AiCommand, AiEvent, AiHandle};

    let config = build_ai_config(ai_config);
    if !config.enabled {
        return None;
    }

    let strategy = build_ai_strategy(&config)?;

    let (cmd_tx, cmd_rx) = mpsc::channel::<AiCommand>();
    let (evt_tx, evt_rx) = mpsc::channel::<AiEvent>();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to create tokio runtime for AI worker");

        rt.block_on(async move {
            ai_worker_loop(strategy, cmd_rx, evt_tx).await;
        });
    });

    Some(AiHandle::new(cmd_tx, evt_rx))
}

/// Builds `AiConfig` starting from the config file values, then layering
/// env-var auto-detection for fields that weren't explicitly set.
#[cfg(feature = "ai")]
fn build_ai_config(base: &weavr_ai::AiConfig) -> weavr_ai::AiConfig {
    let mut config = base.clone();

    // Auto-detect provider from env vars if not set in config
    if config.provider.is_none() {
        #[cfg(feature = "ai-claude")]
        if std::env::var("ANTHROPIC_API_KEY").is_ok() {
            config.enabled = true;
            config.provider = Some("claude".into());
        }

        #[cfg(feature = "ai-openai")]
        if !config.enabled && std::env::var("OPENAI_API_KEY").is_ok() {
            config.enabled = true;
            config.provider = Some("openai".into());
        }
    }

    config
}

/// Builds an `AiStrategy` from the given configuration.
#[cfg(feature = "ai")]
fn build_ai_strategy(config: &weavr_ai::AiConfig) -> Option<weavr_ai::AiStrategy> {
    let provider_name = config.provider.as_deref().unwrap_or("claude");
    let provider: Box<dyn weavr_ai::AiProvider> = match provider_name {
        #[cfg(feature = "ai-claude")]
        "claude" => {
            match weavr_ai::providers::ClaudeProvider::with_timeout(&config.claude, config.timeout)
            {
                Ok(p) => Box::new(p),
                Err(e) => {
                    eprintln!("weavr: AI provider error: {e}");
                    return None;
                }
            }
        }
        #[cfg(feature = "ai-openai")]
        "openai" => match weavr_ai::providers::OpenAiProvider::new(&config.openai) {
            Ok(p) => Box::new(p),
            Err(e) => {
                eprintln!("weavr: AI provider error: {e}");
                return None;
            }
        },
        other => {
            eprintln!("weavr: unknown AI provider '{other}'");
            return None;
        }
    };

    Some(weavr_ai::AiStrategy::new(provider, config.clone()))
}

/// Main loop for the AI background worker.
#[cfg(feature = "ai")]
async fn ai_worker_loop(
    strategy: weavr_ai::AiStrategy,
    cmd_rx: std::sync::mpsc::Receiver<weavr_tui::ai::AiCommand>,
    evt_tx: std::sync::mpsc::Sender<weavr_tui::ai::AiEvent>,
) {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use weavr_tui::ai::{AiCommand, AiEvent};

    let cancelled = Arc::new(AtomicBool::new(false));

    while let Ok(cmd) = cmd_rx.recv() {
        match cmd {
            AiCommand::Shutdown => break,

            AiCommand::Cancel { .. } => {
                cancelled.store(true, Ordering::Relaxed);
            }

            AiCommand::Suggest { hunk_id, hunk } => {
                cancelled.store(false, Ordering::Relaxed);
                match strategy.suggest(&hunk).await {
                    Ok(Some(resolution)) => {
                        if !cancelled.load(Ordering::Relaxed) {
                            let confidence = resolution.metadata.confidence;
                            let _ = evt_tx.send(AiEvent::Suggestion {
                                hunk_id,
                                resolution,
                                confidence,
                            });
                        }
                    }
                    Ok(None) => {
                        if !cancelled.load(Ordering::Relaxed) {
                            let _ = evt_tx.send(AiEvent::NoSuggestion {
                                hunk_id,
                                reason: "Provider declined to suggest".into(),
                            });
                        }
                    }
                    Err(e) => {
                        let _ = evt_tx.send(AiEvent::Error {
                            hunk_id,
                            message: e.to_string(),
                        });
                    }
                }
            }

            AiCommand::SuggestAll { hunks } => {
                cancelled.store(false, Ordering::Relaxed);
                for (hunk_id, hunk) in hunks {
                    if cancelled.load(Ordering::Relaxed) {
                        break;
                    }
                    match strategy.suggest(&hunk).await {
                        Ok(Some(resolution)) => {
                            let confidence = resolution.metadata.confidence;
                            let _ = evt_tx.send(AiEvent::Suggestion {
                                hunk_id,
                                resolution,
                                confidence,
                            });
                        }
                        Ok(None) => {
                            let _ = evt_tx.send(AiEvent::NoSuggestion {
                                hunk_id,
                                reason: "Provider declined".into(),
                            });
                        }
                        Err(e) => {
                            let _ = evt_tx.send(AiEvent::Error {
                                hunk_id,
                                message: e.to_string(),
                            });
                        }
                    }
                }
                let _ = evt_tx.send(AiEvent::BatchComplete);
            }

            AiCommand::Explain { hunk_id, hunk } => {
                cancelled.store(false, Ordering::Relaxed);
                match strategy.explain(&hunk).await {
                    Ok(Some(text)) => {
                        if !cancelled.load(Ordering::Relaxed) {
                            let _ = evt_tx.send(AiEvent::Explanation { hunk_id, text });
                        }
                    }
                    Ok(None) => {
                        if !cancelled.load(Ordering::Relaxed) {
                            let _ = evt_tx.send(AiEvent::NoSuggestion {
                                hunk_id,
                                reason: "No explanation available".into(),
                            });
                        }
                    }
                    Err(e) => {
                        let _ = evt_tx.send(AiEvent::Error {
                            hunk_id,
                            message: e.to_string(),
                        });
                    }
                }
            }
        }
    }
}
