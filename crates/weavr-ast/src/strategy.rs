//! AST strategy wrapper that coordinates mergers and applies configuration.

use std::path::Path;

use weavr_core::{Language, Resolution};

use crate::config::AstConfig;
use crate::error::AstError;
use crate::AstMerger;

/// Coordinates registered [`AstMerger`] implementations with configuration-based
/// filtering and confidence thresholds.
///
/// `AstStrategy` returns `Option<Resolution>` — when it returns `None`, the caller
/// is expected to fall back to text-based strategies.
pub struct AstStrategy {
    mergers: Vec<Box<dyn AstMerger>>,
    config: AstConfig,
}

impl AstStrategy {
    /// Creates a new `AstStrategy` with the given mergers and configuration.
    #[must_use]
    pub fn new(mergers: Vec<Box<dyn AstMerger>>, config: AstConfig) -> Self {
        Self { mergers, config }
    }

    /// Attempts to resolve a conflict hunk using AST-based merging.
    ///
    /// Returns:
    /// - `Ok(Some(resolution))` if a merger produced a result above the confidence threshold
    /// - `Ok(None)` if AST merging is disabled, no merger matches, the merger declines,
    ///   or the confidence is below the threshold
    /// - `Err(AstError)` on parse or internal errors
    ///
    /// # Errors
    ///
    /// Returns [`AstError::ParseError`] or [`AstError::Internal`] if a merger
    /// encounters an error during merging.
    pub fn try_resolve(
        &self,
        hunk: &weavr_core::ConflictHunk,
        file_path: &Path,
        language: Language,
    ) -> Result<Option<Resolution>, AstError> {
        if !self.config.enabled {
            return Ok(None);
        }

        if self.config.excluded_languages.contains(&language) {
            return Ok(None);
        }

        for merger in &self.mergers {
            if !merger.supports(file_path, language) {
                continue;
            }

            if let Some(result) = merger.try_merge(hunk) {
                if result.confidence < self.config.min_confidence {
                    return Ok(None);
                }

                return Ok(Some(Resolution::ast_merged(
                    result.content,
                    language,
                    result.description,
                    result.confidence,
                )));
            }
        }

        Ok(None)
    }

    /// Returns all languages supported across all registered mergers.
    #[must_use]
    pub fn supported_languages(&self) -> Vec<Language> {
        let mut languages = Vec::new();
        for merger in &self.mergers {
            for &lang in merger.supported_languages() {
                if !languages.contains(&lang) {
                    languages.push(lang);
                }
            }
        }
        languages
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use weavr_core::{
        ConflictHunk, HunkContent, HunkContext, HunkId, HunkState, Language, ResolutionSource,
        ResolutionStrategyKind,
    };

    use crate::AstMergeResult;

    fn test_hunk() -> ConflictHunk {
        ConflictHunk {
            id: HunkId(1),
            left: HunkContent {
                text: "left".to_string(),
            },
            right: HunkContent {
                text: "right".to_string(),
            },
            base: None,
            context: HunkContext::default(),
            state: HunkState::default(),
        }
    }

    struct FakeMerger {
        languages: Vec<Language>,
        result: Option<AstMergeResult>,
    }

    impl AstMerger for FakeMerger {
        fn supported_languages(&self) -> &[Language] {
            &self.languages
        }

        fn supports(&self, _path: &Path, language: Language) -> bool {
            self.languages.contains(&language)
        }

        fn try_merge(&self, _hunk: &ConflictHunk) -> Option<AstMergeResult> {
            self.result.clone()
        }
    }

    #[test]
    fn returns_none_when_disabled() {
        let config = AstConfig {
            enabled: false,
            ..Default::default()
        };
        let strategy = AstStrategy::new(vec![], config);
        let result = strategy
            .try_resolve(&test_hunk(), Path::new("main.rs"), Language::Rust)
            .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn returns_none_when_language_excluded() {
        let config = AstConfig {
            excluded_languages: vec![Language::Rust],
            ..Default::default()
        };
        let merger = FakeMerger {
            languages: vec![Language::Rust],
            result: Some(AstMergeResult {
                content: "merged".to_string(),
                confidence: 0.9,
                description: "test".to_string(),
            }),
        };
        let strategy = AstStrategy::new(vec![Box::new(merger)], config);
        let result = strategy
            .try_resolve(&test_hunk(), Path::new("main.rs"), Language::Rust)
            .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn returns_none_when_no_merger_matches() {
        let config = AstConfig::default();
        let merger = FakeMerger {
            languages: vec![Language::Go],
            result: Some(AstMergeResult {
                content: "merged".to_string(),
                confidence: 0.9,
                description: "test".to_string(),
            }),
        };
        let strategy = AstStrategy::new(vec![Box::new(merger)], config);
        let result = strategy
            .try_resolve(&test_hunk(), Path::new("main.rs"), Language::Rust)
            .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn returns_none_when_merger_declines() {
        let config = AstConfig::default();
        let merger = FakeMerger {
            languages: vec![Language::Rust],
            result: None,
        };
        let strategy = AstStrategy::new(vec![Box::new(merger)], config);
        let result = strategy
            .try_resolve(&test_hunk(), Path::new("main.rs"), Language::Rust)
            .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn returns_none_when_confidence_below_threshold() {
        let config = AstConfig {
            min_confidence: 0.8,
            ..Default::default()
        };
        let merger = FakeMerger {
            languages: vec![Language::Rust],
            result: Some(AstMergeResult {
                content: "merged".to_string(),
                confidence: 0.5,
                description: "low confidence".to_string(),
            }),
        };
        let strategy = AstStrategy::new(vec![Box::new(merger)], config);
        let result = strategy
            .try_resolve(&test_hunk(), Path::new("main.rs"), Language::Rust)
            .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn returns_resolution_on_successful_merge() {
        let config = AstConfig::default();
        let merger = FakeMerger {
            languages: vec![Language::Rust],
            result: Some(AstMergeResult {
                content: "merged imports".to_string(),
                confidence: 0.9,
                description: "Merged 3 imports".to_string(),
            }),
        };
        let strategy = AstStrategy::new(vec![Box::new(merger)], config);
        let result = strategy
            .try_resolve(&test_hunk(), Path::new("main.rs"), Language::Rust)
            .unwrap();

        let resolution = result.expect("should return a resolution");
        assert_eq!(resolution.content, "merged imports");
        assert_eq!(
            resolution.kind,
            ResolutionStrategyKind::AstMerged {
                language: Language::Rust
            }
        );
        assert_eq!(resolution.metadata.source, ResolutionSource::Ast);
        assert_eq!(
            resolution.metadata.notes.as_deref(),
            Some("Merged 3 imports")
        );
        assert_eq!(resolution.metadata.confidence, Some(90));
    }

    #[test]
    fn supported_languages_aggregates_mergers() {
        let merger1 = FakeMerger {
            languages: vec![Language::Rust, Language::Go],
            result: None,
        };
        let merger2 = FakeMerger {
            languages: vec![Language::Go, Language::TypeScript],
            result: None,
        };
        let strategy = AstStrategy::new(
            vec![Box::new(merger1), Box::new(merger2)],
            AstConfig::default(),
        );
        let languages = strategy.supported_languages();
        assert_eq!(languages.len(), 3);
        assert!(languages.contains(&Language::Rust));
        assert!(languages.contains(&Language::Go));
        assert!(languages.contains(&Language::TypeScript));
    }
}
