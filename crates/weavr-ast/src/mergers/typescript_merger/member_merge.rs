//! Declaration-level merge algorithms for 2-way and 3-way merges.
//!
//! Mirrors the C# merger's `member_merge` but operates on
//! `TsDeclaration` instead of `CSharpDeclaration`.

use std::collections::{BTreeMap, BTreeSet};

use super::identity::TsIdentity;
use super::import_merge;
use super::parse::{DeclarationKind, TsDeclaration};
use crate::mergers::confidence::{compute_import_confidence, compute_mixed_confidence};

/// The result of merging declarations from two or three sides.
pub(super) struct MergedDeclarations {
    pub declarations: Vec<TsDeclaration>,
    pub confidence: f32,
    pub description: String,
}

/// Returns whether all declarations are import statements.
fn all_imports(decls: &[TsDeclaration]) -> bool {
    !decls.is_empty()
        && decls
            .iter()
            .all(|d| d.kind == DeclarationKind::ImportStatement)
}

/// Builds a map from identity to declaration for fast lookup.
fn build_identity_map(decls: &[TsDeclaration]) -> BTreeMap<TsIdentity, &TsDeclaration> {
    let mut map = BTreeMap::new();
    for decl in decls {
        map.insert(decl.identity.clone(), decl);
    }
    map
}

/// Returns whether two declarations have the same source text (modulo whitespace normalization).
fn source_equal(a: &TsDeclaration, b: &TsDeclaration) -> bool {
    normalize_whitespace(&a.source_text) == normalize_whitespace(&b.source_text)
}

/// Normalizes whitespace for comparison: collapses runs of whitespace to single space, trims.
fn normalize_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Two-way merge: identity-based merge of left and right declarations.
///
/// - If all declarations are imports, delegates to `import_merge`.
/// - For declarations with the same identity but different content, bails out.
/// - Disjoint declarations are combined.
/// - Returns `None` if a conflict is detected that cannot be resolved.
pub(super) fn merge_two_way(
    left: &[TsDeclaration],
    right: &[TsDeclaration],
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

    let mut merged: Vec<TsDeclaration> = Vec::new();
    let mut seen = BTreeSet::new();
    let mut descriptions = Vec::new();
    let mut has_import_merge = false;

    // Collect import declarations for separate merging
    let left_imports: Vec<TsDeclaration> = left
        .iter()
        .filter(|d| d.kind == DeclarationKind::ImportStatement)
        .cloned()
        .collect();
    let right_imports: Vec<TsDeclaration> = right
        .iter()
        .filter(|d| d.kind == DeclarationKind::ImportStatement)
        .cloned()
        .collect();

    if !left_imports.is_empty() || !right_imports.is_empty() {
        if let Some((import_decls, desc)) =
            import_merge::merge_import_declarations(&left_imports, &right_imports, None)
        {
            merged.extend(import_decls);
            descriptions.push(desc);
            has_import_merge = true;
        } else {
            // Identical imports -- keep left's
            merged.extend(left_imports);
        }
        // Mark all import identities as seen
        for decl in left.iter().chain(right.iter()) {
            if decl.kind == DeclarationKind::ImportStatement {
                seen.insert(decl.identity.clone());
            }
        }
    }

    // Process non-import left declarations
    for decl in left {
        if decl.kind == DeclarationKind::ImportStatement {
            continue;
        }
        let id = decl.identity.clone();
        seen.insert(id.clone());

        if let Some(right_decl) = right_map.get(&id) {
            if source_equal(decl, right_decl) {
                // Identical -- keep once
                merged.push(decl.clone());
            } else {
                // Non-import conflict -- bail out (no structural member merging for prototype)
                return None;
            }
        } else {
            // Only on left side
            merged.push(decl.clone());
        }
    }

    // Add right-only non-import declarations
    for decl in right {
        if decl.kind == DeclarationKind::ImportStatement {
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

    let confidence = compute_mixed_confidence(has_import_merge, false, false);
    Some(MergedDeclarations {
        declarations: merged,
        confidence,
        description,
    })
}

/// Three-way merge: classifies each identity as unchanged/added/modified/deleted per side.
#[allow(clippy::too_many_lines)]
pub(super) fn merge_three_way(
    base: &[TsDeclaration],
    left: &[TsDeclaration],
    right: &[TsDeclaration],
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

    let mut merged: Vec<TsDeclaration> = Vec::new();
    let mut seen = BTreeSet::new();
    let mut descriptions = Vec::new();
    let mut has_import_merge = false;

    // Handle import declarations via 3-way import merge
    let base_imports: Vec<TsDeclaration> = base
        .iter()
        .filter(|d| d.kind == DeclarationKind::ImportStatement)
        .cloned()
        .collect();
    let left_imports: Vec<TsDeclaration> = left
        .iter()
        .filter(|d| d.kind == DeclarationKind::ImportStatement)
        .cloned()
        .collect();
    let right_imports: Vec<TsDeclaration> = right
        .iter()
        .filter(|d| d.kind == DeclarationKind::ImportStatement)
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
        } else {
            // Identical imports -- keep left's
            merged.extend(left_imports);
        }
        for decl in base.iter().chain(left.iter()).chain(right.iter()) {
            if decl.kind == DeclarationKind::ImportStatement {
                seen.insert(decl.identity.clone());
            }
        }
    }

    // Collect all non-import identities across all three sides
    let mut all_ids = BTreeSet::new();
    for decl in base.iter().chain(left.iter()).chain(right.iter()) {
        if decl.kind != DeclarationKind::ImportStatement {
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
                let left_changed = !source_equal(b, l);
                let right_changed = !source_equal(b, r);

                match (left_changed, right_changed) {
                    (false, false) => merged.push((*b).clone()),
                    (true, false) => merged.push((*l).clone()),
                    (false, true) => merged.push((*r).clone()),
                    (true, true) => {
                        if source_equal(l, r) {
                            merged.push((*l).clone());
                        } else {
                            return None; // Conflict
                        }
                    }
                }
            }
            // Deleted by one side -- respect deletion (unless modified by the other)
            (Some(b), Some(l), None) => {
                if !source_equal(b, l) {
                    return None; // Modified + deleted = conflict
                }
                // Deleted by right, unchanged by left -- respect deletion
            }
            (Some(b), None, Some(r)) => {
                if !source_equal(b, r) {
                    return None; // Modified + deleted = conflict
                }
                // Deleted by left, unchanged by right -- respect deletion
            }
            // Both sides deleted, or identity not actually present
            (Some(_) | None, None, None) => {}
            // Added by both sides
            (None, Some(l), Some(r)) => {
                if source_equal(l, r) {
                    merged.push((*l).clone());
                } else {
                    return None; // Conflict
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

    let confidence = compute_mixed_confidence(has_import_merge, false, true);
    Some(MergedDeclarations {
        declarations: merged,
        confidence,
        description,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mergers::typescript_merger::parse::parse_fragment;

    fn parse(text: &str) -> Vec<TsDeclaration> {
        match parse_fragment(text) {
            super::super::parse::ParsedFragment::Declarations(decls) => decls,
            super::super::parse::ParsedFragment::Unparsable => {
                panic!("failed to parse: {text}")
            }
        }
    }

    #[test]
    fn two_way_disjoint_functions() {
        let left = parse("function foo() {}");
        let right = parse("function bar() {}");
        let result = merge_two_way(&left, &right).unwrap();
        assert_eq!(result.declarations.len(), 2);
    }

    #[test]
    fn two_way_identical_functions() {
        let left = parse("function foo() {}");
        let right = parse("function foo() {}");
        let result = merge_two_way(&left, &right).unwrap();
        assert_eq!(result.declarations.len(), 1);
    }

    #[test]
    fn two_way_conflicting_functions() {
        let left = parse("function foo() { return 1; }");
        let right = parse("function foo() { return 2; }");
        let result = merge_two_way(&left, &right);
        assert!(result.is_none(), "conflicting functions should return None");
    }

    #[test]
    fn three_way_one_side_modified() {
        let base = parse("function foo() { return 1; }");
        let left = parse("function foo() { return 42; }");
        let right = parse("function foo() { return 1; }");
        let result = merge_three_way(&base, &left, &right).unwrap();
        assert!(result.declarations[0].source_text.contains("42"));
    }

    #[test]
    fn mixed_imports_and_declarations() {
        let left = parse("import { A } from 'x';\nfunction foo() {}");
        let right = parse("import { B } from 'x';\nfunction bar() {}");
        let result = merge_two_way(&left, &right).unwrap();
        assert!(result.declarations.len() >= 3); // merged import + foo + bar
    }
}
