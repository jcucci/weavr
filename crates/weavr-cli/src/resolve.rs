//! Resolve subcommand — applies per-hunk resolutions from a JSON map.

use std::collections::HashSet;
use std::path::PathBuf;

use serde::Deserialize;

use crate::cli::{OutputFormat, ResolveArgs};
use crate::config::WeavrConfig;
use crate::discovery;
use crate::error::{exit_codes, CliError};
use crate::headless::AiHandle;
use crate::output::{self, JsonAiDetails, JsonAiHunkResult, JsonResolveFile, JsonResolveOutput};

use weavr_core::{AcceptBothOptions, HunkId, MergeSession, Resolution};

/// JSON input: the top-level resolutions wrapper.
#[derive(Debug, Deserialize)]
struct ResolveInput {
    resolutions: Vec<ResolutionEntry>,
}

/// A single per-hunk resolution entry.
#[derive(Debug, Deserialize)]
struct ResolutionEntry {
    hunk_id: u32,
    strategy: ResolutionStrategy,
    #[serde(default)]
    content: Option<String>,
}

/// Strategy names accepted in the JSON input.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ResolutionStrategy {
    Left,
    Right,
    Both,
    Manual,
    Ai,
}

/// Reads the resolutions JSON from a file path or stdin (`-`).
fn read_resolutions(path: &PathBuf) -> Result<ResolveInput, CliError> {
    let json_str = if path.as_os_str() == "-" {
        std::io::read_to_string(std::io::stdin())
            .map_err(|e| CliError::InvalidArgs(format!("failed to read stdin: {e}")))?
    } else {
        std::fs::read_to_string(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                CliError::FileNotFound(path.clone())
            } else {
                CliError::Io(e)
            }
        })?
    };

    serde_json::from_str::<ResolveInput>(&json_str)
        .map_err(|e| CliError::InvalidArgs(format!("invalid resolutions JSON: {e}")))
}

/// Result of resolving a single file.
struct FileResolveResult {
    output_content: String,
    total_hunks: usize,
    resolved_hunks: usize,
    unresolved: Vec<u32>,
    is_fully_resolved: bool,
    ai_hunks: Vec<JsonAiHunkResult>,
}

/// Validates the resolution input against a file's hunks.
fn validate_input(
    input: &ResolveInput,
    hunk_ids: &[HunkId],
    path: &std::path::Path,
    fail_on_ambiguous: bool,
) -> Result<(), CliError> {
    let valid_ids: HashSet<u32> = hunk_ids.iter().map(|id| id.0).collect();

    for entry in &input.resolutions {
        if !valid_ids.contains(&entry.hunk_id) {
            return Err(CliError::InvalidArgs(format!(
                "hunk_id {} does not exist in {}",
                entry.hunk_id,
                path.display()
            )));
        }
    }

    if fail_on_ambiguous {
        let covered: HashSet<u32> = input.resolutions.iter().map(|e| e.hunk_id).collect();
        let missing_count = valid_ids.iter().filter(|id| !covered.contains(id)).count();
        if missing_count > 0 {
            return Err(CliError::AmbiguousHunks(missing_count));
        }
    }

    Ok(())
}

/// Result of building resolutions, including any AI metadata.
struct BuildResult {
    resolutions: Vec<(HunkId, Resolution)>,
    ai_hunks: Vec<JsonAiHunkResult>,
}

/// Builds Resolution objects from the input entries, borrowing hunks immutably.
fn build_resolutions(
    input: &ResolveInput,
    session: &MergeSession,
    both_options: &AcceptBothOptions,
    ai: &AiHandle<'_>,
) -> Result<BuildResult, CliError> {
    let hunks = session.hunks();
    let mut result = Vec::new();
    let mut ai_hunks = Vec::new();

    for entry in &input.resolutions {
        let hunk_id = HunkId(entry.hunk_id);
        let hunk = hunks
            .iter()
            .find(|h| h.id == hunk_id)
            .expect("hunk_id should exist after validate_input");

        let resolution = match entry.strategy {
            ResolutionStrategy::Left => Resolution::accept_left(hunk),
            ResolutionStrategy::Right => Resolution::accept_right(hunk),
            ResolutionStrategy::Both => Resolution::accept_both(hunk, both_options),
            ResolutionStrategy::Manual => {
                let text = entry.content.as_ref().ok_or_else(|| {
                    CliError::InvalidArgs(format!(
                        "hunk_id {}: manual strategy requires \"content\" field",
                        entry.hunk_id
                    ))
                })?;
                Resolution::manual(text.clone())
            }
            ResolutionStrategy::Ai => resolve_ai_hunk(hunk, entry.hunk_id, ai, &mut ai_hunks)?,
        };
        result.push((hunk_id, resolution));
    }
    Ok(BuildResult {
        resolutions: result,
        ai_hunks,
    })
}

/// Attempts AI resolution for a single hunk in the resolve subcommand.
#[cfg(feature = "ai")]
fn resolve_ai_hunk(
    hunk: &weavr_core::ConflictHunk,
    hunk_id: u32,
    ai: &AiHandle<'_>,
    ai_hunks: &mut Vec<JsonAiHunkResult>,
) -> Result<Resolution, CliError> {
    match ai.suggest_blocking(hunk) {
        Ok(Some(resolution)) => {
            ai_hunks.push(JsonAiHunkResult {
                hunk_id,
                provider: ai.provider_name().to_string(),
                confidence: resolution.metadata.confidence,
                explanation: resolution.metadata.notes.clone(),
                used_fallback: false,
            });
            Ok(resolution)
        }
        Ok(None) => Err(CliError::InvalidArgs(format!(
            "hunk_id {hunk_id}: AI declined resolution (confidence below threshold)"
        ))),
        Err(e) => Err(CliError::Ai(e)),
    }
}

/// Stub when `ai` feature is disabled — returns a clear error.
#[cfg(not(feature = "ai"))]
fn resolve_ai_hunk(
    _hunk: &weavr_core::ConflictHunk,
    _hunk_id: u32,
    _ai: &AiHandle<'_>,
    _ai_hunks: &mut Vec<JsonAiHunkResult>,
) -> Result<Resolution, CliError> {
    Err(CliError::InvalidArgs(
        "\"strategy\": \"ai\" requires the 'ai' feature (compile with --features ai-claude)".into(),
    ))
}

/// Processes a single file: validates, applies resolutions, and produces output.
fn resolve_file(
    path: &PathBuf,
    input: &ResolveInput,
    both_options: &AcceptBothOptions,
    fail_on_ambiguous: bool,
    ai: &AiHandle<'_>,
) -> Result<FileResolveResult, CliError> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            CliError::FileNotFound(path.clone())
        } else {
            CliError::Io(e)
        }
    })?;

    let mut session = MergeSession::from_conflicted(&content, path.clone())?;
    let total_hunks = session.hunks().len();
    let hunk_ids: Vec<HunkId> = session.hunks().iter().map(|h| h.id).collect();

    validate_input(input, &hunk_ids, path, fail_on_ambiguous)?;

    let build_result = build_resolutions(input, &session, both_options, ai)?;
    for (hunk_id, resolution) in build_result.resolutions {
        session.set_resolution(hunk_id, resolution)?;
    }

    let is_fully_resolved = session.is_fully_resolved();
    let unresolved: Vec<u32> = session.unresolved_hunks().iter().map(|id| id.0).collect();
    let resolved_hunks = total_hunks - unresolved.len();

    let output_content = if is_fully_resolved {
        let _applied = session.apply()?;
        session.validate()?;
        session.complete()?.content
    } else {
        session.apply_partial()?.content
    };

    Ok(FileResolveResult {
        output_content,
        total_hunks,
        resolved_hunks,
        unresolved,
        is_fully_resolved,
        ai_hunks: build_result.ai_hunks,
    })
}

/// Runs the `resolve` subcommand.
#[allow(clippy::too_many_lines)]
pub fn run(args: &ResolveArgs, config: &WeavrConfig) -> Result<i32, CliError> {
    let input = read_resolutions(&args.resolutions)?;
    let is_json = args.format == OutputFormat::Json;

    let both_options = AcceptBothOptions {
        deduplicate: args.dedupe,
        ..AcceptBothOptions::default()
    };

    // Check if any resolution uses the AI strategy
    let uses_ai = input
        .resolutions
        .iter()
        .any(|e| matches!(e.strategy, ResolutionStrategy::Ai));

    // Build AI handle if needed
    #[cfg(feature = "ai")]
    let ai_runtime;
    #[cfg(feature = "ai")]
    let ai_strategy_obj;
    #[cfg(feature = "ai")]
    let ai_handle = if uses_ai {
        if !config.ai.enabled {
            return Err(CliError::InvalidArgs(
                "\"strategy\": \"ai\" requires [ai] enabled=true in config".into(),
            ));
        }
        ai_runtime = tokio::runtime::Runtime::new()
            .map_err(|e| CliError::InvalidArgs(format!("failed to create async runtime: {e}")))?;
        ai_strategy_obj = crate::build_ai_provider(&config.ai)?;
        AiHandle::some(&ai_strategy_obj, &ai_runtime)
    } else {
        AiHandle::none()
    };
    #[cfg(not(feature = "ai"))]
    let ai_handle = AiHandle::none();
    #[cfg(not(feature = "ai"))]
    let _ = (uses_ai, config);

    let backend = if args.auto_stage && !args.dry_run {
        let b = discovery::discover_backend(args.vcs);
        if b.is_none() {
            eprintln!("weavr: VCS backend not found, staging disabled");
        }
        b
    } else {
        None
    };

    let mut json_results: Vec<JsonResolveFile> = Vec::new();
    let mut any_unresolved = false;

    for path in &args.files {
        let result = resolve_file(
            path,
            &input,
            &both_options,
            args.fail_on_ambiguous,
            &ai_handle,
        )?;

        if !result.is_fully_resolved {
            any_unresolved = true;
        }

        let written = !args.dry_run;

        if args.dry_run {
            if !is_json {
                print!("{}", result.output_content);
            }
        } else {
            std::fs::write(path, &result.output_content)?;
            if !is_json {
                if result.is_fully_resolved {
                    println!(
                        "{}: {} hunks resolved",
                        path.display(),
                        result.resolved_hunks
                    );
                } else {
                    println!(
                        "{}: partially resolved ({}/{} hunks)",
                        path.display(),
                        result.resolved_hunks,
                        result.total_hunks
                    );
                }
            }
        }

        if args.auto_stage && !args.dry_run && result.is_fully_resolved {
            if let Some(ref backend) = backend {
                match backend.stage_file(path) {
                    Ok(()) => {
                        if !is_json {
                            println!("{}: staged", path.display());
                        }
                    }
                    Err(e) => eprintln!("{}: staging failed: {e}", path.display()),
                }
            }
        }

        if is_json {
            let ai_details = if result.ai_hunks.is_empty() {
                None
            } else {
                let provider = result
                    .ai_hunks
                    .first()
                    .map(|h| h.provider.clone())
                    .unwrap_or_default();
                Some(JsonAiDetails {
                    provider,
                    hunks: result.ai_hunks,
                })
            };
            json_results.push(JsonResolveFile {
                path: path.clone(),
                total_hunks: result.total_hunks,
                resolved_hunks: result.resolved_hunks,
                unresolved_hunks: result.unresolved,
                written,
                ai: ai_details,
            });
        }
    }

    if is_json {
        output::print_json(&JsonResolveOutput {
            results: json_results,
        })?;
    }

    Ok(if any_unresolved && args.fail_on_ambiguous {
        exit_codes::UNRESOLVED
    } else {
        exit_codes::SUCCESS
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;
    use tempfile::NamedTempFile;

    fn default_config() -> WeavrConfig {
        WeavrConfig::from_raw(&crate::config::RawConfig::default()).unwrap()
    }

    fn conflict_content() -> &'static str {
        "before\n<<<<<<< HEAD\nfn foo() { 1 }\n=======\nfn foo() { 2 }\n>>>>>>> branch\nafter\n"
    }

    fn two_hunk_content() -> &'static str {
        concat!(
            "before\n",
            "<<<<<<< HEAD\nfn foo() { 1 }\n=======\nfn foo() { 2 }\n>>>>>>> branch\n",
            "middle\n",
            "<<<<<<< HEAD\nfn bar() { 3 }\n=======\nfn bar() { 4 }\n>>>>>>> branch\n",
            "after\n"
        )
    }

    fn make_resolutions_json(entries: &str) -> NamedTempFile {
        let mut tmp = NamedTempFile::new().unwrap();
        write!(tmp, "{{\"resolutions\": [{entries}]}}").unwrap();
        tmp
    }

    #[test]
    fn parse_valid_json_all_strategies() {
        let json = r#"{
            "resolutions": [
                {"hunk_id": 0, "strategy": "left"},
                {"hunk_id": 1, "strategy": "right"},
                {"hunk_id": 2, "strategy": "both"},
                {"hunk_id": 3, "strategy": "manual", "content": "merged"}
            ]
        }"#;
        let input: ResolveInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.resolutions.len(), 4);
        assert!(matches!(
            input.resolutions[0].strategy,
            ResolutionStrategy::Left
        ));
        assert!(matches!(
            input.resolutions[1].strategy,
            ResolutionStrategy::Right
        ));
        assert!(matches!(
            input.resolutions[2].strategy,
            ResolutionStrategy::Both
        ));
        assert!(matches!(
            input.resolutions[3].strategy,
            ResolutionStrategy::Manual
        ));
        assert_eq!(input.resolutions[3].content.as_deref(), Some("merged"));
    }

    #[test]
    fn parse_ai_strategy_json() {
        let json = r#"{
            "resolutions": [
                {"hunk_id": 0, "strategy": "ai"}
            ]
        }"#;
        let input: ResolveInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.resolutions.len(), 1);
        assert!(matches!(
            input.resolutions[0].strategy,
            ResolutionStrategy::Ai
        ));
    }

    #[test]
    fn manual_without_content_errors() {
        let mut conflict = NamedTempFile::new().unwrap();
        write!(conflict, "{}", conflict_content()).unwrap();

        let resolutions = make_resolutions_json(r#"{"hunk_id": 0, "strategy": "manual"}"#);

        let args = ResolveArgs {
            files: vec![conflict.path().to_path_buf()],
            resolutions: resolutions.path().to_path_buf(),
            dry_run: true,
            fail_on_ambiguous: false,
            dedupe: false,
            auto_stage: false,

            vcs: crate::cli::VcsChoice::Auto,
            format: OutputFormat::Text,
        };
        let result = run(&args, &default_config());
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("manual strategy requires"), "{err}");
    }

    #[test]
    fn unknown_hunk_id_errors() {
        let mut conflict = NamedTempFile::new().unwrap();
        write!(conflict, "{}", conflict_content()).unwrap();

        let resolutions = make_resolutions_json(r#"{"hunk_id": 99, "strategy": "left"}"#);

        let args = ResolveArgs {
            files: vec![conflict.path().to_path_buf()],
            resolutions: resolutions.path().to_path_buf(),
            dry_run: true,
            fail_on_ambiguous: false,
            dedupe: false,
            auto_stage: false,

            vcs: crate::cli::VcsChoice::Auto,
            format: OutputFormat::Text,
        };
        let result = run(&args, &default_config());
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("hunk_id 99 does not exist"), "{err}");
    }

    #[test]
    fn fail_on_ambiguous_with_missing_hunks() {
        let mut conflict = NamedTempFile::new().unwrap();
        write!(conflict, "{}", two_hunk_content()).unwrap();

        // Only resolve hunk 0, leave hunk 1 uncovered
        let resolutions = make_resolutions_json(r#"{"hunk_id": 0, "strategy": "left"}"#);

        let args = ResolveArgs {
            files: vec![conflict.path().to_path_buf()],
            resolutions: resolutions.path().to_path_buf(),
            dry_run: true,
            fail_on_ambiguous: true,
            dedupe: false,
            auto_stage: false,

            vcs: crate::cli::VcsChoice::Auto,
            format: OutputFormat::Text,
        };
        let result = run(&args, &default_config());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CliError::AmbiguousHunks(1)));
    }

    #[test]
    fn dry_run_does_not_write_file() {
        let mut conflict = NamedTempFile::new().unwrap();
        write!(conflict, "{}", conflict_content()).unwrap();
        let original = std::fs::read_to_string(conflict.path()).unwrap();

        let resolutions = make_resolutions_json(r#"{"hunk_id": 0, "strategy": "left"}"#);

        let args = ResolveArgs {
            files: vec![conflict.path().to_path_buf()],
            resolutions: resolutions.path().to_path_buf(),
            dry_run: true,
            fail_on_ambiguous: false,
            dedupe: false,
            auto_stage: false,

            vcs: crate::cli::VcsChoice::Auto,
            format: OutputFormat::Text,
        };
        let code = run(&args, &default_config()).unwrap();
        assert_eq!(code, exit_codes::SUCCESS);

        let after = std::fs::read_to_string(conflict.path()).unwrap();
        assert_eq!(original, after, "file should not be modified in dry-run");
    }

    #[test]
    fn resolve_left_writes_file() {
        let mut conflict = NamedTempFile::new().unwrap();
        write!(conflict, "{}", conflict_content()).unwrap();

        let resolutions = make_resolutions_json(r#"{"hunk_id": 0, "strategy": "left"}"#);

        let args = ResolveArgs {
            files: vec![conflict.path().to_path_buf()],
            resolutions: resolutions.path().to_path_buf(),
            dry_run: false,
            fail_on_ambiguous: false,
            dedupe: false,
            auto_stage: false,

            vcs: crate::cli::VcsChoice::Auto,
            format: OutputFormat::Text,
        };
        let code = run(&args, &default_config()).unwrap();
        assert_eq!(code, exit_codes::SUCCESS);

        let result = std::fs::read_to_string(conflict.path()).unwrap();
        assert!(result.contains("fn foo() { 1 }"));
        assert!(!result.contains("<<<<<<<"));
    }

    #[test]
    fn resolve_right_strategy() {
        let mut conflict = NamedTempFile::new().unwrap();
        write!(conflict, "{}", conflict_content()).unwrap();

        let resolutions = make_resolutions_json(r#"{"hunk_id": 0, "strategy": "right"}"#);

        let args = ResolveArgs {
            files: vec![conflict.path().to_path_buf()],
            resolutions: resolutions.path().to_path_buf(),
            dry_run: false,
            fail_on_ambiguous: false,
            dedupe: false,
            auto_stage: false,

            vcs: crate::cli::VcsChoice::Auto,
            format: OutputFormat::Text,
        };
        let code = run(&args, &default_config()).unwrap();
        assert_eq!(code, exit_codes::SUCCESS);

        let result = std::fs::read_to_string(conflict.path()).unwrap();
        assert!(result.contains("fn foo() { 2 }"));
        assert!(!result.contains("<<<<<<<"));
    }

    #[test]
    fn resolve_manual_strategy() {
        let mut conflict = NamedTempFile::new().unwrap();
        write!(conflict, "{}", conflict_content()).unwrap();

        let resolutions = make_resolutions_json(
            r#"{"hunk_id": 0, "strategy": "manual", "content": "fn foo() { merged }"}"#,
        );

        let args = ResolveArgs {
            files: vec![conflict.path().to_path_buf()],
            resolutions: resolutions.path().to_path_buf(),
            dry_run: false,
            fail_on_ambiguous: false,
            dedupe: false,
            auto_stage: false,

            vcs: crate::cli::VcsChoice::Auto,
            format: OutputFormat::Text,
        };
        let code = run(&args, &default_config()).unwrap();
        assert_eq!(code, exit_codes::SUCCESS);

        let result = std::fs::read_to_string(conflict.path()).unwrap();
        assert!(result.contains("fn foo() { merged }"));
    }

    #[test]
    fn resolve_multiple_hunks() {
        let mut conflict = NamedTempFile::new().unwrap();
        write!(conflict, "{}", two_hunk_content()).unwrap();

        let resolutions = make_resolutions_json(
            r#"{"hunk_id": 0, "strategy": "left"}, {"hunk_id": 1, "strategy": "right"}"#,
        );

        let args = ResolveArgs {
            files: vec![conflict.path().to_path_buf()],
            resolutions: resolutions.path().to_path_buf(),
            dry_run: false,
            fail_on_ambiguous: false,
            dedupe: false,
            auto_stage: false,

            vcs: crate::cli::VcsChoice::Auto,
            format: OutputFormat::Text,
        };
        let code = run(&args, &default_config()).unwrap();
        assert_eq!(code, exit_codes::SUCCESS);

        let result = std::fs::read_to_string(conflict.path()).unwrap();
        assert!(result.contains("fn foo() { 1 }"));
        assert!(result.contains("fn bar() { 4 }"));
        assert!(!result.contains("<<<<<<<"));
    }

    #[test]
    fn partial_resolve_without_fail_on_ambiguous() {
        let mut conflict = NamedTempFile::new().unwrap();
        write!(conflict, "{}", two_hunk_content()).unwrap();

        // Only resolve hunk 0
        let resolutions = make_resolutions_json(r#"{"hunk_id": 0, "strategy": "left"}"#);

        let args = ResolveArgs {
            files: vec![conflict.path().to_path_buf()],
            resolutions: resolutions.path().to_path_buf(),
            dry_run: false,
            fail_on_ambiguous: false,
            dedupe: false,
            auto_stage: false,

            vcs: crate::cli::VcsChoice::Auto,
            format: OutputFormat::Text,
        };
        let code = run(&args, &default_config()).unwrap();
        assert_eq!(code, exit_codes::SUCCESS);

        let result = std::fs::read_to_string(conflict.path()).unwrap();
        // Hunk 0 should be resolved
        assert!(result.contains("fn foo() { 1 }"));
        // Hunk 1 should still have conflict markers
        assert!(result.contains("<<<<<<<"));
    }

    #[test]
    fn invalid_json_errors() {
        let mut resolutions = NamedTempFile::new().unwrap();
        write!(resolutions, "not json").unwrap();

        let mut conflict = NamedTempFile::new().unwrap();
        write!(conflict, "{}", conflict_content()).unwrap();

        let args = ResolveArgs {
            files: vec![conflict.path().to_path_buf()],
            resolutions: resolutions.path().to_path_buf(),
            dry_run: true,
            fail_on_ambiguous: false,
            dedupe: false,
            auto_stage: false,

            vcs: crate::cli::VcsChoice::Auto,
            format: OutputFormat::Text,
        };
        let result = run(&args, &default_config());
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("invalid resolutions JSON"), "{err}");
    }

    #[test]
    fn resolutions_file_not_found() {
        let mut conflict = NamedTempFile::new().unwrap();
        write!(conflict, "{}", conflict_content()).unwrap();

        let args = ResolveArgs {
            files: vec![conflict.path().to_path_buf()],
            resolutions: PathBuf::from("/nonexistent/resolutions.json"),
            dry_run: true,
            fail_on_ambiguous: false,
            dedupe: false,
            auto_stage: false,

            vcs: crate::cli::VcsChoice::Auto,
            format: OutputFormat::Text,
        };
        let result = run(&args, &default_config());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CliError::FileNotFound(_)));
    }

    #[test]
    fn json_output_format() {
        let mut conflict = NamedTempFile::new().unwrap();
        write!(conflict, "{}", conflict_content()).unwrap();

        let resolutions = make_resolutions_json(r#"{"hunk_id": 0, "strategy": "left"}"#);

        let args = ResolveArgs {
            files: vec![conflict.path().to_path_buf()],
            resolutions: resolutions.path().to_path_buf(),
            dry_run: true,
            fail_on_ambiguous: false,
            dedupe: false,
            auto_stage: false,

            vcs: crate::cli::VcsChoice::Auto,
            format: OutputFormat::Json,
        };
        let code = run(&args, &default_config()).unwrap();
        assert_eq!(code, exit_codes::SUCCESS);
    }

    #[test]
    fn multiple_files() {
        let mut conflict1 = NamedTempFile::new().unwrap();
        write!(conflict1, "{}", conflict_content()).unwrap();

        let mut conflict2 = NamedTempFile::new().unwrap();
        write!(conflict2, "{}", conflict_content()).unwrap();

        let resolutions = make_resolutions_json(r#"{"hunk_id": 0, "strategy": "left"}"#);

        let args = ResolveArgs {
            files: vec![
                conflict1.path().to_path_buf(),
                conflict2.path().to_path_buf(),
            ],
            resolutions: resolutions.path().to_path_buf(),
            dry_run: false,
            fail_on_ambiguous: false,
            dedupe: false,
            auto_stage: false,

            vcs: crate::cli::VcsChoice::Auto,
            format: OutputFormat::Text,
        };
        let code = run(&args, &default_config()).unwrap();
        assert_eq!(code, exit_codes::SUCCESS);

        for path in [conflict1.path(), conflict2.path()] {
            let result = std::fs::read_to_string(path).unwrap();
            assert!(result.contains("fn foo() { 1 }"));
            assert!(!result.contains("<<<<<<<"));
        }
    }
}
