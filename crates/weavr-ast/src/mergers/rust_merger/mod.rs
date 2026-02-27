//! Rust AST merger — structural merge for Rust source code.
//!
//! Uses `syn` for parsing and `prettyplease` for deterministic output formatting.
//! Handles use-statement dedup, disjoint function additions, and impl block
//! method merging.

mod format;
mod identity;
mod impl_merge;
mod item_merge;
mod parse;
mod tokens;
mod use_merge;

#[cfg(test)]
mod tests;

use std::path::Path;

use weavr_core::{ConflictHunk, Language};

use crate::error::AstError;
use crate::{AstMergeResult, AstMerger};

use self::format::format_items;
use self::item_merge::{merge_three_way, merge_two_way};
use self::parse::{parse_fragment, ParsedFragment};

/// AST-based merger for Rust source code.
///
/// Parses conflict hunks with `syn`, merges at the item level, and
/// formats output with `prettyplease` for deterministic results.
#[derive(Debug, Default)]
pub struct RustMerger;

impl RustMerger {
    /// Creates a new `RustMerger`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl AstMerger for RustMerger {
    fn supported_languages(&self) -> &[Language] {
        &[Language::Rust]
    }

    fn supports(&self, _path: &Path, language: Language) -> bool {
        language == Language::Rust
    }

    fn try_merge(&self, hunk: &ConflictHunk) -> Result<Option<AstMergeResult>, AstError> {
        let left = match parse_fragment(&hunk.left.text) {
            ParsedFragment::Items(items) => items,
            ParsedFragment::Unparsable => return Ok(None),
        };

        let right = match parse_fragment(&hunk.right.text) {
            ParsedFragment::Items(items) => items,
            ParsedFragment::Unparsable => return Ok(None),
        };

        let base = if let Some(ref base_content) = hunk.base {
            match parse_fragment(&base_content.text) {
                ParsedFragment::Items(items) => Some(items),
                ParsedFragment::Unparsable => None,
            }
        } else {
            None
        };

        let merged = if let Some(base_items) = base {
            merge_three_way(&base_items, &left, &right)?
        } else {
            merge_two_way(&left, &right)?
        };

        let Some(result) = merged else {
            return Ok(None);
        };

        let content = format_items(&result.items)?;

        Ok(Some(AstMergeResult {
            content,
            confidence: result.confidence,
            description: result.description,
        }))
    }
}
