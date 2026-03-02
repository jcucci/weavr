//! Declaration-level merge algorithms for 2-way and 3-way merges.
//!
//! Mirrors the C# merger's `member_merge` but operates on
//! `GoDeclaration` instead of `CSharpDeclaration`.

use std::collections::{BTreeMap, BTreeSet};

use super::identity::GoIdentity;
use super::import_merge;
use super::parse::{is_dot_import, DeclarationKind, GoDeclaration};
use crate::mergers::confidence::{compute_import_confidence, compute_mixed_confidence};

/// The result of merging declarations from two or three sides.
pub(super) struct MergedDeclarations {
    pub declarations: Vec<GoDeclaration>,
    pub confidence: f32,
    pub description: String,
}

/// Returns whether all declarations are import declarations.
fn all_imports(decls: &[GoDeclaration]) -> bool {
    !decls.is_empty()
        && decls
            .iter()
            .all(|d| d.kind == DeclarationKind::ImportDeclaration)
}

/// Builds a map from identity to declaration for fast lookup.
fn build_identity_map(decls: &[GoDeclaration]) -> BTreeMap<GoIdentity, &GoDeclaration> {
    let mut map = BTreeMap::new();
    for decl in decls {
        map.insert(decl.identity.clone(), decl);
    }
    map
}

/// Returns whether two declarations have the same source text (modulo whitespace normalization).
fn source_equal(a: &GoDeclaration, b: &GoDeclaration) -> bool {
    normalize_whitespace(&a.source_text) == normalize_whitespace(&b.source_text)
}

/// Normalizes whitespace for comparison: collapses runs of whitespace to single space, trims.
fn normalize_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Two-way merge: identity-based merge of left and right declarations.
///
/// - If all declarations are import declarations, delegates to `import_merge`.
/// - For declarations with the same identity but different content, tries type member merge.
/// - Disjoint declarations are combined.
/// - Returns `None` if a conflict is detected that cannot be resolved.
pub(super) fn merge_two_way(
    left: &[GoDeclaration],
    right: &[GoDeclaration],
) -> Option<MergedDeclarations> {
    // Special-case: pure import merge
    if all_imports(left) && all_imports(right) {
        return import_merge::merge_import_declarations(left, right, None).map(|(decls, desc)| {
            MergedDeclarations {
                confidence: compute_import_confidence(all_imports(&decls), false),
                declarations: decls,
                description: desc,
            }
        });
    }

    let right_map = build_identity_map(right);

    let mut merged: Vec<GoDeclaration> = Vec::new();
    let mut seen = BTreeSet::new();
    let mut descriptions = Vec::new();
    let mut has_member_disjoint = false;
    let mut has_import_merge = false;

    // Collect import declarations for separate merging
    let left_imports: Vec<GoDeclaration> = left
        .iter()
        .filter(|d| d.kind == DeclarationKind::ImportDeclaration)
        .cloned()
        .collect();
    let right_imports: Vec<GoDeclaration> = right
        .iter()
        .filter(|d| d.kind == DeclarationKind::ImportDeclaration)
        .cloned()
        .collect();

    if !left_imports.is_empty() || !right_imports.is_empty() {
        if let Some((import_decls, desc)) =
            import_merge::merge_import_declarations(&left_imports, &right_imports, None)
        {
            merged.extend(import_decls);
            descriptions.push(desc);
            has_import_merge = true;
        } else if left_imports.iter().any(|d| is_dot_import(d))
            || right_imports.iter().any(|d| is_dot_import(d))
        {
            // Dot imports cannot be safely merged -- bail out to text merge
            return None;
        } else {
            // Identical imports -- just keep left's
            merged.extend(left_imports);
        }
        // Mark all import identities as seen
        for decl in left.iter().chain(right.iter()) {
            if decl.kind == DeclarationKind::ImportDeclaration {
                seen.insert(decl.identity.clone());
            }
        }
    }

    // Process non-import left declarations
    for decl in left {
        if decl.kind == DeclarationKind::ImportDeclaration {
            continue;
        }
        let id = decl.identity.clone();
        seen.insert(id.clone());

        if let Some(right_decl) = right_map.get(&id) {
            if source_equal(decl, right_decl) {
                // Identical -- keep once
                merged.push(decl.clone());
            } else {
                // Try type member merge
                match try_merge_matching_declarations(decl, right_decl) {
                    Some(merged_decl) => {
                        descriptions.push(format!("Merged type {}", declaration_name(decl)));
                        has_member_disjoint = true;
                        merged.push(merged_decl);
                    }
                    None => return None, // Unresolvable conflict
                }
            }
        } else {
            // Only on left side
            merged.push(decl.clone());
        }
    }

    // Add right-only non-import declarations
    for decl in right {
        if decl.kind == DeclarationKind::ImportDeclaration {
            continue;
        }
        let id = &decl.identity;
        if !seen.contains(id) {
            merged.push(decl.clone());
        }
    }

    let description = if descriptions.is_empty() {
        "Merged declarations".to_string()
    } else {
        descriptions.join("; ")
    };

    let confidence = compute_mixed_confidence(has_import_merge, has_member_disjoint, false);
    Some(MergedDeclarations {
        declarations: merged,
        confidence,
        description,
    })
}

/// Three-way merge: classifies each identity as unchanged/added/modified/deleted per side.
#[allow(clippy::too_many_lines)]
pub(super) fn merge_three_way(
    base: &[GoDeclaration],
    left: &[GoDeclaration],
    right: &[GoDeclaration],
) -> Option<MergedDeclarations> {
    // Special-case: pure import merge
    if all_imports(left) && all_imports(right) && all_imports(base) {
        return import_merge::merge_import_declarations(left, right, Some(base)).map(
            |(decls, desc)| MergedDeclarations {
                confidence: compute_import_confidence(all_imports(&decls), true),
                declarations: decls,
                description: desc,
            },
        );
    }

    let base_map = build_identity_map(base);
    let left_map = build_identity_map(left);
    let right_map = build_identity_map(right);

    let mut merged: Vec<GoDeclaration> = Vec::new();
    let mut seen = BTreeSet::new();
    let mut descriptions = Vec::new();
    let mut has_member_disjoint = false;
    let mut has_import_merge = false;

    // Handle import declarations via 3-way import merge
    let base_imports: Vec<GoDeclaration> = base
        .iter()
        .filter(|d| d.kind == DeclarationKind::ImportDeclaration)
        .cloned()
        .collect();
    let left_imports: Vec<GoDeclaration> = left
        .iter()
        .filter(|d| d.kind == DeclarationKind::ImportDeclaration)
        .cloned()
        .collect();
    let right_imports: Vec<GoDeclaration> = right
        .iter()
        .filter(|d| d.kind == DeclarationKind::ImportDeclaration)
        .cloned()
        .collect();

    if !base_imports.is_empty() || !left_imports.is_empty() || !right_imports.is_empty() {
        if let Some((import_decls, desc)) = import_merge::merge_import_declarations(
            &left_imports,
            &right_imports,
            Some(&base_imports),
        ) {
            merged.extend(import_decls);
            descriptions.push(desc);
            has_import_merge = true;
        } else if base_imports.iter().any(|d| is_dot_import(d))
            || left_imports.iter().any(|d| is_dot_import(d))
            || right_imports.iter().any(|d| is_dot_import(d))
        {
            // Dot imports cannot be safely merged -- bail out to text merge
            return None;
        } else {
            merged.extend(left_imports);
        }
        for decl in base.iter().chain(left.iter()).chain(right.iter()) {
            if decl.kind == DeclarationKind::ImportDeclaration {
                seen.insert(decl.identity.clone());
            }
        }
    }

    // Collect all non-import identities across all three sides
    let mut all_ids = BTreeSet::new();
    for decl in base.iter().chain(left.iter()).chain(right.iter()) {
        if decl.kind != DeclarationKind::ImportDeclaration {
            all_ids.insert(decl.identity.clone());
        }
    }

    for id in &all_ids {
        if seen.contains(id) {
            continue;
        }
        seen.insert(id.clone());

        let in_base = base_map.get(id);
        let in_left = left_map.get(id);
        let in_right = right_map.get(id);

        match (in_base, in_left, in_right) {
            // In all three -- check for modifications
            (Some(b), Some(l), Some(r)) => {
                if !merge_three_present(b, l, r, &mut merged, &mut has_member_disjoint) {
                    return None;
                }
            }
            // Deleted by one side -- respect deletion (unless modified by the other)
            (Some(b), Some(l), None) => {
                if !source_equal(b, l) {
                    return None;
                }
            }
            (Some(b), None, Some(r)) => {
                if !source_equal(b, r) {
                    return None;
                }
            }
            // Both sides deleted, or identity not actually present
            (Some(_) | None, None, None) => {}
            // Added by both sides
            (None, Some(l), Some(r)) => {
                if source_equal(l, r) {
                    merged.push((*l).clone());
                } else {
                    match try_merge_matching_declarations(l, r) {
                        Some(merged_decl) => {
                            has_member_disjoint = true;
                            merged.push(merged_decl);
                        }
                        None => return None,
                    }
                }
            }
            (None, Some(l), None) => {
                merged.push((*l).clone());
            }
            (None, None, Some(r)) => {
                merged.push((*r).clone());
            }
        }
    }

    let description = if descriptions.is_empty() {
        "Three-way merged declarations".to_string()
    } else {
        descriptions.join("; ")
    };

    let confidence = compute_mixed_confidence(has_import_merge, has_member_disjoint, true);
    Some(MergedDeclarations {
        declarations: merged,
        confidence,
        description,
    })
}

/// Handles the case where a declaration is present in all three sides (base, left, right).
///
/// Returns `false` if the declarations conflict and cannot be merged.
fn merge_three_present(
    b: &GoDeclaration,
    l: &GoDeclaration,
    r: &GoDeclaration,
    merged: &mut Vec<GoDeclaration>,
    has_member_disjoint: &mut bool,
) -> bool {
    let left_changed = !source_equal(b, l);
    let right_changed = !source_equal(b, r);

    match (left_changed, right_changed) {
        (false, false) => {
            // Unchanged -- keep base version
            merged.push(b.clone());
        }
        (true, false) => {
            // Only left modified -- take left
            merged.push(l.clone());
        }
        (false, true) => {
            // Only right modified -- take right
            merged.push(r.clone());
        }
        (true, true) => {
            // Both modified -- try type member merge
            if source_equal(l, r) {
                merged.push(l.clone());
            } else if let Some(merged_decl) = try_merge_matching_declarations(l, r) {
                *has_member_disjoint = true;
                merged.push(merged_decl);
            } else {
                return false;
            }
        }
    }

    true
}

/// Attempts to merge two declarations that share the same identity but differ in content.
///
/// Only handles type declarations (struct/interface) with disjoint member additions.
/// Returns `None` for other declaration types, signaling the conflict cannot be resolved.
fn try_merge_matching_declarations(
    left: &GoDeclaration,
    right: &GoDeclaration,
) -> Option<GoDeclaration> {
    if left.kind != DeclarationKind::TypeDeclaration
        || right.kind != DeclarationKind::TypeDeclaration
    {
        return None;
    }

    // Only merge if both have children (struct fields or interface methods)
    if left.children.is_empty() && right.children.is_empty() {
        return None;
    }

    merge_type_members(left, right)
}

/// Merges members of two type declarations (struct fields or interface methods).
fn merge_type_members(left: &GoDeclaration, right: &GoDeclaration) -> Option<GoDeclaration> {
    let right_map = build_member_identity_map(&right.children);
    let mut merged_members: Vec<GoDeclaration> = Vec::new();
    let mut seen = BTreeSet::new();

    for member in &left.children {
        let id = &member.identity;
        seen.insert(id.clone());

        if let Some(right_member) = right_map.get(id) {
            if source_equal(member, right_member) {
                merged_members.push(member.clone());
            } else {
                return None;
            }
        } else {
            merged_members.push(member.clone());
        }
    }

    for member in &right.children {
        if !seen.contains(&member.identity) {
            merged_members.push(member.clone());
        }
    }

    Some(reconstruct_type_declaration(left, merged_members))
}

/// Builds a member identity map for fast lookup.
fn build_member_identity_map(members: &[GoDeclaration]) -> BTreeMap<GoIdentity, &GoDeclaration> {
    let mut map = BTreeMap::new();
    for member in members {
        map.insert(member.identity.clone(), member);
    }
    map
}

/// Reconstructs a type declaration with new children members.
///
/// Preserves the original header from the left side and reassembles the body
/// from the merged members.
fn reconstruct_type_declaration(
    original: &GoDeclaration,
    children: Vec<GoDeclaration>,
) -> GoDeclaration {
    let indent = detect_indentation(&original.source_text);

    let header = if let Some(brace_pos) = original.source_text.find('{') {
        &original.source_text[..=brace_pos]
    } else {
        return GoDeclaration {
            children,
            ..original.clone()
        };
    };

    let mut body = String::new();
    body.push_str(header);
    body.push('\n');

    for child in &children {
        for line in child.source_text.lines() {
            body.push_str(&indent);
            body.push('\t');
            body.push_str(line);
            body.push('\n');
        }
    }

    body.push_str(&indent);
    body.push('}');

    GoDeclaration {
        source_text: body,
        children,
        ..original.clone()
    }
}

/// Detects the indentation level of the first line.
fn detect_indentation(text: &str) -> String {
    let first_line = text.lines().next().unwrap_or("");
    let trimmed = first_line.trim_start();
    let indent_len = first_line.len() - trimmed.len();
    first_line[..indent_len].to_string()
}

/// Returns a human-readable name for a declaration.
fn declaration_name(decl: &GoDeclaration) -> String {
    match &decl.identity {
        GoIdentity::Type(n) | GoIdentity::Function(n) => n.clone(),
        GoIdentity::Method(recv, name) => format!("{recv}.{name}"),
        _ => String::from("(unknown)"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mergers::go_merger::parse::parse_fragment;

    fn parse(text: &str) -> Vec<GoDeclaration> {
        match parse_fragment(text) {
            super::super::parse::ParsedFragment::Declarations(decls) => decls,
            super::super::parse::ParsedFragment::Unparsable => {
                panic!("failed to parse: {text}")
            }
        }
    }

    #[test]
    fn two_way_disjoint_functions() {
        let left = parse("func Foo() {}");
        let right = parse("func Bar() {}");
        let result = merge_two_way(&left, &right).unwrap();
        assert_eq!(result.declarations.len(), 2);
    }

    #[test]
    fn two_way_identical_functions() {
        let left = parse("func Foo() {}");
        let right = parse("func Foo() {}");
        let result = merge_two_way(&left, &right).unwrap();
        assert_eq!(result.declarations.len(), 1);
    }

    #[test]
    fn two_way_conflicting_functions() {
        let left = parse("func Foo() { return 1 }");
        let right = parse("func Foo() { return 2 }");
        let result = merge_two_way(&left, &right);
        assert!(result.is_none(), "conflicting functions should return None");
    }

    #[test]
    fn source_equal_ignores_whitespace() {
        let a = GoDeclaration {
            kind: DeclarationKind::Function,
            identity: GoIdentity::Function("Foo".to_string()),
            source_text: "func  Foo()  { }".to_string(),
            children: Vec::new(),
        };
        let b = GoDeclaration {
            kind: DeclarationKind::Function,
            identity: GoIdentity::Function("Foo".to_string()),
            source_text: "func Foo() { }".to_string(),
            children: Vec::new(),
        };
        assert!(super::source_equal(&a, &b));
    }

    #[test]
    fn struct_disjoint_field_additions() {
        let left = parse("type Config struct {\n\tName string\n\tHost string\n}");
        let right = parse("type Config struct {\n\tName string\n\tPort int\n}");
        let result = merge_two_way(&left, &right).unwrap();
        assert_eq!(result.declarations.len(), 1);
        assert!(result.declarations[0].source_text.contains("Host"));
        assert!(result.declarations[0].source_text.contains("Port"));
    }

    #[test]
    fn interface_disjoint_method_additions() {
        let left = parse("type Handler interface {\n\tServeHTTP()\n\tInit()\n}");
        let right = parse("type Handler interface {\n\tServeHTTP()\n\tClose()\n}");
        let result = merge_two_way(&left, &right).unwrap();
        assert_eq!(result.declarations.len(), 1);
        assert!(result.declarations[0].source_text.contains("Init"));
        assert!(result.declarations[0].source_text.contains("Close"));
    }
}
