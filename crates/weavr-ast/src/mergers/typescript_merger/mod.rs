//! TypeScript AST merger -- structural merge for TypeScript/TSX source code.
//!
//! Uses `tree-sitter` (TSX grammar) for parsing and source text slices for output.
//! Handles import dedup (named imports, type-only imports, namespace imports),
//! disjoint declaration additions, and graceful fallback to text merge.

mod format;
mod identity;
mod import_merge;
mod member_merge;
mod parse;

#[cfg(test)]
mod tests;

use std::path::Path;

use weavr_core::{ConflictHunk, Language};

use crate::error::AstError;
use crate::mergers::common::try_merge_ast;
use crate::{AstMergeResult, AstMerger};

use self::format::format_declarations;
use self::member_merge::{merge_three_way, merge_two_way};
use self::parse::{parse_fragment, ParsedFragment};

/// AST-based merger for TypeScript/TSX source code.
///
/// Parses conflict hunks with `tree-sitter` (TSX grammar, a strict superset of TS),
/// merges at the declaration level, and preserves original source text for
/// deterministic results.
#[derive(Debug, Default)]
pub struct TypeScriptMerger;

impl TypeScriptMerger {
    /// Creates a new `TypeScriptMerger`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl AstMerger for TypeScriptMerger {
    fn supported_languages(&self) -> &[Language] {
        &[Language::TypeScript]
    }

    fn supports(&self, _path: &Path, language: Language) -> bool {
        language == Language::TypeScript
    }

    fn try_merge(&self, hunk: &ConflictHunk) -> Result<Option<AstMergeResult>, AstError> {
        try_merge_ast(
            hunk,
            |text| match parse_fragment(text) {
                ParsedFragment::Declarations(decls) => Some(decls),
                ParsedFragment::Unparsable => None,
            },
            |left, right| merge_two_way(left, right),
            |base, left, right| merge_three_way(base, left, right),
            |decls| Ok(format_declarations(decls)),
        )
    }
}
