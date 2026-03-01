//! Tiered fragment parsing for Go source code.
//!
//! Conflict hunks often contain partial Go code (e.g., a few `import` specs
//! or function bodies without the surrounding package clause). This module tries
//! multiple parsing strategies to extract valid declarations using tree-sitter.

use tree_sitter::{Node, Parser};

use super::identity::{GoIdentity, ImportKey, ImportKind};

/// The result of attempting to parse a code fragment.
pub(super) enum ParsedFragment {
    /// Successfully parsed into a list of declarations.
    Declarations(Vec<GoDeclaration>),
    /// Could not be parsed by any strategy.
    Unparsable,
}

/// A single Go declaration extracted from the parse tree.
#[derive(Debug, Clone)]
pub(super) struct GoDeclaration {
    /// The kind of declaration.
    pub kind: DeclarationKind,
    /// Identity for matching across conflict sides.
    pub identity: GoIdentity,
    /// The original source text of this declaration.
    pub source_text: String,
    /// Nested members (for struct fields, interface methods, etc.).
    pub children: Vec<GoDeclaration>,
}

/// The kind of Go declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DeclarationKind {
    /// `import "fmt"` or `import (...)`
    ImportDeclaration,
    /// `func Foo() { ... }`
    Function,
    /// `func (r *Receiver) Foo() { ... }`
    Method,
    /// `type Foo struct { ... }` / `type Foo interface { ... }` / `type Foo = Bar`
    TypeDeclaration,
    /// `const Foo = 1` or `const (...)`
    Const,
    /// `var Foo int` or `var (...)`
    Var,
}

fn create_parser() -> Parser {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_go::LANGUAGE.into())
        .expect("failed to set Go language for tree-sitter");
    parser
}

/// Attempts to parse a text fragment into Go declarations using a tiered strategy:
///
/// 1. Parse as a complete source file — handles top-level imports, funcs, types.
/// 2. Wrap in `package __weavr__; ` prefix — Go requires a package clause.
/// 3. Wrap in `package __weavr__; func __weavr__() { }` — for expression fragments.
/// 4. If all fail, return [`ParsedFragment::Unparsable`].
pub(super) fn parse_fragment(text: &str) -> ParsedFragment {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return ParsedFragment::Declarations(Vec::new());
    }

    // Bail out on build directives and CGO
    if contains_build_directives(trimmed) || contains_cgo(trimmed) {
        return ParsedFragment::Unparsable;
    }

    let mut parser = create_parser();

    // Strategy 1: parse as a complete source file
    if let Some(decls) = try_parse_as_source_file(&mut parser, trimmed) {
        return ParsedFragment::Declarations(decls);
    }

    // Strategy 2: wrap in package clause
    let wrapped_pkg = format!("package __weavr__\n{trimmed}");
    if let Some(decls) = try_parse_as_source_file(&mut parser, &wrapped_pkg) {
        // Filter out the synthetic package clause
        let decls = decls
            .into_iter()
            .filter(|d| !matches!(&d.identity, GoIdentity::Unknown(s) if s == "package __weavr__"))
            .collect::<Vec<_>>();
        if !decls.is_empty() {
            return ParsedFragment::Declarations(decls);
        }
    }

    // Strategy 3: wrap in package clause + function body
    let wrapped_func = format!("package __weavr__\nfunc __weavr__() {{\n{trimmed}\n}}");
    if let Some(decls) = try_parse_as_source_file(&mut parser, &wrapped_func) {
        // Extract the inner statements as-is — but this is rarely useful for Go
        // top-level merging, so we only use it as a last resort
        let inner: Vec<GoDeclaration> = decls
            .into_iter()
            .filter(|d| {
                !matches!(&d.identity, GoIdentity::Unknown(s) if s == "package __weavr__")
                    && !matches!(&d.identity, GoIdentity::Function(s) if s == "__weavr__")
            })
            .collect();
        if !inner.is_empty() {
            return ParsedFragment::Declarations(inner);
        }
    }

    ParsedFragment::Unparsable
}

/// Returns `true` if the text contains `//go:build` or `// +build` directives.
fn contains_build_directives(text: &str) -> bool {
    text.lines().any(|line| {
        let t = line.trim();
        t.starts_with("//go:build") || t.starts_with("// +build")
    })
}

/// Returns `true` if the text contains CGO blocks (`import "C"`).
fn contains_cgo(text: &str) -> bool {
    text.lines().any(|line| {
        let t = line.trim();
        t == "import \"C\"" || t == "import \"C\";"
    })
}

/// Tries to parse as a complete source file. Returns `None` if too many errors.
fn try_parse_as_source_file(parser: &mut Parser, text: &str) -> Option<Vec<GoDeclaration>> {
    let tree = parser.parse(text, None)?;
    let root = tree.root_node();

    if has_too_many_errors(&root) {
        return None;
    }

    let decls = extract_declarations(&root, text);
    if decls.is_empty() && !text.trim().is_empty() {
        return None;
    }

    Some(decls)
}

/// Returns `true` if >30% of root-level children are ERROR nodes.
fn has_too_many_errors(root: &Node<'_>) -> bool {
    let total = root.child_count();
    if total == 0 {
        return false;
    }
    let errors = (0..total)
        .filter_map(|i| root.child(i))
        .filter(|c| c.kind() == "ERROR" || c.is_error())
        .count();
    errors * 100 > total * 30
}

/// Extracts top-level declarations from a root node.
fn extract_declarations(root: &Node<'_>, source: &str) -> Vec<GoDeclaration> {
    let mut decls = Vec::new();
    for i in 0..root.child_count() {
        if let Some(child) = root.child(i) {
            if let Some(decl) = node_to_declaration(&child, source) {
                decls.push(decl);
            }
        }
    }
    decls
}

/// Converts a tree-sitter node into a `GoDeclaration`.
fn node_to_declaration(node: &Node<'_>, source: &str) -> Option<GoDeclaration> {
    let source_text = node_text(node, source).to_string();
    let kind_str = node.kind();

    match kind_str {
        "import_declaration" => parse_import_declaration(node, source),
        "function_declaration" => {
            let name = get_field_text(node, "name", source).unwrap_or_default();
            Some(GoDeclaration {
                kind: DeclarationKind::Function,
                identity: GoIdentity::Function(name),
                source_text,
                children: Vec::new(),
            })
        }
        "method_declaration" => {
            let name = get_field_text(node, "name", source).unwrap_or_default();
            let receiver = extract_receiver_type(node, source);
            Some(GoDeclaration {
                kind: DeclarationKind::Method,
                identity: GoIdentity::Method(receiver, name),
                source_text,
                children: Vec::new(),
            })
        }
        "type_declaration" => parse_type_declaration(node, source),
        "const_declaration" => {
            let name = extract_spec_names(node, source, "const_spec");
            Some(GoDeclaration {
                kind: DeclarationKind::Const,
                identity: GoIdentity::Const(name),
                source_text,
                children: Vec::new(),
            })
        }
        "var_declaration" => {
            let name = extract_spec_names(node, source, "var_spec");
            Some(GoDeclaration {
                kind: DeclarationKind::Var,
                identity: GoIdentity::Var(name),
                source_text,
                children: Vec::new(),
            })
        }
        "package_clause" => Some(GoDeclaration {
            kind: DeclarationKind::Function, // placeholder kind
            identity: GoIdentity::Unknown(source_text.clone()),
            source_text,
            children: Vec::new(),
        }),
        _ => None,
    }
}

/// Parses an `import_declaration` node into individual import declarations.
///
/// For grouped imports (`import (...)`), expands into one `GoDeclaration` per spec.
/// For single imports (`import "fmt"`), creates one declaration.
fn parse_import_declaration(node: &Node<'_>, source: &str) -> Option<GoDeclaration> {
    let mut children = Vec::new();

    for i in 0..node.child_count() {
        let Some(child) = node.child(i) else {
            continue;
        };
        match child.kind() {
            "import_spec" => {
                if let Some(import) = parse_import_spec(&child, source) {
                    children.push(import);
                }
            }
            "import_spec_list" => {
                for j in 0..child.child_count() {
                    let Some(spec) = child.child(j) else {
                        continue;
                    };
                    if spec.kind() == "import_spec" {
                        if let Some(import) = parse_import_spec(&spec, source) {
                            children.push(import);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    if children.is_empty() {
        return None;
    }

    // Use the full source text of the import declaration
    let source_text = node_text(node, source).to_string();

    // For single imports, use the child's identity directly
    let identity = if children.len() == 1 {
        children[0].identity.clone()
    } else {
        // For grouped imports, use a composite identity
        GoIdentity::Unknown(source_text.clone())
    };

    Some(GoDeclaration {
        kind: DeclarationKind::ImportDeclaration,
        identity,
        source_text,
        children,
    })
}

/// Parses a single `import_spec` node into a `GoDeclaration`.
fn parse_import_spec(node: &Node<'_>, source: &str) -> Option<GoDeclaration> {
    let path_node = node.child_by_field_name("path")?;
    let raw_path = node_text(&path_node, source);
    // Strip quotes from path
    let path = raw_path.trim_matches('"').trim_matches('`').to_string();

    let kind = if let Some(name_node) = node.child_by_field_name("name") {
        let name_text = node_text(&name_node, source);
        match name_text {
            "_" => ImportKind::Blank,
            "." => ImportKind::Dot,
            alias => ImportKind::Named(alias.to_string()),
        }
    } else {
        ImportKind::Normal
    };

    let key = ImportKey {
        path: path.clone(),
        kind,
    };

    Some(GoDeclaration {
        kind: DeclarationKind::ImportDeclaration,
        identity: GoIdentity::Import(key),
        source_text: node_text(node, source).to_string(),
        children: Vec::new(),
    })
}

/// Parses a `type_declaration` node. Extracts the type name and nested
/// members for structs and interfaces.
fn parse_type_declaration(node: &Node<'_>, source: &str) -> Option<GoDeclaration> {
    let source_text = node_text(node, source).to_string();

    // type_declaration -> type_spec | type_alias (possibly inside parenthesized group)
    for i in 0..node.child_count() {
        let Some(child) = node.child(i) else {
            continue;
        };
        match child.kind() {
            "type_spec" => {
                return parse_type_spec(&child, source, &source_text);
            }
            "type_alias" => {
                let name = get_field_text(&child, "name", source).unwrap_or_default();
                return Some(GoDeclaration {
                    kind: DeclarationKind::TypeDeclaration,
                    identity: GoIdentity::Type(name),
                    source_text,
                    children: Vec::new(),
                });
            }
            _ => {}
        }
    }

    // Fallback: try extracting name directly
    let name = get_field_text(node, "name", source).unwrap_or_default();
    Some(GoDeclaration {
        kind: DeclarationKind::TypeDeclaration,
        identity: GoIdentity::Type(name),
        source_text,
        children: Vec::new(),
    })
}

/// Parses a `type_spec` node, extracting struct fields or interface methods.
fn parse_type_spec(node: &Node<'_>, source: &str, full_source_text: &str) -> Option<GoDeclaration> {
    let name = get_field_text(node, "name", source).unwrap_or_default();

    // Get the type field to determine if this is a struct or interface
    let type_node = node.child_by_field_name("type")?;
    let children = match type_node.kind() {
        "struct_type" => extract_struct_fields(&type_node, source),
        "interface_type" => extract_interface_methods(&type_node, source),
        _ => Vec::new(),
    };

    Some(GoDeclaration {
        kind: DeclarationKind::TypeDeclaration,
        identity: GoIdentity::Type(name),
        source_text: full_source_text.to_string(),
        children,
    })
}

/// Extracts field declarations from a struct type.
fn extract_struct_fields(node: &Node<'_>, source: &str) -> Vec<GoDeclaration> {
    let mut fields = Vec::new();
    // struct_type -> field_declaration_list -> field_declaration*
    for i in 0..node.child_count() {
        let Some(child) = node.child(i) else {
            continue;
        };
        if child.kind() == "field_declaration_list" {
            for j in 0..child.child_count() {
                let Some(field) = child.child(j) else {
                    continue;
                };
                if field.kind() == "field_declaration" {
                    let field_name = get_field_text(&field, "name", source).unwrap_or_default();
                    let field_text = node_text(&field, source).to_string();
                    fields.push(GoDeclaration {
                        kind: DeclarationKind::Var, // reuse Var for struct fields
                        identity: GoIdentity::Var(field_name),
                        source_text: field_text,
                        children: Vec::new(),
                    });
                }
            }
        }
    }
    fields
}

/// Extracts method specifications from an interface type.
fn extract_interface_methods(node: &Node<'_>, source: &str) -> Vec<GoDeclaration> {
    let mut methods = Vec::new();
    for i in 0..node.child_count() {
        let Some(child) = node.child(i) else {
            continue;
        };
        match child.kind() {
            "method_elem" => {
                let name = get_field_text(&child, "name", source).unwrap_or_default();
                let method_text = node_text(&child, source).to_string();
                methods.push(GoDeclaration {
                    kind: DeclarationKind::Function, // reuse Function for interface methods
                    identity: GoIdentity::Function(name),
                    source_text: method_text,
                    children: Vec::new(),
                });
            }
            "type_elem" => {
                let text = node_text(&child, source).to_string();
                methods.push(GoDeclaration {
                    kind: DeclarationKind::TypeDeclaration,
                    identity: GoIdentity::Unknown(text.clone()),
                    source_text: text,
                    children: Vec::new(),
                });
            }
            _ => {}
        }
    }
    methods
}

/// Extracts the receiver type from a method declaration.
fn extract_receiver_type(node: &Node<'_>, source: &str) -> String {
    let Some(receiver) = node.child_by_field_name("receiver") else {
        return String::new();
    };
    // receiver is a parameter_list; look for the type inside
    for i in 0..receiver.child_count() {
        let Some(param) = receiver.child(i) else {
            continue;
        };
        if param.kind() == "parameter_declaration" {
            if let Some(type_node) = param.child_by_field_name("type") {
                let text = node_text(&type_node, source);
                // Strip pointer prefix for identity matching
                return text.trim_start_matches('*').to_string();
            }
        }
    }
    String::new()
}

/// Extracts names from spec children (const_spec, var_spec).
fn extract_spec_names(node: &Node<'_>, source: &str, spec_kind: &str) -> String {
    let mut names = Vec::new();
    for i in 0..node.child_count() {
        let Some(child) = node.child(i) else {
            continue;
        };
        if child.kind() == spec_kind {
            if let Some(name) = get_field_text(&child, "name", source) {
                names.push(name);
            }
        }
    }
    if names.is_empty() {
        // Fallback: use source text hash
        node_text(node, source).to_string()
    } else {
        names.join(",")
    }
}

/// Returns `true` if an import declaration contains dot imports.
pub(super) fn is_dot_import(decl: &GoDeclaration) -> bool {
    if decl.kind != DeclarationKind::ImportDeclaration {
        return false;
    }
    // Check the declaration itself
    if let GoIdentity::Import(ref key) = decl.identity {
        if key.kind == ImportKind::Dot {
            return true;
        }
    }
    // Check children (for grouped imports)
    decl.children.iter().any(|child| {
        if let GoIdentity::Import(ref key) = child.identity {
            key.kind == ImportKind::Dot
        } else {
            false
        }
    })
}

/// Gets the text of a named field on a tree-sitter node.
fn get_field_text(node: &Node<'_>, field: &str, source: &str) -> Option<String> {
    node.child_by_field_name(field)
        .map(|n| node_text(&n, source).to_string())
}

/// Extracts the text of a tree-sitter node from the source.
fn node_text<'a>(node: &Node<'_>, source: &'a str) -> &'a str {
    &source[node.start_byte()..node.end_byte()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single_import() {
        let text = "import \"fmt\"";
        match parse_fragment(text) {
            ParsedFragment::Declarations(decls) => {
                assert_eq!(decls.len(), 1);
                assert_eq!(decls[0].kind, DeclarationKind::ImportDeclaration);
                assert_eq!(decls[0].children.len(), 1);
            }
            ParsedFragment::Unparsable => panic!("should parse single import"),
        }
    }

    #[test]
    fn parse_grouped_imports() {
        let text = "import (\n\t\"fmt\"\n\t\"os\"\n)";
        match parse_fragment(text) {
            ParsedFragment::Declarations(decls) => {
                assert_eq!(decls.len(), 1);
                assert_eq!(decls[0].kind, DeclarationKind::ImportDeclaration);
                assert_eq!(decls[0].children.len(), 2);
            }
            ParsedFragment::Unparsable => panic!("should parse grouped imports"),
        }
    }

    #[test]
    fn parse_function() {
        let text = "func Foo() {}";
        match parse_fragment(text) {
            ParsedFragment::Declarations(decls) => {
                assert_eq!(decls.len(), 1);
                assert_eq!(decls[0].kind, DeclarationKind::Function);
                assert_eq!(decls[0].identity, GoIdentity::Function("Foo".to_string()));
            }
            ParsedFragment::Unparsable => panic!("should parse function"),
        }
    }

    #[test]
    fn parse_method() {
        let text = "func (s *MyStruct) Foo() {}";
        match parse_fragment(text) {
            ParsedFragment::Declarations(decls) => {
                assert_eq!(decls.len(), 1);
                assert_eq!(decls[0].kind, DeclarationKind::Method);
                assert_eq!(
                    decls[0].identity,
                    GoIdentity::Method("MyStruct".to_string(), "Foo".to_string())
                );
            }
            ParsedFragment::Unparsable => panic!("should parse method"),
        }
    }

    #[test]
    fn parse_struct() {
        let text = "type MyStruct struct {\n\tName string\n\tAge  int\n}";
        match parse_fragment(text) {
            ParsedFragment::Declarations(decls) => {
                assert_eq!(decls.len(), 1);
                assert_eq!(decls[0].kind, DeclarationKind::TypeDeclaration);
                assert_eq!(decls[0].identity, GoIdentity::Type("MyStruct".to_string()));
                assert_eq!(decls[0].children.len(), 2);
            }
            ParsedFragment::Unparsable => panic!("should parse struct"),
        }
    }

    #[test]
    fn parse_interface() {
        let text = "type Reader interface {\n\tRead(p []byte) (n int, err error)\n}";
        match parse_fragment(text) {
            ParsedFragment::Declarations(decls) => {
                assert_eq!(decls.len(), 1);
                assert_eq!(decls[0].kind, DeclarationKind::TypeDeclaration);
                assert_eq!(decls[0].identity, GoIdentity::Type("Reader".to_string()));
                assert_eq!(decls[0].children.len(), 1);
            }
            ParsedFragment::Unparsable => panic!("should parse interface"),
        }
    }

    #[test]
    fn parse_type_alias() {
        let text = "type ID = string";
        match parse_fragment(text) {
            ParsedFragment::Declarations(decls) => {
                assert_eq!(decls.len(), 1);
                assert_eq!(decls[0].kind, DeclarationKind::TypeDeclaration);
            }
            ParsedFragment::Unparsable => panic!("should parse type alias"),
        }
    }

    #[test]
    fn parse_const() {
        let text = "const MaxRetries = 3";
        match parse_fragment(text) {
            ParsedFragment::Declarations(decls) => {
                assert_eq!(decls.len(), 1);
                assert_eq!(decls[0].kind, DeclarationKind::Const);
            }
            ParsedFragment::Unparsable => panic!("should parse const"),
        }
    }

    #[test]
    fn parse_var() {
        let text = "var ErrNotFound = errors.New(\"not found\")";
        match parse_fragment(text) {
            ParsedFragment::Declarations(decls) => {
                assert_eq!(decls.len(), 1);
                assert_eq!(decls[0].kind, DeclarationKind::Var);
            }
            ParsedFragment::Unparsable => panic!("should parse var"),
        }
    }

    #[test]
    fn parse_empty_input() {
        match parse_fragment("") {
            ParsedFragment::Declarations(decls) => assert!(decls.is_empty()),
            ParsedFragment::Unparsable => panic!("empty input should produce empty declarations"),
        }
    }

    #[test]
    fn parse_whitespace_only() {
        match parse_fragment("   \n  \t  ") {
            ParsedFragment::Declarations(decls) => assert!(decls.is_empty()),
            ParsedFragment::Unparsable => {
                panic!("whitespace should produce empty declarations")
            }
        }
    }

    #[test]
    fn build_directive_returns_unparsable() {
        let text = "//go:build linux\n\nimport \"fmt\"";
        assert!(matches!(parse_fragment(text), ParsedFragment::Unparsable));
    }

    #[test]
    fn legacy_build_tag_returns_unparsable() {
        let text = "// +build linux\n\nimport \"fmt\"";
        assert!(matches!(parse_fragment(text), ParsedFragment::Unparsable));
    }

    #[test]
    fn cgo_import_returns_unparsable() {
        let text = "import \"C\"";
        assert!(matches!(parse_fragment(text), ParsedFragment::Unparsable));
    }

    #[test]
    fn named_import_detected() {
        let text = "import f \"fmt\"";
        match parse_fragment(text) {
            ParsedFragment::Declarations(decls) => {
                assert_eq!(decls.len(), 1);
                let child = &decls[0].children[0];
                assert!(matches!(
                    &child.identity,
                    GoIdentity::Import(ImportKey { kind: ImportKind::Named(alias), .. }) if alias == "f"
                ));
            }
            ParsedFragment::Unparsable => panic!("should parse named import"),
        }
    }

    #[test]
    fn blank_import_detected() {
        let text = "import _ \"database/sql\"";
        match parse_fragment(text) {
            ParsedFragment::Declarations(decls) => {
                assert_eq!(decls.len(), 1);
                let child = &decls[0].children[0];
                assert!(matches!(
                    &child.identity,
                    GoIdentity::Import(ImportKey {
                        kind: ImportKind::Blank,
                        ..
                    })
                ));
            }
            ParsedFragment::Unparsable => panic!("should parse blank import"),
        }
    }

    #[test]
    fn dot_import_detection() {
        let text = "import . \"fmt\"";
        match parse_fragment(text) {
            ParsedFragment::Declarations(decls) => {
                assert!(is_dot_import(&decls[0]));
            }
            ParsedFragment::Unparsable => panic!("should parse dot import"),
        }
    }

    #[test]
    fn parse_mixed_declarations() {
        let text = "\
import \"fmt\"

func Hello() {
\tfmt.Println(\"hello\")
}

type Config struct {
\tName string
}";
        match parse_fragment(text) {
            ParsedFragment::Declarations(decls) => {
                assert_eq!(decls.len(), 3);
                assert_eq!(decls[0].kind, DeclarationKind::ImportDeclaration);
                assert_eq!(decls[1].kind, DeclarationKind::Function);
                assert_eq!(decls[2].kind, DeclarationKind::TypeDeclaration);
            }
            ParsedFragment::Unparsable => panic!("should parse mixed declarations"),
        }
    }
}
