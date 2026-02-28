//! Tiered fragment parsing for TypeScript/TSX source code.
//!
//! Conflict hunks often contain partial TypeScript code (e.g., a few `import`
//! statements or declaration bodies without the surrounding module). This module
//! tries multiple parsing strategies to extract valid declarations using tree-sitter.

use std::collections::BTreeSet;

use tree_sitter::{Node, Parser};

use super::identity::{ImportKey, ImportKind, ImportSpecifier, TsIdentity};

/// The result of attempting to parse a code fragment.
pub(super) enum ParsedFragment {
    /// Successfully parsed into a list of declarations.
    Declarations(Vec<TsDeclaration>),
    /// Could not be parsed by any strategy.
    Unparsable,
}

/// A single TypeScript declaration extracted from the parse tree.
#[derive(Debug, Clone)]
pub(super) struct TsDeclaration {
    /// The kind of declaration.
    pub kind: DeclarationKind,
    /// Identity for matching across conflict sides.
    pub identity: TsIdentity,
    /// The original source text of this declaration.
    pub source_text: String,
    /// Import specifiers (only populated for import statements).
    pub specifiers: BTreeSet<ImportSpecifier>,
}

/// The kind of TypeScript declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DeclarationKind {
    /// `import { A } from 'x'`
    ImportStatement,
    /// `export { A } from 'x'`
    ExportStatement,
    /// `function foo() { }`
    Function,
    /// `class Foo { }`
    Class,
    /// `interface IFoo { }`
    Interface,
    /// `type Foo = ...`
    TypeAlias,
    /// `enum Foo { }`
    Enum,
    /// `const/let/var foo = ...`
    Variable,
    /// `namespace Foo { }` or `module Foo { }`
    Namespace,
}

fn create_parser() -> Parser {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_typescript::LANGUAGE_TSX.into())
        .expect("failed to set TypeScript/TSX language for tree-sitter");
    parser
}

/// Attempts to parse a text fragment into TypeScript declarations using a tiered strategy:
///
/// 1. Parse as a complete module -- handles top-level imports/exports/declarations.
/// 2. Wrap in a dummy namespace -- handles bare class-level declarations.
/// 3. Wrap in a dummy namespace + class -- handles bare method fragments.
/// 4. If all fail, return [`ParsedFragment::Unparsable`].
pub(super) fn parse_fragment(text: &str) -> ParsedFragment {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return ParsedFragment::Declarations(Vec::new());
    }

    // Bail out on triple-slash reference directives and dynamic import() expressions
    if contains_bail_out_patterns(trimmed) {
        return ParsedFragment::Unparsable;
    }

    let mut parser = create_parser();

    // Strategy 1: parse as a complete module
    if let Some(decls) = try_parse_as_module(&mut parser, trimmed) {
        return ParsedFragment::Declarations(decls);
    }

    // Strategy 2: wrap in a dummy namespace (for bare class-level declarations)
    let wrapped_ns = format!("namespace __WeavrDummy__ {{ {trimmed} }}");
    if let Some(decls) = try_parse_wrapped(&mut parser, &wrapped_ns) {
        return ParsedFragment::Declarations(decls);
    }

    // Strategy 3: wrap in a dummy namespace + class (for bare member fragments)
    let wrapped_full =
        format!("namespace __WeavrDummy__ {{ class __WeavrDummy__ {{ {trimmed} }} }}");
    if let Some(decls) = try_parse_wrapped_class(&mut parser, &wrapped_full) {
        return ParsedFragment::Declarations(decls);
    }

    ParsedFragment::Unparsable
}

/// Returns `true` if the text contains patterns we bail out on:
/// - Triple-slash reference directives (`/// <reference .../>`)
///
/// Dynamic `import()` expressions are handled separately in `parse_import_statement`,
/// which returns `ImportParseResult::Unsupported` and propagates via `NodeConversion::BailOut`.
fn contains_bail_out_patterns(text: &str) -> bool {
    text.lines().any(|line| {
        let t = line.trim();
        // Triple-slash reference directives
        t.starts_with("/// <reference")
    })
}

/// Tries to parse as a complete module. Returns `None` if too many errors.
fn try_parse_as_module(parser: &mut Parser, text: &str) -> Option<Vec<TsDeclaration>> {
    let tree = parser.parse(text, None)?;
    let root = tree.root_node();

    if has_too_many_errors(&root) {
        return None;
    }

    let decls = extract_declarations(&root, text)?;
    if decls.is_empty() && !text.trim().is_empty() {
        return None;
    }

    Some(decls)
}

/// Tries to parse wrapped text and extract inner declarations (stripping the namespace wrapper).
fn try_parse_wrapped(parser: &mut Parser, wrapped: &str) -> Option<Vec<TsDeclaration>> {
    let tree = parser.parse(wrapped, None)?;
    let root = tree.root_node();

    if has_too_many_errors(&root) {
        return None;
    }

    // Find the dummy module declaration and extract its body
    for i in 0..root.child_count() {
        let Some(child) = root.child(i) else {
            continue;
        };
        if is_module_or_namespace(&child) {
            let name = get_name_text(&child, wrapped);
            if name.as_deref() == Some("__WeavrDummy__") {
                if let Some(body) = child.child_by_field_name("body") {
                    let mut decls = Vec::new();
                    for j in 0..body.child_count() {
                        if let Some(member) = body.child(j) {
                            if member.kind() == "{" || member.kind() == "}" {
                                continue;
                            }
                            match node_to_declaration_inner(&member, wrapped) {
                                NodeConversion::Ok(decl) => decls.push(decl),
                                NodeConversion::BailOut => return None,
                                NodeConversion::Skip => {}
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
fn try_parse_wrapped_class(parser: &mut Parser, wrapped: &str) -> Option<Vec<TsDeclaration>> {
    let tree = parser.parse(wrapped, None)?;
    let root = tree.root_node();

    if has_too_many_errors(&root) {
        return None;
    }

    // Navigate: root -> module_declaration -> body -> class_declaration -> body
    for i in 0..root.child_count() {
        let Some(ns) = root.child(i) else {
            continue;
        };
        if !is_module_or_namespace(&ns) {
            continue;
        }
        let Some(ns_body) = ns.child_by_field_name("body") else {
            continue;
        };
        for j in 0..ns_body.child_count() {
            let Some(class) = ns_body.child(j) else {
                continue;
            };
            if class.kind() != "class_declaration" {
                continue;
            }
            let Some(class_body) = class.child_by_field_name("body") else {
                continue;
            };
            let mut decls = Vec::new();
            for k in 0..class_body.child_count() {
                if let Some(member) = class_body.child(k) {
                    if member.kind() == "{" || member.kind() == "}" {
                        continue;
                    }
                    match node_to_declaration_inner(&member, wrapped) {
                        NodeConversion::Ok(decl) => decls.push(decl),
                        NodeConversion::BailOut => return None,
                        NodeConversion::Skip => {}
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
///
/// Returns `None` if any node triggers a bail-out (e.g., unsupported import form),
/// signaling that the entire fragment should be treated as unparsable.
fn extract_declarations(root: &Node<'_>, source: &str) -> Option<Vec<TsDeclaration>> {
    let mut decls = Vec::new();
    for i in 0..root.child_count() {
        if let Some(child) = root.child(i) {
            match node_to_declaration_inner(&child, source) {
                NodeConversion::Ok(decl) => decls.push(decl),
                NodeConversion::BailOut => return None,
                NodeConversion::Skip => {}
            }
        }
    }
    Some(decls)
}

/// Result of converting a tree-sitter node into a declaration.
enum NodeConversion {
    /// Successfully converted to a declaration.
    Ok(TsDeclaration),
    /// The node contains an unsupported import form — the entire fragment is unparsable.
    BailOut,
    /// The node was not recognized or is not a declaration (e.g., punctuation, comments).
    Skip,
}

/// Converts a tree-sitter node into a `TsDeclaration`.
fn node_to_declaration_inner(node: &Node<'_>, source: &str) -> NodeConversion {
    let source_text = node_text(node, source).to_string();
    let kind_str = node.kind();

    match kind_str {
        "import_statement" => match parse_import_statement(node, source) {
            ImportParseResult::Ok(decl) => NodeConversion::Ok(decl),
            ImportParseResult::Unsupported => NodeConversion::BailOut,
        },
        "export_statement" => {
            let name = get_export_identity(node, source);
            NodeConversion::Ok(TsDeclaration {
                kind: DeclarationKind::ExportStatement,
                identity: TsIdentity::Export(name),
                source_text,
                specifiers: BTreeSet::new(),
            })
        }
        "function_declaration" | "generator_function_declaration" => {
            let name = get_name_text(node, source).unwrap_or_default();
            NodeConversion::Ok(TsDeclaration {
                kind: DeclarationKind::Function,
                identity: TsIdentity::Function(name),
                source_text,
                specifiers: BTreeSet::new(),
            })
        }
        "class_declaration" => {
            let name = get_name_text(node, source).unwrap_or_default();
            NodeConversion::Ok(TsDeclaration {
                kind: DeclarationKind::Class,
                identity: TsIdentity::Class(name),
                source_text,
                specifiers: BTreeSet::new(),
            })
        }
        "interface_declaration" => {
            let name = get_name_text(node, source).unwrap_or_default();
            NodeConversion::Ok(TsDeclaration {
                kind: DeclarationKind::Interface,
                identity: TsIdentity::Interface(name),
                source_text,
                specifiers: BTreeSet::new(),
            })
        }
        "type_alias_declaration" => {
            let name = get_name_text(node, source).unwrap_or_default();
            NodeConversion::Ok(TsDeclaration {
                kind: DeclarationKind::TypeAlias,
                identity: TsIdentity::TypeAlias(name),
                source_text,
                specifiers: BTreeSet::new(),
            })
        }
        "enum_declaration" => {
            let name = get_name_text(node, source).unwrap_or_default();
            NodeConversion::Ok(TsDeclaration {
                kind: DeclarationKind::Enum,
                identity: TsIdentity::Enum(name),
                source_text,
                specifiers: BTreeSet::new(),
            })
        }
        "lexical_declaration" | "variable_declaration" => {
            let name = extract_variable_names(node, source);
            NodeConversion::Ok(TsDeclaration {
                kind: DeclarationKind::Variable,
                identity: TsIdentity::Variable(name),
                source_text,
                specifiers: BTreeSet::new(),
            })
        }
        "module" | "internal_module" => {
            let name = get_name_text(node, source).unwrap_or_default();
            NodeConversion::Ok(TsDeclaration {
                kind: DeclarationKind::Namespace,
                identity: TsIdentity::Namespace(name),
                source_text,
                specifiers: BTreeSet::new(),
            })
        }
        _ => NodeConversion::Skip,
    }
}

/// Result of attempting to parse an import statement.
///
/// Distinguished from `Option<TsDeclaration>` to differentiate "unsupported import
/// that should bail out the entire fragment" from "successfully parsed import".
pub(super) enum ImportParseResult {
    /// Successfully parsed into a declaration.
    Ok(TsDeclaration),
    /// Unsupported import form (default+named combo, dynamic import) — the caller
    /// should treat the entire fragment as unparsable to avoid silent data loss.
    Unsupported,
}

/// Parses an import statement into a `TsDeclaration` with identity and specifiers.
fn parse_import_statement(node: &Node<'_>, source: &str) -> ImportParseResult {
    let source_text = node_text(node, source).to_string();
    let trimmed = source_text.trim();

    // Bail out on dynamic import() expressions
    if trimmed.contains("import(") {
        return ImportParseResult::Unsupported;
    }

    // Extract the module specifier (the string after `from`)
    let Some(module) = extract_module_specifier(node, source) else {
        return ImportParseResult::Unsupported;
    };

    // Determine import kind and extract specifiers
    let classification = classify_import(node, source, trimmed);

    // Bail out on default+named combo: `import React, { useState } from 'react'`
    if is_default_named_combo(node, source) {
        return ImportParseResult::Unsupported;
    }

    let import_key = ImportKey {
        module,
        kind: classification.kind,
        namespace_alias: classification.namespace_alias,
    };

    ImportParseResult::Ok(TsDeclaration {
        kind: DeclarationKind::ImportStatement,
        identity: TsIdentity::Import(import_key),
        source_text,
        specifiers: classification.specifiers,
    })
}

/// Extracts the module specifier string from an import statement node.
fn extract_module_specifier(node: &Node<'_>, source: &str) -> Option<String> {
    // Look for a `source` field (tree-sitter-typescript uses "source" for the from clause)
    if let Some(source_node) = node.child_by_field_name("source") {
        let text = node_text(&source_node, source);
        // Strip quotes
        let stripped = text.trim_matches(|c| c == '\'' || c == '"' || c == '`');
        return Some(stripped.to_string());
    }

    // For side-effect imports like `import './polyfill'`, the string is a direct child
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "string" {
                let text = node_text(&child, source);
                let stripped = text.trim_matches(|c| c == '\'' || c == '"' || c == '`');
                return Some(stripped.to_string());
            }
        }
    }

    None
}

/// Result of classifying an import statement.
struct ImportClassification {
    kind: ImportKind,
    specifiers: BTreeSet<ImportSpecifier>,
    /// The namespace alias for `import * as X` imports.
    namespace_alias: Option<String>,
}

/// Classifies an import statement and extracts its specifiers.
fn classify_import(node: &Node<'_>, source: &str, text: &str) -> ImportClassification {
    let is_type_only = text.starts_with("import type ");

    // Check for side-effect import first: `import './polyfill'`
    // Side-effect imports have no import_clause child.
    if !has_import_clause(node) {
        return ImportClassification {
            kind: ImportKind::SideEffect,
            specifiers: BTreeSet::new(),
            namespace_alias: None,
        };
    }

    // Check for namespace import: `import * as X from 'x'`
    // The namespace_import node lives inside import_clause.
    if let Some(alias) = extract_namespace_alias(node, source) {
        return ImportClassification {
            kind: ImportKind::Namespace,
            specifiers: BTreeSet::new(),
            namespace_alias: Some(alias),
        };
    }

    // Check for named imports: `import { A, B } from 'x'`
    let mut specifiers = BTreeSet::new();
    let import_clause = find_import_clause(node);

    if let Some(clause) = import_clause {
        extract_specifiers_from_clause(&clause, source, &mut specifiers);
    }

    let kind = if is_type_only {
        ImportKind::TypeOnly
    } else {
        ImportKind::Value
    };

    ImportClassification {
        kind,
        specifiers,
        namespace_alias: None,
    }
}

/// Extracts the namespace alias from `import * as X`, returning the alias `X`.
/// Returns `None` if this is not a namespace import.
fn extract_namespace_alias(node: &Node<'_>, source: &str) -> Option<String> {
    for i in 0..node.child_count() {
        let Some(child) = node.child(i) else {
            continue;
        };
        if child.kind() == "namespace_import" {
            return get_name_text(&child, source);
        }
        // namespace_import may be nested inside import_clause
        if child.kind() == "import_clause" {
            for j in 0..child.child_count() {
                if let Some(clause_child) = child.child(j) {
                    if clause_child.kind() == "namespace_import" {
                        return get_name_text(&clause_child, source);
                    }
                }
            }
        }
    }
    None
}

/// Checks if this import has a default+named combo pattern.
///
/// Pattern: `import React, { useState } from 'react'`
/// The tree-sitter AST has both an `identifier` (default) and `named_imports` as siblings.
fn is_default_named_combo(node: &Node<'_>, _source: &str) -> bool {
    let mut has_default = false;
    let mut has_named = false;

    for i in 0..node.child_count() {
        let Some(child) = node.child(i) else {
            continue;
        };
        match child.kind() {
            "import_clause" => {
                // Check within the import clause for default + named_imports combo
                for j in 0..child.child_count() {
                    if let Some(clause_child) = child.child(j) {
                        match clause_child.kind() {
                            "identifier" => has_default = true,
                            "named_imports" => has_named = true,
                            _ => {}
                        }
                    }
                }
            }
            "identifier" => has_default = true,
            "named_imports" => has_named = true,
            _ => {}
        }
    }

    has_default && has_named
}

/// Finds the import clause (`named_imports`) node within an import statement.
fn find_import_clause<'a>(node: &'a Node<'a>) -> Option<Node<'a>> {
    for i in 0..node.child_count() {
        let Some(child) = node.child(i) else {
            continue;
        };
        match child.kind() {
            "import_clause" => {
                // Look inside the import clause for named_imports
                for j in 0..child.child_count() {
                    if let Some(clause_child) = child.child(j) {
                        if clause_child.kind() == "named_imports" {
                            return Some(clause_child);
                        }
                    }
                }
                return None;
            }
            "named_imports" => return Some(child),
            _ => {}
        }
    }
    None
}

/// Checks whether the import has an import clause (any imports beyond side-effect).
fn has_import_clause(node: &Node<'_>) -> bool {
    for i in 0..node.child_count() {
        let Some(child) = node.child(i) else {
            continue;
        };
        match child.kind() {
            "import_clause" | "named_imports" | "namespace_import" | "identifier" => {
                return true;
            }
            _ => {}
        }
    }
    false
}

/// Extracts import specifiers from a `named_imports` node.
fn extract_specifiers_from_clause(
    named_imports: &Node<'_>,
    source: &str,
    specifiers: &mut BTreeSet<ImportSpecifier>,
) {
    for i in 0..named_imports.child_count() {
        let Some(child) = named_imports.child(i) else {
            continue;
        };
        if child.kind() == "import_specifier" {
            if let Some(spec) = parse_import_specifier(&child, source) {
                specifiers.insert(spec);
            }
        }
    }
}

/// Parses a single import specifier node into an `ImportSpecifier`.
fn parse_import_specifier(node: &Node<'_>, source: &str) -> Option<ImportSpecifier> {
    let text = node_text(node, source).trim().to_string();

    // Check if this is a type-only specifier: `type A` or `type A as B`
    let is_type = text.starts_with("type ");

    let name_node = node.child_by_field_name("name");
    let alias_node = node.child_by_field_name("alias");

    let name = if let Some(n) = name_node {
        node_text(&n, source).to_string()
    } else {
        // Fallback: first identifier child
        let mut found = String::new();
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if child.kind() == "identifier" || child.kind() == "type_identifier" {
                    found = node_text(&child, source).to_string();
                    break;
                }
            }
        }
        found
    };

    if name.is_empty() {
        return None;
    }

    let alias = alias_node.map(|a| node_text(&a, source).to_string());

    Some(ImportSpecifier {
        name,
        alias,
        is_type,
    })
}

/// Gets the name text of a declaration node.
fn get_name_text(node: &Node<'_>, source: &str) -> Option<String> {
    if let Some(name_node) = node.child_by_field_name("name") {
        return Some(node_text(&name_node, source).to_string());
    }
    // Fallback: first identifier child
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "identifier" || child.kind() == "type_identifier" {
                return Some(node_text(&child, source).to_string());
            }
        }
    }
    None
}

/// Gets a simple export identity string from an export statement.
fn get_export_identity(node: &Node<'_>, source: &str) -> String {
    node_text(node, source).to_string()
}

/// Extracts variable names from a lexical/variable declaration.
fn extract_variable_names(node: &Node<'_>, source: &str) -> String {
    let mut names = Vec::new();
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "variable_declarator" {
                if let Some(name_node) = child.child_by_field_name("name") {
                    names.push(node_text(&name_node, source).to_string());
                }
            }
        }
    }
    names.join(",")
}

/// Checks if a node is a module or namespace declaration.
fn is_module_or_namespace(node: &Node<'_>) -> bool {
    matches!(
        node.kind(),
        "module" | "internal_module" | "ambient_declaration"
    )
}

/// Extracts the text of a tree-sitter node from the source.
fn node_text<'a>(node: &Node<'_>, source: &'a str) -> &'a str {
    &source[node.start_byte()..node.end_byte()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_import_statements() {
        let text = "import { useState } from 'react';\nimport { useEffect } from 'react';";
        let result = parse_fragment(text);
        match result {
            ParsedFragment::Declarations(decls) => {
                assert_eq!(decls.len(), 2);
                assert_eq!(decls[0].kind, DeclarationKind::ImportStatement);
                assert_eq!(decls[1].kind, DeclarationKind::ImportStatement);
            }
            ParsedFragment::Unparsable => panic!("should parse import statements"),
        }
    }

    #[test]
    fn parse_type_import() {
        let text = "import type { Foo } from './types';";
        let result = parse_fragment(text);
        match result {
            ParsedFragment::Declarations(decls) => {
                assert_eq!(decls.len(), 1);
                assert_eq!(decls[0].kind, DeclarationKind::ImportStatement);
                if let TsIdentity::Import(ref key) = decls[0].identity {
                    assert_eq!(key.kind, ImportKind::TypeOnly);
                    assert_eq!(key.module, "./types");
                } else {
                    panic!("expected Import identity");
                }
            }
            ParsedFragment::Unparsable => panic!("should parse type import"),
        }
    }

    #[test]
    fn parse_side_effect_import() {
        let text = "import './polyfill';";
        let result = parse_fragment(text);
        match result {
            ParsedFragment::Declarations(decls) => {
                assert_eq!(decls.len(), 1);
                if let TsIdentity::Import(ref key) = decls[0].identity {
                    assert_eq!(key.kind, ImportKind::SideEffect);
                    assert_eq!(key.module, "./polyfill");
                } else {
                    panic!("expected Import identity");
                }
            }
            ParsedFragment::Unparsable => panic!("should parse side-effect import"),
        }
    }

    #[test]
    fn parse_namespace_import() {
        let text = "import * as React from 'react';";
        let result = parse_fragment(text);
        match result {
            ParsedFragment::Declarations(decls) => {
                assert_eq!(decls.len(), 1);
                if let TsIdentity::Import(ref key) = decls[0].identity {
                    assert_eq!(key.kind, ImportKind::Namespace);
                    assert_eq!(key.module, "react");
                } else {
                    panic!("expected Import identity");
                }
            }
            ParsedFragment::Unparsable => panic!("should parse namespace import"),
        }
    }

    #[test]
    fn parse_class() {
        let text = "class Foo { bar() {} }";
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
    fn parse_function() {
        let text = "function hello() { return 'world'; }";
        let result = parse_fragment(text);
        match result {
            ParsedFragment::Declarations(decls) => {
                assert_eq!(decls.len(), 1);
                assert_eq!(decls[0].kind, DeclarationKind::Function);
            }
            ParsedFragment::Unparsable => panic!("should parse function"),
        }
    }

    #[test]
    fn parse_interface() {
        let text = "interface Props { name: string; }";
        let result = parse_fragment(text);
        match result {
            ParsedFragment::Declarations(decls) => {
                assert_eq!(decls.len(), 1);
                assert_eq!(decls[0].kind, DeclarationKind::Interface);
            }
            ParsedFragment::Unparsable => panic!("should parse interface"),
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
    fn triple_slash_returns_unparsable() {
        let text = "/// <reference path=\"types.d.ts\" />\nimport { Foo } from './foo';";
        let result = parse_fragment(text);
        assert!(matches!(result, ParsedFragment::Unparsable));
    }

    #[test]
    fn default_named_combo_returns_unparsable() {
        let text = "import React, { useState } from 'react';";
        let result = parse_fragment(text);
        assert!(
            matches!(result, ParsedFragment::Unparsable),
            "default+named combo should bail out to Unparsable"
        );
    }

    #[test]
    fn import_specifier_extraction() {
        let text = "import { useState, useEffect } from 'react';";
        let result = parse_fragment(text);
        match result {
            ParsedFragment::Declarations(decls) => {
                assert_eq!(decls.len(), 1);
                assert_eq!(decls[0].specifiers.len(), 2);
                let names: Vec<_> = decls[0].specifiers.iter().map(|s| &s.name).collect();
                assert!(names.contains(&&"useState".to_string()));
                assert!(names.contains(&&"useEffect".to_string()));
            }
            ParsedFragment::Unparsable => panic!("should parse"),
        }
    }

    #[test]
    fn type_alias_declaration() {
        let text = "type ID = string | number;";
        let result = parse_fragment(text);
        match result {
            ParsedFragment::Declarations(decls) => {
                assert_eq!(decls.len(), 1);
                assert_eq!(decls[0].kind, DeclarationKind::TypeAlias);
            }
            ParsedFragment::Unparsable => panic!("should parse type alias"),
        }
    }

    #[test]
    fn enum_declaration() {
        let text = "enum Color { Red, Blue, Green }";
        let result = parse_fragment(text);
        match result {
            ParsedFragment::Declarations(decls) => {
                assert_eq!(decls.len(), 1);
                assert_eq!(decls[0].kind, DeclarationKind::Enum);
            }
            ParsedFragment::Unparsable => panic!("should parse enum"),
        }
    }

    #[test]
    fn variable_declaration() {
        let text = "const FOO = 42;";
        let result = parse_fragment(text);
        match result {
            ParsedFragment::Declarations(decls) => {
                assert_eq!(decls.len(), 1);
                assert_eq!(decls[0].kind, DeclarationKind::Variable);
            }
            ParsedFragment::Unparsable => panic!("should parse variable declaration"),
        }
    }
}
