//! Deterministic output formatting via `prettyplease`.

use syn::Item;

use crate::AstError;

/// Formats a list of items into a deterministic, idiomatic Rust string.
///
/// Builds a synthetic `syn::File`, renders it to tokens via `quote`,
/// re-parses, and formats with `prettyplease::unparse()`.
pub(super) fn format_items(items: &[Item]) -> Result<String, AstError> {
    let file = syn::File {
        shebang: None,
        attrs: Vec::new(),
        items: items.to_vec(),
    };

    let token_stream = quote::quote!(#file);
    let reparsed = syn::parse_file(&token_stream.to_string())
        .map_err(|e| AstError::Internal(format!("failed to re-parse formatted output: {e}")))?;

    let formatted = prettyplease::unparse(&reparsed);

    // Trim trailing whitespace from each line for clean output
    let cleaned: String = formatted
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n");

    Ok(cleaned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_single_function() {
        let items: Vec<Item> = vec![syn::parse_quote! {
            fn hello() -> i32 {
                42
            }
        }];
        let result = format_items(&items).unwrap();
        assert!(result.contains("fn hello"));
        assert!(result.contains("42"));
    }

    #[test]
    fn format_use_statements() {
        let items: Vec<Item> = vec![
            syn::parse_quote! { use std::fs; },
            syn::parse_quote! { use std::io; },
        ];
        let result = format_items(&items).unwrap();
        assert!(result.contains("use std::fs;"));
        assert!(result.contains("use std::io;"));
    }

    #[test]
    fn format_empty_items() {
        let items: Vec<Item> = vec![];
        let result = format_items(&items).unwrap();
        assert!(result.trim().is_empty());
    }

    #[test]
    fn format_produces_deterministic_output() {
        let items: Vec<Item> = vec![
            syn::parse_quote! { use std::io; },
            syn::parse_quote! { fn foo() {} },
        ];
        let first = format_items(&items).unwrap();
        let second = format_items(&items).unwrap();
        assert_eq!(first, second);
    }
}
