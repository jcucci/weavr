//! Shared token utilities for comparison and identity mapping.

use std::collections::HashMap;

use quote::ToTokens;
use syn::Item;

use super::identity::ItemIdentity;

/// Renders a syntax node to its normalized token string representation.
pub(super) fn render_tokens<T: ToTokens>(node: &T) -> String {
    node.to_token_stream().to_string()
}

/// Returns whether two syntax nodes produce identical token streams.
pub(super) fn tokens_equal<T: ToTokens>(a: &T, b: &T) -> bool {
    render_tokens(a) == render_tokens(b)
}

/// Builds a map from [`ItemIdentity`] to the corresponding item reference.
///
/// If multiple items share the same identity, the last one wins.
pub(super) fn build_identity_map(items: &[Item]) -> HashMap<ItemIdentity, &Item> {
    let mut map = HashMap::new();
    for item in items {
        let id = ItemIdentity::from_item(item);
        map.insert(id, item);
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_tokens_produces_string() {
        let item: Item = syn::parse_quote! { fn hello() {} };
        let rendered = render_tokens(&item);
        assert!(rendered.contains("hello"));
    }

    #[test]
    fn tokens_equal_for_identical_items() {
        let a: Item = syn::parse_quote! { fn foo() -> i32 { 42 } };
        let b: Item = syn::parse_quote! { fn foo() -> i32 { 42 } };
        assert!(tokens_equal(&a, &b));
    }

    #[test]
    fn tokens_not_equal_for_different_items() {
        let a: Item = syn::parse_quote! { fn foo() -> i32 { 42 } };
        let b: Item = syn::parse_quote! { fn foo() -> i32 { 99 } };
        assert!(!tokens_equal(&a, &b));
    }

    #[test]
    fn build_identity_map_indexes_by_identity() {
        let items: Vec<Item> = vec![
            syn::parse_quote! { fn alpha() {} },
            syn::parse_quote! { fn beta() {} },
        ];
        let map = build_identity_map(&items);
        assert_eq!(map.len(), 2);
        assert!(map.contains_key(&ItemIdentity::Function("alpha".to_string())));
        assert!(map.contains_key(&ItemIdentity::Function("beta".to_string())));
    }
}
