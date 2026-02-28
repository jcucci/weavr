//! Output formatting via source text preservation.
//!
//! The TypeScript merger preserves original source text from tree-sitter.
//! Reconstructed declarations (e.g., merged imports) use simple string formatting.

use super::parse::TsDeclaration;

/// Formats a list of declarations into the final merged output string.
///
/// Preserves original source text for unchanged declarations and uses
/// reconstructed text for merged declarations.
pub(super) fn format_declarations(decls: &[TsDeclaration]) -> String {
    if decls.is_empty() {
        return String::new();
    }

    let mut output = String::new();

    for (i, decl) in decls.iter().enumerate() {
        if i > 0 {
            output.push('\n');
        }
        output.push_str(&decl.source_text);
    }

    // Trim trailing whitespace from each line for clean output
    let cleaned: String = output
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n");

    cleaned
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::mergers::typescript_merger::identity::{ImportKey, ImportKind, TsIdentity};
    use crate::mergers::typescript_merger::parse::DeclarationKind;

    fn make_import(module: &str) -> TsDeclaration {
        TsDeclaration {
            kind: DeclarationKind::ImportStatement,
            identity: TsIdentity::Import(ImportKey {
                module: module.to_string(),
                kind: ImportKind::Value,
            }),
            source_text: format!("import {{ }} from '{module}';"),
            specifiers: BTreeSet::new(),
        }
    }

    #[test]
    fn format_single_import() {
        let decls = vec![make_import("react")];
        let result = format_declarations(&decls);
        assert_eq!(result, "import { } from 'react';");
    }

    #[test]
    fn format_multiple_imports() {
        let decls = vec![make_import("react"), make_import("react-dom")];
        let result = format_declarations(&decls);
        assert!(result.contains("react"));
        assert!(result.contains("react-dom"));
    }

    #[test]
    fn format_empty() {
        let decls: Vec<TsDeclaration> = vec![];
        let result = format_declarations(&decls);
        assert!(result.is_empty());
    }

    #[test]
    fn format_preserves_source_text() {
        let decl = TsDeclaration {
            kind: DeclarationKind::Function,
            identity: TsIdentity::Function("hello".to_string()),
            source_text: "function hello() {\n    return 'world';\n}".to_string(),
            specifiers: BTreeSet::new(),
        };
        let result = format_declarations(&[decl]);
        assert!(result.contains("function hello()"));
        assert!(result.contains("return 'world'"));
    }

    #[test]
    fn format_deterministic() {
        let decls = vec![make_import("react"), make_import("react-dom")];
        let first = format_declarations(&decls);
        let second = format_declarations(&decls);
        assert_eq!(first, second);
    }
}
