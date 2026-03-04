//! Declaration-level merge algorithms for 2-way and 3-way merges.
//!
//! Mirrors the Rust merger's `item_merge` + `impl_merge` but operates on
//! `CSharpDeclaration` instead of `syn::Item`.

use std::collections::{BTreeMap, BTreeSet};

use super::identity::{CSharpIdentity, MemberIdentity};
use super::parse::{is_complex_using, to_member_identity, CSharpDeclaration, DeclarationKind};
use super::using_merge;
use crate::mergers::confidence::{compute_import_confidence, compute_mixed_confidence};

/// The result of merging declarations from two or three sides.
pub(super) struct MergedDeclarations {
    pub declarations: Vec<CSharpDeclaration>,
    pub confidence: f32,
    pub description: String,
}

/// Returns whether all declarations are using directives.
fn all_usings(decls: &[CSharpDeclaration]) -> bool {
    !decls.is_empty()
        && decls
            .iter()
            .all(|d| d.kind == DeclarationKind::UsingDirective)
}

/// Builds a map from identity to declaration for fast lookup.
fn build_identity_map(decls: &[CSharpDeclaration]) -> BTreeMap<CSharpIdentity, &CSharpDeclaration> {
    let mut map = BTreeMap::new();
    for decl in decls {
        map.insert(decl.identity.clone(), decl);
    }
    map
}

/// Returns whether two declarations have the same source text (modulo whitespace normalization).
fn source_equal(a: &CSharpDeclaration, b: &CSharpDeclaration) -> bool {
    normalize_whitespace(&a.source_text) == normalize_whitespace(&b.source_text)
}

/// Normalizes whitespace for comparison: collapses runs of whitespace to single space, trims.
fn normalize_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Two-way merge: identity-based merge of left and right declarations.
///
/// - If all declarations are using directives, delegates to `using_merge`.
/// - For declarations with the same identity but different content, tries class member merge.
/// - Disjoint declarations are combined.
/// - Returns `None` if a conflict is detected that cannot be resolved.
pub(super) fn merge_two_way(
    left: &[CSharpDeclaration],
    right: &[CSharpDeclaration],
) -> Option<MergedDeclarations> {
    // Special-case: pure using-directive merge
    if all_usings(left) && all_usings(right) {
        return using_merge::merge_using_directives(left, right, None).map(|(decls, desc)| {
            MergedDeclarations {
                confidence: compute_import_confidence(all_usings(&decls), false),
                declarations: decls,
                description: desc,
            }
        });
    }

    let right_map = build_identity_map(right);

    let mut merged: Vec<CSharpDeclaration> = Vec::new();
    let mut seen = BTreeSet::new();
    let mut descriptions = Vec::new();
    let mut has_member_disjoint = false;
    let mut has_using_merge = false;

    // Collect using declarations for separate merging
    let left_usings: Vec<CSharpDeclaration> = left
        .iter()
        .filter(|d| d.kind == DeclarationKind::UsingDirective)
        .cloned()
        .collect();
    let right_usings: Vec<CSharpDeclaration> = right
        .iter()
        .filter(|d| d.kind == DeclarationKind::UsingDirective)
        .cloned()
        .collect();

    if !left_usings.is_empty() || !right_usings.is_empty() {
        if let Some((using_decls, desc)) =
            using_merge::merge_using_directives(&left_usings, &right_usings, None)
        {
            merged.extend(using_decls);
            descriptions.push(desc);
            has_using_merge = true;
        } else if left_usings.iter().any(is_complex_using)
            || right_usings.iter().any(is_complex_using)
        {
            // Complex usings cannot be safely merged -- bail out to text merge
            return None;
        } else {
            // Identical usings -- just keep left's
            merged.extend(left_usings);
        }
        // Mark all using identities as seen
        for decl in left.iter().chain(right.iter()) {
            if decl.kind == DeclarationKind::UsingDirective {
                seen.insert(decl.identity.clone());
            }
        }
    }

    // Process non-using left declarations
    for decl in left {
        if decl.kind == DeclarationKind::UsingDirective {
            continue;
        }
        let id = decl.identity.clone();
        seen.insert(id.clone());

        if let Some(right_decl) = right_map.get(&id) {
            if source_equal(decl, right_decl) {
                // Identical -- keep once
                merged.push(decl.clone());
            } else {
                // Try class member merge
                match try_merge_matching_declarations(decl, right_decl) {
                    Some(merged_decl) => {
                        descriptions.push(format!("Merged class {}", declaration_name(decl)));
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

    // Add right-only non-using declarations
    for decl in right {
        if decl.kind == DeclarationKind::UsingDirective {
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

    let confidence = compute_mixed_confidence(has_using_merge, has_member_disjoint, false);
    Some(MergedDeclarations {
        declarations: merged,
        confidence,
        description,
    })
}

/// Three-way merge: classifies each identity as unchanged/added/modified/deleted per side.
#[allow(clippy::too_many_lines)]
pub(super) fn merge_three_way(
    base: &[CSharpDeclaration],
    left: &[CSharpDeclaration],
    right: &[CSharpDeclaration],
) -> Option<MergedDeclarations> {
    // Special-case: pure using-directive merge
    if all_usings(left) && all_usings(right) && all_usings(base) {
        return using_merge::merge_using_directives(left, right, Some(base)).map(
            |(decls, desc)| MergedDeclarations {
                confidence: compute_import_confidence(all_usings(&decls), true),
                declarations: decls,
                description: desc,
            },
        );
    }

    let base_map = build_identity_map(base);
    let left_map = build_identity_map(left);
    let right_map = build_identity_map(right);

    let mut merged: Vec<CSharpDeclaration> = Vec::new();
    let mut seen = BTreeSet::new();
    let mut descriptions = Vec::new();
    let mut has_member_disjoint = false;
    let mut has_using_merge = false;

    // Handle using directives via 3-way using merge
    let base_usings: Vec<CSharpDeclaration> = base
        .iter()
        .filter(|d| d.kind == DeclarationKind::UsingDirective)
        .cloned()
        .collect();
    let left_usings: Vec<CSharpDeclaration> = left
        .iter()
        .filter(|d| d.kind == DeclarationKind::UsingDirective)
        .cloned()
        .collect();
    let right_usings: Vec<CSharpDeclaration> = right
        .iter()
        .filter(|d| d.kind == DeclarationKind::UsingDirective)
        .cloned()
        .collect();

    if !base_usings.is_empty() || !left_usings.is_empty() || !right_usings.is_empty() {
        if let Some((using_decls, desc)) =
            using_merge::merge_using_directives(&left_usings, &right_usings, Some(&base_usings))
        {
            merged.extend(using_decls);
            descriptions.push(desc);
            has_using_merge = true;
        } else if base_usings.iter().any(is_complex_using)
            || left_usings.iter().any(is_complex_using)
            || right_usings.iter().any(is_complex_using)
        {
            // Complex usings cannot be safely merged -- bail out to text merge
            return None;
        } else {
            merged.extend(left_usings);
        }
        for decl in base.iter().chain(left.iter()).chain(right.iter()) {
            if decl.kind == DeclarationKind::UsingDirective {
                seen.insert(decl.identity.clone());
            }
        }
    }

    // Collect all non-using identities across all three sides
    let mut all_ids = BTreeSet::new();
    for decl in base.iter().chain(left.iter()).chain(right.iter()) {
        if decl.kind != DeclarationKind::UsingDirective {
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

    let confidence = compute_mixed_confidence(has_using_merge, has_member_disjoint, true);
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
    b: &CSharpDeclaration,
    l: &CSharpDeclaration,
    r: &CSharpDeclaration,
    merged: &mut Vec<CSharpDeclaration>,
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
            // Both modified -- try class member merge
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
/// Only handles class/struct/interface declarations with disjoint member additions.
/// Returns `None` for other declaration types, signaling the conflict cannot be resolved.
fn try_merge_matching_declarations(
    left: &CSharpDeclaration,
    right: &CSharpDeclaration,
) -> Option<CSharpDeclaration> {
    match (left.kind, right.kind) {
        (DeclarationKind::Class, DeclarationKind::Class)
        | (DeclarationKind::Struct, DeclarationKind::Struct)
        | (DeclarationKind::Interface, DeclarationKind::Interface) => {
            merge_type_members(left, right)
        }
        (DeclarationKind::Namespace, DeclarationKind::Namespace) => {
            merge_namespace_members(left, right)
        }
        _ => None,
    }
}

/// Merges members of two type declarations (class/struct/interface).
fn merge_type_members(
    left: &CSharpDeclaration,
    right: &CSharpDeclaration,
) -> Option<CSharpDeclaration> {
    if left.children.is_empty() && right.children.is_empty() {
        return None;
    }

    let right_map = build_member_identity_map(&right.children);
    let mut merged_members: Vec<CSharpDeclaration> = Vec::new();
    let mut seen = BTreeSet::new();

    for member in &left.children {
        let id = to_member_identity(member);
        seen.insert(id.clone());

        if let Some(right_member) = right_map.get(&id) {
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
        let id = to_member_identity(member);
        if !seen.contains(&id) {
            merged_members.push(member.clone());
        }
    }

    Some(reconstruct_type_declaration(left, merged_members))
}

/// Merges members of two namespace declarations.
fn merge_namespace_members(
    left: &CSharpDeclaration,
    right: &CSharpDeclaration,
) -> Option<CSharpDeclaration> {
    if left.children.is_empty() && right.children.is_empty() {
        return None;
    }

    let right_map = build_identity_map_from_children(&right.children);
    let mut merged_children: Vec<CSharpDeclaration> = Vec::new();
    let mut seen = BTreeSet::new();

    for child in &left.children {
        let id = child.identity.clone();
        seen.insert(id.clone());

        if let Some(right_child) = right_map.get(&id) {
            if source_equal(child, right_child) {
                merged_children.push(child.clone());
            } else {
                match try_merge_matching_declarations(child, right_child) {
                    Some(merged_child) => merged_children.push(merged_child),
                    None => return None,
                }
            }
        } else {
            merged_children.push(child.clone());
        }
    }

    for child in &right.children {
        if !seen.contains(&child.identity) {
            merged_children.push(child.clone());
        }
    }

    Some(reconstruct_type_declaration(left, merged_children))
}

/// Builds a member identity map for fast lookup.
fn build_member_identity_map(
    members: &[CSharpDeclaration],
) -> BTreeMap<MemberIdentity, &CSharpDeclaration> {
    let mut map = BTreeMap::new();
    for member in members {
        let id = to_member_identity(member);
        map.insert(id, member);
    }
    map
}

/// Builds an identity map from children declarations.
fn build_identity_map_from_children(
    children: &[CSharpDeclaration],
) -> BTreeMap<CSharpIdentity, &CSharpDeclaration> {
    let mut map = BTreeMap::new();
    for child in children {
        map.insert(child.identity.clone(), child);
    }
    map
}

/// Reconstructs a type declaration with new children members.
///
/// Preserves the original header (modifiers, name, base types) from the left side
/// and reassembles the body from the merged members.
fn reconstruct_type_declaration(
    original: &CSharpDeclaration,
    children: Vec<CSharpDeclaration>,
) -> CSharpDeclaration {
    let indent = detect_indentation(&original.source_text);

    let header = if let Some(brace_pos) = original.source_text.find('{') {
        &original.source_text[..=brace_pos]
    } else {
        return CSharpDeclaration {
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
            body.push_str("    ");
            body.push_str(line);
            body.push('\n');
        }
    }

    body.push_str(&indent);
    body.push('}');

    CSharpDeclaration {
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
fn declaration_name(decl: &CSharpDeclaration) -> String {
    match &decl.identity {
        CSharpIdentity::Class(n)
        | CSharpIdentity::Struct(n)
        | CSharpIdentity::Interface(n)
        | CSharpIdentity::Namespace(n) => n.clone(),
        _ => String::from("(unknown)"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mergers::csharp_merger::parse::parse_fragment;

    fn parse(text: &str) -> Vec<CSharpDeclaration> {
        match parse_fragment(text) {
            super::super::parse::ParsedFragment::Declarations(decls) => decls,
            super::super::parse::ParsedFragment::Unparsable => {
                panic!("failed to parse: {text}")
            }
        }
    }

    #[test]
    fn two_way_disjoint_classes() {
        let left = parse("class Foo { }");
        let right = parse("class Bar { }");
        let result = merge_two_way(&left, &right).unwrap();
        assert_eq!(result.declarations.len(), 2);
    }

    #[test]
    fn two_way_identical_classes() {
        let left = parse("class Foo { }");
        let right = parse("class Foo { }");
        let result = merge_two_way(&left, &right).unwrap();
        assert_eq!(result.declarations.len(), 1);
    }

    #[test]
    fn two_way_conflicting_classes() {
        let left = parse("class Foo { public void A() { } }");
        let right = parse("class Foo { public void B() { } }");
        let result = merge_two_way(&left, &right);
        assert!(result.is_some());
    }

    #[test]
    fn source_equal_ignores_whitespace() {
        let a = CSharpDeclaration {
            kind: DeclarationKind::Class,
            identity: CSharpIdentity::Class("Foo".to_string()),
            source_text: "class  Foo  { }".to_string(),
            children: Vec::new(),
        };
        let b = CSharpDeclaration {
            kind: DeclarationKind::Class,
            identity: CSharpIdentity::Class("Foo".to_string()),
            source_text: "class Foo { }".to_string(),
            children: Vec::new(),
        };
        assert!(super::source_equal(&a, &b));
    }
}
