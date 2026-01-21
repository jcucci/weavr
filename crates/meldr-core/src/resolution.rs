//! Resolution types for merge conflicts.

use serde::{Deserialize, Serialize};

/// Order for `AcceptBoth` strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BothOrder {
    /// Left content first, then right.
    #[default]
    LeftThenRight,
    /// Right content first, then left.
    RightThenLeft,
}

/// Options for the `AcceptBoth` strategy.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AcceptBothOptions {
    /// Order of content combination.
    pub order: BothOrder,
    /// Remove duplicate lines.
    pub deduplicate: bool,
    /// Normalize whitespace before comparison.
    pub trim_whitespace: bool,
}

/// Describes the source/method of a resolution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResolutionStrategyKind {
    /// Use left content verbatim.
    AcceptLeft,
    /// Use right content verbatim.
    AcceptRight,
    /// Combine left and right.
    AcceptBoth(AcceptBothOptions),
    /// User-provided content.
    Manual,
    /// Language-specific AST merge.
    AstMerged {
        /// The language used for AST merging.
        language: String,
    },
    /// AI-generated suggestion.
    AiSuggested {
        /// The AI provider name.
        provider: String,
    },
}

/// Source of a resolution.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ResolutionSource {
    /// Resolution made by user.
    #[default]
    User,
    /// Resolution suggested by AI.
    Ai,
    /// Resolution from AST analysis.
    Ast,
}

/// Metadata about a resolution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ResolutionMetadata {
    /// Source of the resolution.
    pub source: ResolutionSource,
    /// Optional notes.
    pub notes: Option<String>,
}

/// An explicit decision applied to a hunk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resolution {
    /// How the resolution was chosen.
    pub kind: ResolutionStrategyKind,
    /// The resolved content.
    pub content: String,
    /// Additional metadata.
    pub metadata: ResolutionMetadata,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_order_default() {
        assert_eq!(BothOrder::default(), BothOrder::LeftThenRight);
    }

    #[test]
    fn accept_both_options_default() {
        let opts = AcceptBothOptions::default();
        assert_eq!(opts.order, BothOrder::LeftThenRight);
        assert!(!opts.deduplicate);
        assert!(!opts.trim_whitespace);
    }

    #[test]
    fn resolution_source_default() {
        assert_eq!(ResolutionSource::default(), ResolutionSource::User);
    }

    #[test]
    fn resolution_metadata_default() {
        let meta = ResolutionMetadata::default();
        assert_eq!(meta.source, ResolutionSource::User);
        assert!(meta.notes.is_none());
    }

    #[test]
    fn resolution_creation() {
        let resolution = Resolution {
            kind: ResolutionStrategyKind::AcceptLeft,
            content: String::from("left content"),
            metadata: ResolutionMetadata::default(),
        };
        assert_eq!(resolution.kind, ResolutionStrategyKind::AcceptLeft);
    }
}
