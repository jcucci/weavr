//! General item-level merge algorithms for 2-way and 3-way merges.

use std::collections::BTreeSet;

use syn::Item;

use super::identity::ItemIdentity;
use super::impl_merge;
use super::tokens::{build_identity_map, tokens_equal};
use super::use_merge;
use crate::mergers::common::RawMergeOutput;
use crate::mergers::confidence::{compute_import_confidence, compute_mixed_confidence};
use crate::AstError;

/// Returns whether all items in the slice are `use` statements.
fn all_uses(items: &[Item]) -> bool {
    !items.is_empty() && items.iter().all(|i| matches!(i, Item::Use(_)))
}

/// Two-way merge: identity-based merge of left and right items.
///
/// - If all items are use statements, delegates to `use_merge`.
/// - For items with the same identity but different content, tries `impl_merge`.
/// - Disjoint items are combined.
/// - Returns `None` if a conflict is detected that cannot be resolved.
pub(super) fn merge_two_way(
    left: &[Item],
    right: &[Item],
) -> Result<Option<RawMergeOutput<Item>>, AstError> {
    // Special-case: pure use-statement merge
    if all_uses(left) && all_uses(right) {
        return match use_merge::merge_use_items(left, right, None)? {
            Some((items, desc)) => Ok(Some(RawMergeOutput {
                confidence: compute_import_confidence(all_uses(&items), false),
                items,
                description: desc,
            })),
            None => Ok(None),
        };
    }

    let right_map = build_identity_map(right);

    let mut merged: Vec<Item> = Vec::new();
    let mut seen = BTreeSet::new();
    let mut descriptions = Vec::new();
    let mut has_impl_disjoint = false;
    let mut has_use_merge = false;

    // Collect use items for separate merging
    let left_uses: Vec<Item> = left
        .iter()
        .filter(|i| matches!(i, Item::Use(_)))
        .cloned()
        .collect();
    let right_uses: Vec<Item> = right
        .iter()
        .filter(|i| matches!(i, Item::Use(_)))
        .cloned()
        .collect();

    if !left_uses.is_empty() || !right_uses.is_empty() {
        if let Some((use_items, desc)) = use_merge::merge_use_items(&left_uses, &right_uses, None)?
        {
            merged.extend(use_items);
            descriptions.push(desc);
            has_use_merge = true;
        } else {
            // Identical uses — just keep left's
            merged.extend(left_uses);
        }
        // Mark all use identities as seen
        for item in left.iter().chain(right.iter()) {
            if matches!(item, Item::Use(_)) {
                seen.insert(ItemIdentity::from_item(item));
            }
        }
    }

    // Process non-use left items
    for item in left {
        if matches!(item, Item::Use(_)) {
            continue;
        }
        let id = ItemIdentity::from_item(item);
        seen.insert(id.clone());

        if let Some(right_item) = right_map.get(&id) {
            if tokens_equal(item, *right_item) {
                // Identical — keep once
                merged.push(item.clone());
            } else {
                // Try impl merge
                match impl_merge::try_merge_matching_items(item, right_item) {
                    Some(merged_item) => {
                        if let Item::Impl(ref li) = item {
                            descriptions
                                .push(format!("Merged impl {}", impl_merge::impl_description(li)));
                        }
                        has_impl_disjoint = true;
                        merged.push(merged_item);
                    }
                    None => return Ok(None), // Unresolvable conflict
                }
            }
        } else {
            // Only on left side
            merged.push(item.clone());
        }
    }

    // Add right-only non-use items
    for item in right {
        if matches!(item, Item::Use(_)) {
            continue;
        }
        let id = ItemIdentity::from_item(item);
        if !seen.contains(&id) {
            merged.push(item.clone());
        }
    }

    let description = if descriptions.is_empty() {
        "Merged items".to_string()
    } else {
        descriptions.join("; ")
    };

    let confidence = compute_mixed_confidence(has_use_merge, has_impl_disjoint, false);
    Ok(Some(RawMergeOutput {
        items: merged,
        confidence,
        description,
    }))
}

/// Three-way merge: classifies each identity as unchanged/added/modified/deleted per side.
#[allow(clippy::too_many_lines)]
pub(super) fn merge_three_way(
    base: &[Item],
    left: &[Item],
    right: &[Item],
) -> Result<Option<RawMergeOutput<Item>>, AstError> {
    // Special-case: pure use-statement merge
    if all_uses(left) && all_uses(right) && all_uses(base) {
        return match use_merge::merge_use_items(left, right, Some(base))? {
            Some((items, desc)) => Ok(Some(RawMergeOutput {
                confidence: compute_import_confidence(all_uses(&items), true),
                items,
                description: desc,
            })),
            None => Ok(None),
        };
    }

    let base_map = build_identity_map(base);
    let left_map = build_identity_map(left);
    let right_map = build_identity_map(right);

    let mut merged: Vec<Item> = Vec::new();
    let mut seen = BTreeSet::new();
    let mut descriptions = Vec::new();
    let mut has_impl_disjoint = false;
    let mut has_use_merge = false;

    // Handle use items via 3-way use merge
    let base_uses: Vec<Item> = base
        .iter()
        .filter(|i| matches!(i, Item::Use(_)))
        .cloned()
        .collect();
    let left_uses: Vec<Item> = left
        .iter()
        .filter(|i| matches!(i, Item::Use(_)))
        .cloned()
        .collect();
    let right_uses: Vec<Item> = right
        .iter()
        .filter(|i| matches!(i, Item::Use(_)))
        .cloned()
        .collect();

    if !base_uses.is_empty() || !left_uses.is_empty() || !right_uses.is_empty() {
        if let Some((use_items, desc)) =
            use_merge::merge_use_items(&left_uses, &right_uses, Some(&base_uses))?
        {
            merged.extend(use_items);
            descriptions.push(desc);
            has_use_merge = true;
        } else {
            merged.extend(left_uses);
        }
        for item in base.iter().chain(left.iter()).chain(right.iter()) {
            if matches!(item, Item::Use(_)) {
                seen.insert(ItemIdentity::from_item(item));
            }
        }
    }

    // Collect all non-use identities across all three sides
    let mut all_ids = BTreeSet::new();
    for item in base.iter().chain(left.iter()).chain(right.iter()) {
        if !matches!(item, Item::Use(_)) {
            all_ids.insert(ItemIdentity::from_item(item));
        }
    }

    for id in &all_ids {
        if seen.contains(id) {
            continue;
        }
        seen.insert(id.clone());

        let in_base = base_map.get(id);
        let in_left = left_map.get(id);
        let in_right = right_map.get(id);

        match (in_base, in_left, in_right) {
            // In all three — check for modifications
            (Some(b), Some(l), Some(r)) => {
                if !merge_three_present(b, l, r, &mut merged, &mut has_impl_disjoint) {
                    return Ok(None);
                }
            }
            // Deleted by one side — respect deletion (unless modified by the other)
            (Some(b), Some(l), None) => {
                if !tokens_equal(*b, *l) {
                    // Left modified, right deleted — conflict
                    return Ok(None);
                }
                // Right deleted it, left didn't change — respect deletion
            }
            (Some(b), None, Some(r)) => {
                if !tokens_equal(*b, *r) {
                    // Right modified, left deleted — conflict
                    return Ok(None);
                }
                // Left deleted it, right didn't change — respect deletion
            }
            // Both sides deleted, or identity not actually present
            (Some(_) | None, None, None) => {}
            // Added by both sides
            (None, Some(l), Some(r)) => {
                if tokens_equal(*l, *r) {
                    merged.push((*l).clone());
                } else {
                    match impl_merge::try_merge_matching_items(l, r) {
                        Some(merged_item) => {
                            has_impl_disjoint = true;
                            merged.push(merged_item);
                        }
                        None => return Ok(None),
                    }
                }
            }
            (None, Some(l), None) => {
                merged.push((*l).clone());
            }
            (None, None, Some(r)) => {
                merged.push((*r).clone());
            }
        }
    }

    let description = if descriptions.is_empty() {
        "Three-way merged items".to_string()
    } else {
        descriptions.join("; ")
    };

    let confidence = compute_mixed_confidence(has_use_merge, has_impl_disjoint, true);
    Ok(Some(RawMergeOutput {
        items: merged,
        confidence,
        description,
    }))
}

/// Handles the case where an item is present in all three sides (base, left, right).
///
/// Returns `false` if the items conflict and cannot be merged.
fn merge_three_present(
    b: &Item,
    l: &Item,
    r: &Item,
    merged: &mut Vec<Item>,
    has_impl_disjoint: &mut bool,
) -> bool {
    let left_changed = !tokens_equal(b, l);
    let right_changed = !tokens_equal(b, r);

    match (left_changed, right_changed) {
        (false, false) => {
            // Unchanged — keep base version
            merged.push(b.clone());
        }
        (true, false) => {
            // Only left modified — take left
            merged.push(l.clone());
        }
        (false, true) => {
            // Only right modified — take right
            merged.push(r.clone());
        }
        (true, true) => {
            // Both modified — try impl merge
            if tokens_equal(l, r) {
                merged.push(l.clone());
            } else if let Some(merged_item) = impl_merge::try_merge_matching_items(l, r) {
                *has_impl_disjoint = true;
                merged.push(merged_item);
            } else {
                return false;
            }
        }
    }

    true
}
