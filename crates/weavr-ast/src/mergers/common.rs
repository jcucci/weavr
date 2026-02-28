//! Shared set-merging logic used by all language-specific mergers.

use std::collections::BTreeSet;

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
