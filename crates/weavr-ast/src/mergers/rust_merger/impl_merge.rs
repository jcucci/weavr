//! Impl block merging — matches methods by identity and combines disjoint additions.

use std::collections::HashMap;

use syn::{ImplItem, Item};

use super::identity::ImplItemIdentity;
use super::tokens::{render_tokens, tokens_equal};

/// Attempts to merge two items that share the same identity but differ in content.
///
/// Currently only handles impl blocks. Returns `None` for other item types,
/// signaling that the conflict cannot be resolved at the AST level.
pub(super) fn try_merge_matching_items(left: &Item, right: &Item) -> Option<Item> {
    match (left, right) {
        (Item::Impl(left_impl), Item::Impl(right_impl)) => merge_impl_blocks(left_impl, right_impl),
        _ => None,
    }
}

/// Merges two impl blocks that target the same type (and trait).
///
/// Uses [`ImplItemIdentity`] to match methods:
/// - **Identical** — deduplicated (kept once).
/// - **Disjoint** — combined into the merged block.
/// - **Conflicting** (same name, different body) — returns `None`.
fn merge_impl_blocks(left: &syn::ItemImpl, right: &syn::ItemImpl) -> Option<Item> {
    let right_map = build_impl_identity_map(&right.items);

    let mut merged_items: Vec<ImplItem> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    // Process left items
    for item in &left.items {
        let id = ImplItemIdentity::from_impl_item(item);
        seen.insert(id.clone());

        if let Some(right_item) = right_map.get(&id) {
            if tokens_equal(item, *right_item) {
                // Identical — keep once
                merged_items.push(item.clone());
            } else {
                // Conflicting — cannot resolve
                return None;
            }
        } else {
            // Only on left side
            merged_items.push(item.clone());
        }
    }

    // Add right-only items
    for item in &right.items {
        let id = ImplItemIdentity::from_impl_item(item);
        if !seen.contains(&id) {
            merged_items.push(item.clone());
        }
    }

    // Build the merged impl block, preserving the left's structure
    let mut merged = left.clone();
    merged.items = merged_items;

    Some(Item::Impl(merged))
}

fn build_impl_identity_map(items: &[ImplItem]) -> HashMap<ImplItemIdentity, &ImplItem> {
    let mut map = HashMap::new();
    for item in items {
        let id = ImplItemIdentity::from_impl_item(item);
        map.insert(id, item);
    }
    map
}

/// Returns the rendered token string for an impl block, for description purposes.
pub(super) fn impl_description(item: &syn::ItemImpl) -> String {
    if let Some((_, ref path, _)) = item.trait_ {
        format!(
            "{} for {}",
            render_tokens(path),
            render_tokens(&item.self_ty)
        )
    } else {
        render_tokens(&item.self_ty)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_item(code: &str) -> Item {
        let file = syn::parse_file(code).unwrap();
        file.items.into_iter().next().unwrap()
    }

    #[test]
    fn merge_disjoint_methods() {
        let left = parse_item("impl Foo { fn alpha(&self) -> i32 { 1 } }");
        let right = parse_item("impl Foo { fn beta(&self) -> i32 { 2 } }");
        let merged =
            try_merge_matching_items(&left, &right).expect("disjoint methods should merge");
        let rendered = render_tokens(&merged);
        assert!(rendered.contains("alpha"));
        assert!(rendered.contains("beta"));
    }

    #[test]
    fn merge_identical_methods_dedup() {
        let left = parse_item("impl Foo { fn alpha(&self) -> i32 { 1 } }");
        let right = parse_item("impl Foo { fn alpha(&self) -> i32 { 1 } }");
        let merged =
            try_merge_matching_items(&left, &right).expect("identical methods should dedup");
        let rendered = render_tokens(&merged);
        // Should appear exactly once
        assert_eq!(rendered.matches("alpha").count(), 1);
    }

    #[test]
    fn conflicting_methods_returns_none() {
        let left = parse_item("impl Foo { fn alpha(&self) -> i32 { 1 } }");
        let right = parse_item("impl Foo { fn alpha(&self) -> i32 { 999 } }");
        let result = try_merge_matching_items(&left, &right);
        assert!(result.is_none(), "conflicting methods should return None");
    }

    #[test]
    fn non_impl_items_return_none() {
        let left = parse_item("fn foo() {}");
        let right = parse_item("fn foo() { bar(); }");
        let result = try_merge_matching_items(&left, &right);
        assert!(result.is_none());
    }

    enum ImplMergeKind {
        Identical,
        Disjoint,
    }

    fn classify_impl_merge(left: &syn::ItemImpl, right: &syn::ItemImpl) -> Option<ImplMergeKind> {
        let left_map = build_impl_identity_map(&left.items);
        let right_map = build_impl_identity_map(&right.items);

        let mut has_disjoint = false;
        let mut has_conflict = false;

        for item in &left.items {
            let id = ImplItemIdentity::from_impl_item(item);
            if let Some(right_item) = right_map.get(&id) {
                if !tokens_equal(item, *right_item) {
                    has_conflict = true;
                }
            } else {
                has_disjoint = true;
            }
        }

        for item in &right.items {
            let id = ImplItemIdentity::from_impl_item(item);
            if !left_map.contains_key(&id) {
                has_disjoint = true;
            }
        }

        if has_conflict {
            return None;
        }

        if has_disjoint {
            Some(ImplMergeKind::Disjoint)
        } else {
            Some(ImplMergeKind::Identical)
        }
    }

    #[test]
    fn classify_disjoint_impl() {
        let left: syn::ItemImpl = syn::parse_quote! { impl Foo { fn a(&self) {} } };
        let right: syn::ItemImpl = syn::parse_quote! { impl Foo { fn b(&self) {} } };
        let kind = classify_impl_merge(&left, &right);
        assert!(matches!(kind, Some(ImplMergeKind::Disjoint)));
    }

    #[test]
    fn classify_identical_impl() {
        let left: syn::ItemImpl = syn::parse_quote! { impl Foo { fn a(&self) {} } };
        let right: syn::ItemImpl = syn::parse_quote! { impl Foo { fn a(&self) {} } };
        let kind = classify_impl_merge(&left, &right);
        assert!(matches!(kind, Some(ImplMergeKind::Identical)));
    }

    #[test]
    fn classify_conflicting_impl() {
        let left: syn::ItemImpl = syn::parse_quote! { impl Foo { fn a(&self) -> i32 { 1 } } };
        let right: syn::ItemImpl = syn::parse_quote! { impl Foo { fn a(&self) -> i32 { 2 } } };
        let kind = classify_impl_merge(&left, &right);
        assert!(kind.is_none());
    }
}
