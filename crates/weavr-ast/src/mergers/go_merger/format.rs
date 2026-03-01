//! Output formatting via source text preservation.
//!
//! Preserves original source text from tree-sitter for unchanged declarations.
//! Reconstructed declarations (e.g., merged imports) use simple string formatting
//! with Go-conventional grouping.

use super::parse::GoDeclaration;

/// Formats a list of declarations into the final merged output string.
///
/// Preserves original source text for unchanged declarations and uses
/// reconstructed text for merged declarations.
pub(super) fn format_declarations(decls: &[GoDeclaration]) -> String {
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
    use super::*;
    use crate::mergers::go_merger::identity::{GoIdentity, ImportKey, ImportKind};
    use crate::mergers::go_merger::parse::DeclarationKind;

    fn make_import(path: &str) -> GoDeclaration {
        GoDeclaration {
            kind: DeclarationKind::ImportDeclaration,
            identity: GoIdentity::Import(ImportKey {
                path: path.to_string(),
                kind: ImportKind::Normal,
            }),
            source_text: format!("import \"{path}\""),
            children: Vec::new(),
        }
    }

    #[test]
    fn format_single_import() {
        let decls = vec![make_import("fmt")];
        let result = format_declarations(&decls);
        assert_eq!(result, "import \"fmt\"");
    }

    #[test]
    fn format_multiple_declarations() {
        let decls = vec![
            make_import("fmt"),
            GoDeclaration {
                kind: DeclarationKind::Function,
                identity: GoIdentity::Function("main".to_string()),
                source_text: "func main() {\n\tfmt.Println(\"hello\")\n}".to_string(),
                children: Vec::new(),
            },
        ];
        let result = format_declarations(&decls);
        assert!(result.contains("import \"fmt\""));
        assert!(result.contains("func main()"));
    }

    #[test]
    fn format_empty() {
        let decls: Vec<GoDeclaration> = vec![];
        let result = format_declarations(&decls);
        assert!(result.is_empty());
    }

    #[test]
    fn format_preserves_source_text() {
        let decl = GoDeclaration {
            kind: DeclarationKind::Function,
            identity: GoIdentity::Function("Foo".to_string()),
            source_text: "func Foo() {\n\treturn 42\n}".to_string(),
            children: Vec::new(),
        };
        let result = format_declarations(&[decl]);
        assert!(result.contains("func Foo()"));
        assert!(result.contains("return 42"));
    }

    #[test]
    fn format_deterministic() {
        let decls = vec![make_import("fmt"), make_import("os")];
        let first = format_declarations(&decls);
        let second = format_declarations(&decls);
        assert_eq!(first, second);
    }
}
