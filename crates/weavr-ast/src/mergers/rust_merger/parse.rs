//! Tiered fragment parsing for Rust source code.
//!
//! Conflict hunks often contain partial Rust code (e.g., a few `use` statements
//! or method bodies without the surrounding module/impl block). This module
//! tries multiple parsing strategies to extract valid `syn::Item`s.

use syn::Item;

/// The result of attempting to parse a code fragment.
pub(super) enum ParsedFragment {
    /// Successfully parsed into a list of items.
    Items(Vec<Item>),
    /// Could not be parsed by any strategy.
    Unparsable,
}

/// Attempts to parse a text fragment into Rust items using a tiered strategy:
///
/// 1. Parse as a complete file — handles top-level items, use blocks, etc.
/// 2. Wrap in a dummy module and re-parse — handles items needing outer context.
/// 3. If all fail, return [`ParsedFragment::Unparsable`].
pub(super) fn parse_fragment(text: &str) -> ParsedFragment {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return ParsedFragment::Items(Vec::new());
    }

    // Strategy 1: parse as a complete file
    if let Ok(file) = syn::parse_file(trimmed) {
        return ParsedFragment::Items(file.items);
    }

    // Strategy 2: wrap in a dummy module
    let wrapped = format!("mod __weavr_dummy__ {{ {trimmed} }}");
    if let Ok(file) = syn::parse_file(&wrapped) {
        // Extract items from inside the dummy module
        for item in file.items {
            if let Item::Mod(m) = item {
                if let Some((_, items)) = m.content {
                    return ParsedFragment::Items(items);
                }
            }
        }
    }

    ParsedFragment::Unparsable
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_use_statements() {
        let text = "use std::io;\nuse std::fs;";
        let result = parse_fragment(text);
        match result {
            ParsedFragment::Items(items) => assert_eq!(items.len(), 2),
            ParsedFragment::Unparsable => panic!("should parse use statements"),
        }
    }

    #[test]
    fn parse_function() {
        let text = "fn hello() { println!(\"hello\"); }";
        let result = parse_fragment(text);
        match result {
            ParsedFragment::Items(items) => {
                assert_eq!(items.len(), 1);
                assert!(matches!(items[0], Item::Fn(_)));
            }
            ParsedFragment::Unparsable => panic!("should parse function"),
        }
    }

    #[test]
    fn parse_empty_input() {
        let result = parse_fragment("");
        match result {
            ParsedFragment::Items(items) => assert!(items.is_empty()),
            ParsedFragment::Unparsable => panic!("empty input should produce empty items"),
        }
    }

    #[test]
    fn parse_whitespace_only() {
        let result = parse_fragment("   \n  \t  ");
        match result {
            ParsedFragment::Items(items) => assert!(items.is_empty()),
            ParsedFragment::Unparsable => panic!("whitespace should produce empty items"),
        }
    }

    #[test]
    fn parse_unparsable_fragment() {
        let result = parse_fragment("this is not valid rust code @#$%");
        assert!(matches!(result, ParsedFragment::Unparsable));
    }

    #[test]
    fn parse_impl_item_via_wrapping() {
        // An impl method alone isn't a valid top-level item, but wrapping should help
        // if it's a valid item in module context
        let text = "impl Foo { fn bar(&self) {} }";
        let result = parse_fragment(text);
        match result {
            ParsedFragment::Items(items) => {
                assert_eq!(items.len(), 1);
                assert!(matches!(items[0], Item::Impl(_)));
            }
            ParsedFragment::Unparsable => panic!("should parse impl block"),
        }
    }
}
