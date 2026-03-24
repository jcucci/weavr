//! C# AST merger -- structural merge for C# source code.
//!
//! Uses `tree-sitter` for parsing and source text slices for output.
//! Handles using-directive dedup, disjoint declaration additions, and
//! class member merging.

mod format;
mod identity;
mod member_merge;
mod parse;
mod using_merge;

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

/// AST-based merger for C# source code.
///
/// Parses conflict hunks with `tree-sitter`, merges at the declaration level,
/// and preserves original source text for deterministic results.
#[derive(Debug, Default)]
pub struct CSharpMerger;

impl CSharpMerger {
    /// Creates a new `CSharpMerger`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl AstMerger for CSharpMerger {
    fn supported_languages(&self) -> &[Language] {
        &[Language::CSharp]
    }

    fn supports(&self, _path: &Path, language: Language) -> bool {
        language == Language::CSharp
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
