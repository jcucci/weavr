//! Shared logic used by all language-specific mergers.

use std::collections::BTreeSet;

use weavr_core::ConflictHunk;

use crate::error::AstError;
use crate::AstMergeResult;

/// The result of a language-specific merge, generic over the parsed item type.
pub(crate) struct RawMergeOutput<D> {
    pub items: Vec<D>,
    pub confidence: f32,
    pub description: String,
}

/// Shared try_merge scaffold for all AST mergers.
///
/// Handles the common parse-dispatch-format control flow:
/// 1. Parse left and right fragments (bail if unparsable).
/// 2. Parse the optional base fragment.
/// 3. Dispatch to two-way or three-way merge.
/// 4. Format the merged items into a string.
/// 5. Return an [`AstMergeResult`].
pub(crate) fn try_merge_ast<D>(
    hunk: &ConflictHunk,
    parse: impl Fn(&str) -> Option<Vec<D>>,
    merge_two_way: impl FnOnce(&[D], &[D]) -> Result<Option<RawMergeOutput<D>>, AstError>,
    merge_three_way: impl FnOnce(&[D], &[D], &[D]) -> Result<Option<RawMergeOutput<D>>, AstError>,
    format: impl FnOnce(&[D]) -> Result<String, AstError>,
) -> Result<Option<AstMergeResult>, AstError> {
    let Some(left) = parse(&hunk.left.text) else {
        return Ok(None);
    };
    let Some(right) = parse(&hunk.right.text) else {
        return Ok(None);
    };
    let base = hunk.base.as_ref().and_then(|b| parse(&b.text));

    let merged = if let Some(ref base_items) = base {
        merge_three_way(base_items, &left, &right)?
    } else {
        merge_two_way(&left, &right)?
    };

    let Some(result) = merged else {
        return Ok(None);
    };

    let content = format(&result.items)?;

    Ok(Some(AstMergeResult {
        content,
        confidence: result.confidence,
        description: result.description,
    }))
}

/// Three-way merge for ordered sets: union of both sides' additions,
/// minus elements deleted by either side (that weren't re-added by the other).
pub(crate) fn three_way_merge_sets<T: Ord + Clone>(
    base: &BTreeSet<T>,
    left: &BTreeSet<T>,
    right: &BTreeSet<T>,
) -> BTreeSet<T> {
    let mut result = base.clone();

    // Add new elements from left (not in base)
    for item in left.difference(base) {
        result.insert(item.clone());
    }

    // Add new elements from right (not in base)
    for item in right.difference(base) {
        result.insert(item.clone());
    }

    // Remove elements deleted by left (in base but not in left)
    for item in base.difference(left) {
        result.remove(item);
    }

    // Remove elements deleted by right (in base but not in right)
    for item in base.difference(right) {
        result.remove(item);
    }

    result
}
