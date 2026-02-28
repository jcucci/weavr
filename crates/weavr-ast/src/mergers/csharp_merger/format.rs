//! Output formatting via source text preservation.
//!
//! Unlike the Rust merger which uses `prettyplease`, the C# merger preserves
//! original source text from tree-sitter. Reconstructed declarations (e.g.,
//! merged usings) use simple string formatting.

use super::parse::CSharpDeclaration;

/// Formats a list of declarations into the final merged output string.
///
/// Preserves original source text for unchanged declarations and uses
/// reconstructed text for merged declarations.
pub(super) fn format_declarations(decls: &[CSharpDeclaration]) -> String {
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
    use crate::mergers::csharp_merger::identity::CSharpIdentity;
    use crate::mergers::csharp_merger::parse::DeclarationKind;

    fn make_using(path: &str) -> CSharpDeclaration {
        CSharpDeclaration {
            kind: DeclarationKind::UsingDirective,
            identity: CSharpIdentity::Using(path.to_string()),
            source_text: format!("using {path};"),
            children: Vec::new(),
        }
    }

    #[test]
    fn format_single_using() {
        let decls = vec![make_using("System")];
        let result = format_declarations(&decls);
        assert_eq!(result, "using System;");
    }

    #[test]
    fn format_multiple_usings() {
        let decls = vec![make_using("System"), make_using("System.IO")];
        let result = format_declarations(&decls);
        assert_eq!(result, "using System;\nusing System.IO;");
    }

    #[test]
    fn format_empty() {
        let decls: Vec<CSharpDeclaration> = vec![];
        let result = format_declarations(&decls);
        assert!(result.is_empty());
    }

    #[test]
    fn format_preserves_source_text() {
        let decl = CSharpDeclaration {
            kind: DeclarationKind::Class,
            identity: CSharpIdentity::Class("Foo".to_string()),
            source_text: "public class Foo\n{\n    public void Bar() { }\n}".to_string(),
            children: Vec::new(),
        };
        let result = format_declarations(&[decl]);
        assert!(result.contains("public class Foo"));
        assert!(result.contains("public void Bar()"));
    }

    #[test]
    fn format_deterministic() {
        let decls = vec![make_using("System"), make_using("System.IO")];
        let first = format_declarations(&decls);
        let second = format_declarations(&decls);
        assert_eq!(first, second);
    }
}
