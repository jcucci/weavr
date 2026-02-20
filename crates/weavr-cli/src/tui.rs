//! TUI mode implementation.

use std::path::Path;

use weavr_core::MergeSession;
use weavr_tui::App;

use crate::config::WeavrConfig;
use crate::error::CliError;

/// Result of TUI processing for a single file.
pub struct TuiResult {
    /// The resolved content (if fully resolved and saved).
    pub content: Option<String>,
    /// Number of hunks that were resolved.
    pub hunks_resolved: usize,
    /// Total number of hunks in the file.
    pub total_hunks: usize,
}

/// Runs the TUI for a single file.
///
/// Returns the resolution result after the user quits the TUI.
pub fn process_file(path: &Path, config: &WeavrConfig) -> Result<TuiResult, CliError> {
    let content = std::fs::read_to_string(path)?;
    let session = MergeSession::from_conflicted(&content, path.to_path_buf())?;

    // Handle files without conflicts (already clean)
    if session.hunks().is_empty() {
        return Ok(TuiResult {
            content: Some(content),
            hunks_resolved: 0,
            total_hunks: 0,
        });
    }

    let total_hunks = session.hunks().len();

    // Create and configure App
    let mut app = App::with_theme(config.theme);
    app.set_session(session);

    // Wire up AI if configured
    #[cfg(feature = "ai")]
    if let Some(handle) = spawn_ai_worker(&config.ai) {
        app.set_ai_handle(handle);
    }

    // Run TUI event loop
    weavr_tui::run(&mut app)?;

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
            content: Some(result.content),
            hunks_resolved: result.summary.resolved_hunks,
            total_hunks,
        })
    } else {
        // User quit without resolving all hunks
        Ok(TuiResult {
            content: None,
            hunks_resolved: resolved_count,
            total_hunks,
        })
    }
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
