//! Tiered fragment parsing for C# source code.
//!
//! Conflict hunks often contain partial C# code (e.g., a few `using` directives
//! or method bodies without the surrounding class). This module tries multiple
//! parsing strategies to extract valid declarations using tree-sitter.

use tree_sitter::{Node, Parser};

use super::identity::{CSharpIdentity, MemberIdentity};

/// The result of attempting to parse a code fragment.
pub(super) enum ParsedFragment {
    /// Successfully parsed into a list of declarations.
    Declarations(Vec<CSharpDeclaration>),
    /// Could not be parsed by any strategy.
    Unparsable,
}

/// A single C# declaration extracted from the parse tree.
#[derive(Debug, Clone)]
pub(super) struct CSharpDeclaration {
    /// The kind of declaration.
    pub kind: DeclarationKind,
    /// Identity for matching across conflict sides.
    pub identity: CSharpIdentity,
    /// The original source text of this declaration.
    pub source_text: String,
    /// Nested members (for classes, structs, namespaces, etc.).
    pub children: Vec<CSharpDeclaration>,
}

/// The kind of C# declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DeclarationKind {
    /// `using System;`
    UsingDirective,
    /// `namespace Foo { ... }` or `namespace Foo;`
    Namespace,
    /// `class Foo { ... }`
    Class,
    /// `struct Foo { ... }`
    Struct,
    /// `interface IFoo { ... }`
    Interface,
    /// `enum Foo { ... }`
    Enum,
    /// `delegate void Foo();`
    Delegate,
    /// `record Foo(...);`
    Record,
    /// Method or function member
    Method,
    /// Property declaration
    Property,
    /// Field declaration
    Field,
    /// Constructor
    Constructor,
    /// Event declaration
    Event,
    /// Indexer declaration
    Indexer,
}

fn create_parser() -> Parser {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_c_sharp::LANGUAGE.into())
        .expect("failed to set C# language for tree-sitter");
    parser
}

/// Attempts to parse a text fragment into C# declarations using a tiered strategy:
///
/// 1. Parse as a complete compilation unit -- handles top-level using/namespace/class.
/// 2. Wrap in a dummy namespace + class -- handles bare method/property fragments.
/// 3. Wrap in a dummy namespace only -- handles bare class-level declarations.
/// 4. If all fail, return [`ParsedFragment::Unparsable`].
pub(super) fn parse_fragment(text: &str) -> ParsedFragment {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return ParsedFragment::Declarations(Vec::new());
    }

    // Bail out on preprocessor directives -- they affect conditional compilation
    if contains_preprocessor_directives(trimmed) {
        return ParsedFragment::Unparsable;
    }

    let mut parser = create_parser();

    // Strategy 1: parse as a complete compilation unit
    if let Some(decls) = try_parse_as_compilation_unit(&mut parser, trimmed) {
        return ParsedFragment::Declarations(decls);
    }

    // Strategy 2: wrap in dummy namespace only (for bare class-level declarations)
    let wrapped_ns = format!("namespace __WeavrDummy__ {{ {trimmed} }}");
    if let Some(decls) = try_parse_wrapped(&mut parser, &wrapped_ns, trimmed) {
        return ParsedFragment::Declarations(decls);
    }

    // Strategy 3: wrap in dummy namespace + class (for bare member fragments)
    let wrapped_full =
        format!("namespace __WeavrDummy__ {{ class __WeavrDummy__ {{ {trimmed} }} }}");
    if let Some(decls) = try_parse_wrapped_members(&mut parser, &wrapped_full, trimmed) {
        return ParsedFragment::Declarations(decls);
    }

    ParsedFragment::Unparsable
}

/// Returns `true` if the text contains preprocessor directives that would
/// affect conditional compilation structure.
fn contains_preprocessor_directives(text: &str) -> bool {
    text.lines().any(|line| {
        let t = line.trim();
        t.starts_with("#if")
            || t.starts_with("#else")
            || t.starts_with("#elif")
            || t.starts_with("#endif")
            || t.starts_with("#region")
            || t.starts_with("#endregion")
    })
}

/// Tries to parse as a complete compilation unit. Returns `None` if too many errors.
fn try_parse_as_compilation_unit(
    parser: &mut Parser,
    text: &str,
) -> Option<Vec<CSharpDeclaration>> {
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

/// Tries to parse wrapped text and extract the inner declarations (stripping the wrapper).
fn try_parse_wrapped(
    parser: &mut Parser,
    wrapped: &str,
    _original: &str,
) -> Option<Vec<CSharpDeclaration>> {
    let tree = parser.parse(wrapped, None)?;
    let root = tree.root_node();

    if has_too_many_errors(&root) {
        return None;
    }

    // Find the dummy namespace and extract its children
    for i in 0..root.child_count() {
        let child = root.child(i)?;
        if child.kind() == "namespace_declaration" {
            let name = get_name_text(&child, wrapped);
            if name.as_deref() == Some("__WeavrDummy__") {
                // Extract the body declarations, using original source text
                if let Some(body) = child.child_by_field_name("body") {
                    let mut decls = Vec::new();
                    for j in 0..body.child_count() {
                        if let Some(member) = body.child(j) {
                            if member.kind() == "{" || member.kind() == "}" {
                                continue;
                            }
                            if let Some(decl) = node_to_declaration(&member, wrapped) {
                                // Re-extract with the original source offsets mapped back
                                decls.push(remap_declaration_source(decl, wrapped));
                            }
                        }
                    }
                    if !decls.is_empty() {
                        return Some(decls);
                    }
                }
            }
        }
    }

    None
}

/// Tries to parse wrapped-in-class text and extract inner member declarations.
fn try_parse_wrapped_members(
    parser: &mut Parser,
    wrapped: &str,
    _original: &str,
) -> Option<Vec<CSharpDeclaration>> {
    let tree = parser.parse(wrapped, None)?;
    let root = tree.root_node();

    if has_too_many_errors(&root) {
        return None;
    }

    // Navigate: root -> namespace_declaration -> body -> class_declaration -> body
    for i in 0..root.child_count() {
        let ns = root.child(i)?;
        if ns.kind() != "namespace_declaration" {
            continue;
        }
        let ns_body = ns.child_by_field_name("body")?;
        for j in 0..ns_body.child_count() {
            let class = ns_body.child(j)?;
            if class.kind() != "class_declaration" {
                continue;
            }
            let class_body = class.child_by_field_name("body")?;
            let mut decls = Vec::new();
            for k in 0..class_body.child_count() {
                if let Some(member) = class_body.child(k) {
                    if member.kind() == "{" || member.kind() == "}" {
                        continue;
                    }
                    if let Some(decl) = node_to_declaration(&member, wrapped) {
                        decls.push(remap_declaration_source(decl, wrapped));
                    }
                }
            }
            if !decls.is_empty() {
                return Some(decls);
            }
        }
    }

    None
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
fn extract_declarations(root: &Node<'_>, source: &str) -> Vec<CSharpDeclaration> {
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

/// Converts a tree-sitter node into a `CSharpDeclaration`.
fn node_to_declaration(node: &Node<'_>, source: &str) -> Option<CSharpDeclaration> {
    let source_text = node_text(node, source).to_string();
    let kind_str = node.kind();

    match kind_str {
        "using_directive" => {
            let path = extract_using_path(node, source);
            Some(CSharpDeclaration {
                kind: DeclarationKind::UsingDirective,
                identity: CSharpIdentity::Using(path),
                source_text,
                children: Vec::new(),
            })
        }
        "namespace_declaration" | "file_scoped_namespace_declaration" => {
            let name = get_name_text(node, source).unwrap_or_default();
            let children = extract_body_members(node, source);
            Some(CSharpDeclaration {
                kind: DeclarationKind::Namespace,
                identity: CSharpIdentity::Namespace(name),
                source_text,
                children,
            })
        }
        "class_declaration" => parse_partial_type_node(node, source, DeclarationKind::Class),
        "struct_declaration" => parse_partial_type_node(node, source, DeclarationKind::Struct),
        "interface_declaration" => {
            let name = get_declaration_name(node, source);
            let children = extract_body_members(node, source);
            Some(CSharpDeclaration {
                kind: DeclarationKind::Interface,
                identity: CSharpIdentity::Interface(name),
                source_text,
                children,
            })
        }
        "enum_declaration" => {
            let name = get_declaration_name(node, source);
            Some(CSharpDeclaration {
                kind: DeclarationKind::Enum,
                identity: CSharpIdentity::Enum(name),
                source_text,
                children: Vec::new(),
            })
        }
        "delegate_declaration" => {
            let name = get_declaration_name(node, source);
            Some(CSharpDeclaration {
                kind: DeclarationKind::Delegate,
                identity: CSharpIdentity::Delegate(name),
                source_text,
                children: Vec::new(),
            })
        }
        "record_declaration" | "record_struct_declaration" => {
            let name = get_declaration_name(node, source);
            Some(CSharpDeclaration {
                kind: DeclarationKind::Record,
                identity: CSharpIdentity::Record(name),
                source_text,
                children: Vec::new(),
            })
        }
        _ => parse_member_node(node, source, kind_str),
    }
}

/// Parses a class or struct that may have the `partial` modifier (bails out if partial).
fn parse_partial_type_node(
    node: &Node<'_>,
    source: &str,
    decl_kind: DeclarationKind,
) -> Option<CSharpDeclaration> {
    if is_partial(node, source) {
        return None;
    }
    let name = get_declaration_name(node, source);
    let children = extract_body_members(node, source);
    let identity = match decl_kind {
        DeclarationKind::Class => CSharpIdentity::Class(name),
        DeclarationKind::Struct => CSharpIdentity::Struct(name),
        _ => return None,
    };
    Some(CSharpDeclaration {
        kind: decl_kind,
        identity,
        source_text: node_text(node, source).to_string(),
        children,
    })
}

/// Parses a member-level node (method, property, field, etc.).
fn parse_member_node(node: &Node<'_>, source: &str, kind_str: &str) -> Option<CSharpDeclaration> {
    let source_text = node_text(node, source).to_string();
    let (kind, identity) = match kind_str {
        "method_declaration" => {
            let name = get_declaration_name(node, source);
            let param_count = count_parameters(node);
            (
                DeclarationKind::Method,
                CSharpIdentity::Unknown(format!("method:{name}/{param_count}")),
            )
        }
        "property_declaration" => {
            let name = get_declaration_name(node, source);
            (
                DeclarationKind::Property,
                CSharpIdentity::Unknown(format!("property:{name}")),
            )
        }
        "field_declaration" => {
            let name = extract_field_name(node, source);
            (
                DeclarationKind::Field,
                CSharpIdentity::Unknown(format!("field:{name}")),
            )
        }
        "constructor_declaration" => {
            let param_count = count_parameters(node);
            (
                DeclarationKind::Constructor,
                CSharpIdentity::Unknown(format!("ctor/{param_count}")),
            )
        }
        "event_declaration" | "event_field_declaration" => {
            let name = get_declaration_name(node, source);
            (
                DeclarationKind::Event,
                CSharpIdentity::Unknown(format!("event:{name}")),
            )
        }
        "indexer_declaration" => {
            let param_count = count_parameters(node);
            (
                DeclarationKind::Indexer,
                CSharpIdentity::Unknown(format!("indexer/{param_count}")),
            )
        }
        _ => return None,
    };
    Some(CSharpDeclaration {
        kind,
        identity,
        source_text,
        children: Vec::new(),
    })
}

/// Gets the name identifier text of a declaration node.
fn get_declaration_name(node: &Node<'_>, source: &str) -> String {
    // Try "name" field first (most common)
    if let Some(name_node) = node.child_by_field_name("name") {
        let name = node_text(&name_node, source).to_string();
        // Check for type parameters (generic arity)
        if let Some(type_params) = node.child_by_field_name("type_parameters") {
            let arity = count_type_params(&type_params);
            return format!("{name}`{arity}");
        }
        return name;
    }
    // Fallback: first identifier child
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "identifier" {
                return node_text(&child, source).to_string();
            }
        }
    }
    String::new()
}

/// Gets the name text from a namespace or `qualified_name` node.
fn get_name_text(node: &Node<'_>, source: &str) -> Option<String> {
    if let Some(name_node) = node.child_by_field_name("name") {
        return Some(node_text(&name_node, source).to_string());
    }
    None
}

/// Extracts the normalized using path from a `using_directive` node.
fn extract_using_path(node: &Node<'_>, source: &str) -> String {
    // The using directive text is like "using System.IO;"
    // We need to extract just the path part
    let text = node_text(node, source);
    let text = text.trim();

    // Check for complex usings that we should bail out on
    // These are handled at the merge level, but we still extract the path for identity
    let stripped = text
        .strip_prefix("using")
        .unwrap_or(text)
        .trim()
        .trim_end_matches(';')
        .trim();

    stripped.to_string()
}

/// Checks if a using directive is "complex" (static, alias, global).
pub(super) fn is_complex_using(decl: &CSharpDeclaration) -> bool {
    if decl.kind != DeclarationKind::UsingDirective {
        return false;
    }
    let text = decl.source_text.trim();
    text.starts_with("using static ")
        || text.starts_with("global using ")
        || text.contains(" = ")
        || text.starts_with('[') // attributed using
}

/// Counts parameters in a method/constructor/indexer node.
fn count_parameters(node: &Node<'_>) -> usize {
    if let Some(params) = node.child_by_field_name("parameters") {
        let mut count = 0;
        for i in 0..params.child_count() {
            if let Some(child) = params.child(i) {
                if child.kind() == "parameter" {
                    count += 1;
                }
            }
        }
        count
    } else {
        0
    }
}

/// Counts type parameters for generic arity.
fn count_type_params(type_params: &Node<'_>) -> usize {
    let mut count = 0;
    for i in 0..type_params.child_count() {
        if let Some(child) = type_params.child(i) {
            if child.kind() == "type_parameter" {
                count += 1;
            }
        }
    }
    count
}

/// Extracts the variable name from a field declaration.
fn extract_field_name(node: &Node<'_>, source: &str) -> String {
    // Field declarations have a variable_declaration child with variable_declarator children
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "variable_declaration" {
                for j in 0..child.child_count() {
                    if let Some(declarator) = child.child(j) {
                        if declarator.kind() == "variable_declarator" {
                            return get_declaration_name(&declarator, source);
                        }
                    }
                }
            }
        }
    }
    String::new()
}

/// Checks if a type declaration has the `partial` modifier.
fn is_partial(node: &Node<'_>, source: &str) -> bool {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if (child.kind() == "modifier" || child.kind() == "partial")
                && node_text(&child, source) == "partial"
            {
                return true;
            }
        }
    }
    false
}

/// Extracts member declarations from a class/struct/interface/namespace body.
fn extract_body_members(node: &Node<'_>, source: &str) -> Vec<CSharpDeclaration> {
    let mut members = Vec::new();

    let Some(body) = node.child_by_field_name("body") else {
        // For file-scoped namespaces, members are siblings after the semicolon
        return Vec::new();
    };

    for i in 0..body.child_count() {
        if let Some(child) = body.child(i) {
            if child.kind() == "{" || child.kind() == "}" {
                continue;
            }
            if let Some(decl) = node_to_member_declaration(&child, source) {
                members.push(decl);
            }
        }
    }

    members
}

/// Converts a tree-sitter node into a member-level `CSharpDeclaration`.
fn node_to_member_declaration(node: &Node<'_>, source: &str) -> Option<CSharpDeclaration> {
    let source_text = node_text(node, source).to_string();
    let kind = node.kind();

    match kind {
        "method_declaration" => {
            let name = get_declaration_name(node, source);
            let param_count = count_parameters(node);
            Some(CSharpDeclaration {
                kind: DeclarationKind::Method,
                identity: CSharpIdentity::Unknown(format!("method:{name}/{param_count}")),
                source_text,
                children: Vec::new(),
            })
        }
        "property_declaration" => {
            let name = get_declaration_name(node, source);
            Some(CSharpDeclaration {
                kind: DeclarationKind::Property,
                identity: CSharpIdentity::Unknown(format!("property:{name}")),
                source_text,
                children: Vec::new(),
            })
        }
        "field_declaration" => {
            let name = extract_field_name(node, source);
            Some(CSharpDeclaration {
                kind: DeclarationKind::Field,
                identity: CSharpIdentity::Unknown(format!("field:{name}")),
                source_text,
                children: Vec::new(),
            })
        }
        "constructor_declaration" => {
            let param_count = count_parameters(node);
            Some(CSharpDeclaration {
                kind: DeclarationKind::Constructor,
                identity: CSharpIdentity::Unknown(format!("ctor/{param_count}")),
                source_text,
                children: Vec::new(),
            })
        }
        "event_declaration" | "event_field_declaration" => {
            let name = get_declaration_name(node, source);
            Some(CSharpDeclaration {
                kind: DeclarationKind::Event,
                identity: CSharpIdentity::Unknown(format!("event:{name}")),
                source_text,
                children: Vec::new(),
            })
        }
        "indexer_declaration" => {
            let param_count = count_parameters(node);
            Some(CSharpDeclaration {
                kind: DeclarationKind::Indexer,
                identity: CSharpIdentity::Unknown(format!("indexer/{param_count}")),
                source_text,
                children: Vec::new(),
            })
        }
        // Nested type declarations
        "class_declaration"
        | "struct_declaration"
        | "interface_declaration"
        | "enum_declaration"
        | "delegate_declaration"
        | "record_declaration"
        | "record_struct_declaration" => node_to_declaration(node, source),
        _ => None,
    }
}

/// Extracts the text of a tree-sitter node from the source.
fn node_text<'a>(node: &Node<'_>, source: &'a str) -> &'a str {
    &source[node.start_byte()..node.end_byte()]
}

/// Remaps source text in a declaration (used when parsing wrapped fragments).
fn remap_declaration_source(mut decl: CSharpDeclaration, _wrapped: &str) -> CSharpDeclaration {
    // The source_text already points to the right text from the wrapped source,
    // which is fine since we preserve it as-is. The identity is already correct.
    // For children, remap recursively.
    decl.children = decl
        .children
        .into_iter()
        .map(|c| remap_declaration_source(c, _wrapped))
        .collect();
    decl
}

/// Converts a member-level `CSharpDeclaration` identity into a `MemberIdentity`.
pub(super) fn to_member_identity(decl: &CSharpDeclaration) -> MemberIdentity {
    match decl.kind {
        DeclarationKind::Method => {
            // Parse from identity string "method:Name/paramCount"
            if let CSharpIdentity::Unknown(ref s) = decl.identity {
                if let Some(rest) = s.strip_prefix("method:") {
                    if let Some((name, count_str)) = rest.rsplit_once('/') {
                        let count = count_str.parse().unwrap_or(0);
                        return MemberIdentity::Method(name.to_string(), count);
                    }
                }
            }
            MemberIdentity::Unknown(decl.source_text.clone())
        }
        DeclarationKind::Property => {
            if let CSharpIdentity::Unknown(ref s) = decl.identity {
                if let Some(name) = s.strip_prefix("property:") {
                    return MemberIdentity::Property(name.to_string());
                }
            }
            MemberIdentity::Unknown(decl.source_text.clone())
        }
        DeclarationKind::Field => {
            if let CSharpIdentity::Unknown(ref s) = decl.identity {
                if let Some(name) = s.strip_prefix("field:") {
                    return MemberIdentity::Field(name.to_string());
                }
            }
            MemberIdentity::Unknown(decl.source_text.clone())
        }
        DeclarationKind::Constructor => {
            if let CSharpIdentity::Unknown(ref s) = decl.identity {
                if let Some(count_str) = s.strip_prefix("ctor/") {
                    let count = count_str.parse().unwrap_or(0);
                    return MemberIdentity::Constructor(count);
                }
            }
            MemberIdentity::Unknown(decl.source_text.clone())
        }
        DeclarationKind::Event => {
            if let CSharpIdentity::Unknown(ref s) = decl.identity {
                if let Some(name) = s.strip_prefix("event:") {
                    return MemberIdentity::Event(name.to_string());
                }
            }
            MemberIdentity::Unknown(decl.source_text.clone())
        }
        DeclarationKind::Indexer => {
            if let CSharpIdentity::Unknown(ref s) = decl.identity {
                if let Some(count_str) = s.strip_prefix("indexer/") {
                    let count = count_str.parse().unwrap_or(0);
                    return MemberIdentity::Indexer(count);
                }
            }
            MemberIdentity::Unknown(decl.source_text.clone())
        }
        _ => MemberIdentity::Unknown(decl.source_text.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_using_statements() {
        let text = "using System;\nusing System.IO;";
        let result = parse_fragment(text);
        match result {
            ParsedFragment::Declarations(decls) => {
                assert_eq!(decls.len(), 2);
                assert_eq!(decls[0].kind, DeclarationKind::UsingDirective);
                assert_eq!(decls[1].kind, DeclarationKind::UsingDirective);
            }
            ParsedFragment::Unparsable => panic!("should parse using statements"),
        }
    }

    #[test]
    fn parse_class() {
        let text = "class Foo { public void Bar() { } }";
        let result = parse_fragment(text);
        match result {
            ParsedFragment::Declarations(decls) => {
                assert_eq!(decls.len(), 1);
                assert_eq!(decls[0].kind, DeclarationKind::Class);
            }
            ParsedFragment::Unparsable => panic!("should parse class"),
        }
    }

    #[test]
    fn parse_empty_input() {
        let result = parse_fragment("");
        match result {
            ParsedFragment::Declarations(decls) => assert!(decls.is_empty()),
            ParsedFragment::Unparsable => panic!("empty input should produce empty declarations"),
        }
    }

    #[test]
    fn parse_whitespace_only() {
        let result = parse_fragment("   \n  \t  ");
        match result {
            ParsedFragment::Declarations(decls) => assert!(decls.is_empty()),
            ParsedFragment::Unparsable => {
                panic!("whitespace should produce empty declarations")
            }
        }
    }

    #[test]
    fn parse_preprocessor_returns_unparsable() {
        let text = "#if DEBUG\nusing System;\n#endif";
        let result = parse_fragment(text);
        assert!(matches!(result, ParsedFragment::Unparsable));
    }

    #[test]
    fn parse_namespace_with_class() {
        let text = "namespace Foo {\n    class Bar { }\n}";
        let result = parse_fragment(text);
        match result {
            ParsedFragment::Declarations(decls) => {
                assert_eq!(decls.len(), 1);
                assert_eq!(decls[0].kind, DeclarationKind::Namespace);
                assert_eq!(decls[0].children.len(), 1);
            }
            ParsedFragment::Unparsable => panic!("should parse namespace"),
        }
    }

    #[test]
    fn partial_class_returns_none() {
        let text = "partial class Foo { }";
        let result = parse_fragment(text);
        match result {
            ParsedFragment::Declarations(decls) => {
                // partial class should be skipped (returns None from node_to_declaration)
                assert!(decls.is_empty());
            }
            ParsedFragment::Unparsable => { /* also acceptable */ }
        }
    }

    #[test]
    fn complex_using_detection() {
        let static_using = CSharpDeclaration {
            kind: DeclarationKind::UsingDirective,
            identity: CSharpIdentity::Using("static System.Math".to_string()),
            source_text: "using static System.Math;".to_string(),
            children: Vec::new(),
        };
        assert!(is_complex_using(&static_using));

        let alias_using = CSharpDeclaration {
            kind: DeclarationKind::UsingDirective,
            identity: CSharpIdentity::Using("Alias = System.IO".to_string()),
            source_text: "using Alias = System.IO;".to_string(),
            children: Vec::new(),
        };
        assert!(is_complex_using(&alias_using));

        let simple_using = CSharpDeclaration {
            kind: DeclarationKind::UsingDirective,
            identity: CSharpIdentity::Using("System".to_string()),
            source_text: "using System;".to_string(),
            children: Vec::new(),
        };
        assert!(!is_complex_using(&simple_using));
    }
}
