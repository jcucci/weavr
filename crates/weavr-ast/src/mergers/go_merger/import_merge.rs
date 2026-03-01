//! Import declaration flattening, deduplication, grouping, and merging.
//!
//! Go `import` declarations are a common source of merge conflicts.
//! This module handles them by extracting import keys into sets,
//! then performing set operations (union / difference) for deterministic,
//! grouped output following Go conventions.

use std::collections::BTreeSet;

use super::identity::{GoIdentity, ImportKey, ImportKind};
use super::parse::{is_dot_import, DeclarationKind, GoDeclaration};
use crate::mergers::common::three_way_merge_sets;

/// Extracts all import keys from a list of declarations.
pub(super) fn collect_import_keys(decls: &[GoDeclaration]) -> BTreeSet<ImportKey> {
    let mut keys = BTreeSet::new();
    for decl in decls {
        if decl.kind != DeclarationKind::ImportDeclaration {
            continue;
        }
        // For single imports, use the declaration's identity directly
        if let GoIdentity::Import(ref key) = decl.identity {
            keys.insert(key.clone());
        }
        // For grouped imports, collect from children
        for child in &decl.children {
            if let GoIdentity::Import(ref key) = child.identity {
                keys.insert(key.clone());
            }
        }
    }
    keys
}

/// Returns `true` if any import declaration contains dot imports.
fn has_dot_imports(decls: &[GoDeclaration]) -> bool {
    decls
        .iter()
        .any(|d| d.kind == DeclarationKind::ImportDeclaration && is_dot_import(d))
}

/// Returns `true` if a path is a standard library import (no `.` in path).
fn is_stdlib_import(path: &str) -> bool {
    !path.contains('.')
}

/// Merges import declarations from two (or three) sides.
///
/// - **2-way** (no base): takes the union of all import keys.
/// - **3-way**: respects deletions — if an import was in base but removed by one side,
///   it stays removed unless the other side also added it fresh.
///
/// Returns `None` if both sides have identical import keys, or if any import
/// is a dot import (which would lose semantics during flattening).
pub(super) fn merge_import_declarations(
    left: &[GoDeclaration],
    right: &[GoDeclaration],
    base: Option<&[GoDeclaration]>,
) -> Option<(Vec<GoDeclaration>, String)> {
    // Bail out when imports carry metadata we can't preserve through flattening
    if has_dot_imports(left)
        || has_dot_imports(right)
        || base.as_ref().is_some_and(|b| has_dot_imports(b))
    {
        return None;
    }

    let left_keys = collect_import_keys(left);
    let right_keys = collect_import_keys(right);

    let merged_keys = if let Some(base_decls) = base {
        let base_keys = collect_import_keys(base_decls);
        three_way_merge_sets(&base_keys, &left_keys, &right_keys)
    } else {
        // 2-way: union
        left_keys.union(&right_keys).cloned().collect()
    };

    if merged_keys == left_keys && merged_keys == right_keys {
        return None;
    }

    let count = merged_keys.len();

    // Build a single grouped import declaration with Go-conventional grouping
    let import_text = format_grouped_import(&merged_keys);
    let decls = vec![GoDeclaration {
        kind: DeclarationKind::ImportDeclaration,
        identity: GoIdentity::Unknown(import_text.clone()),
        source_text: import_text,
        children: merged_keys
            .iter()
            .map(|key| GoDeclaration {
                kind: DeclarationKind::ImportDeclaration,
                identity: GoIdentity::Import(key.clone()),
                source_text: format_import_spec(key),
                children: Vec::new(),
            })
            .collect(),
    }];

    let description = format!("Merged {count} imports");
    Some((decls, description))
}

/// Formats a set of import keys into a grouped import block following Go conventions:
///
/// Group 1: stdlib imports (no `.` in path)
/// Group 2: external imports (contain `.` in path)
/// Blank line separator between groups, alphabetical within each group.
fn format_grouped_import(keys: &BTreeSet<ImportKey>) -> String {
    let mut stdlib: Vec<&ImportKey> = Vec::new();
    let mut external: Vec<&ImportKey> = Vec::new();

    for key in keys {
        if is_stdlib_import(&key.path) {
            stdlib.push(key);
        } else {
            external.push(key);
        }
    }

    // Already sorted by BTreeSet ordering

    if stdlib.is_empty() && external.is_empty() {
        return String::new();
    }

    // Single import, no grouping needed
    if stdlib.len() + external.len() == 1 {
        let key = stdlib.first().or(external.first()).unwrap();
        return format!("import {}", format_import_spec(key));
    }

    let mut lines = Vec::new();
    lines.push("import (".to_string());

    for key in &stdlib {
        lines.push(format!("\t{}", format_import_spec(key)));
    }

    // Blank line separator between groups if both are non-empty
    if !stdlib.is_empty() && !external.is_empty() {
        lines.push(String::new());
    }

    for key in &external {
        lines.push(format!("\t{}", format_import_spec(key)));
    }

    lines.push(")".to_string());
    lines.join("\n")
}

/// Formats a single import spec from an `ImportKey`.
fn format_import_spec(key: &ImportKey) -> String {
    match &key.kind {
        ImportKind::Normal => format!("\"{}\"", key.path),
        ImportKind::Blank => format!("_ \"{}\"", key.path),
        ImportKind::Named(alias) => format!("{alias} \"{}\"", key.path),
        ImportKind::Dot => format!(". \"{}\"", key.path),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_import(path: &str) -> GoDeclaration {
        let key = ImportKey {
            path: path.to_string(),
            kind: ImportKind::Normal,
        };
        GoDeclaration {
            kind: DeclarationKind::ImportDeclaration,
            identity: GoIdentity::Import(key.clone()),
            source_text: format!("import \"{path}\""),
            children: vec![GoDeclaration {
                kind: DeclarationKind::ImportDeclaration,
                identity: GoIdentity::Import(key),
                source_text: format!("\"{path}\""),
                children: Vec::new(),
            }],
        }
    }

    fn make_named_import(path: &str, alias: &str) -> GoDeclaration {
        let key = ImportKey {
            path: path.to_string(),
            kind: ImportKind::Named(alias.to_string()),
        };
        GoDeclaration {
            kind: DeclarationKind::ImportDeclaration,
            identity: GoIdentity::Import(key.clone()),
            source_text: format!("import {alias} \"{path}\""),
            children: vec![GoDeclaration {
                kind: DeclarationKind::ImportDeclaration,
                identity: GoIdentity::Import(key),
                source_text: format!("{alias} \"{path}\""),
                children: Vec::new(),
            }],
        }
    }

    fn make_blank_import(path: &str) -> GoDeclaration {
        let key = ImportKey {
            path: path.to_string(),
            kind: ImportKind::Blank,
        };
        GoDeclaration {
            kind: DeclarationKind::ImportDeclaration,
            identity: GoIdentity::Import(key.clone()),
            source_text: format!("import _ \"{path}\""),
            children: vec![GoDeclaration {
                kind: DeclarationKind::ImportDeclaration,
                identity: GoIdentity::Import(key),
                source_text: format!("_ \"{path}\""),
                children: Vec::new(),
            }],
        }
    }

    fn make_dot_import(path: &str) -> GoDeclaration {
        let key = ImportKey {
            path: path.to_string(),
            kind: ImportKind::Dot,
        };
        GoDeclaration {
            kind: DeclarationKind::ImportDeclaration,
            identity: GoIdentity::Import(key.clone()),
            source_text: format!("import . \"{path}\""),
            children: vec![GoDeclaration {
                kind: DeclarationKind::ImportDeclaration,
                identity: GoIdentity::Import(key),
                source_text: format!(". \"{path}\""),
                children: Vec::new(),
            }],
        }
    }

    #[test]
    fn two_way_union() {
        let left = vec![make_import("fmt"), make_import("os")];
        let right = vec![make_import("fmt"), make_import("io")];
        let (decls, desc) = merge_import_declarations(&left, &right, None).unwrap();
        let keys = collect_import_keys(&decls);
        assert_eq!(keys.len(), 3);
        assert!(desc.contains('3'));
    }

    #[test]
    fn two_way_identical_returns_none() {
        let left = vec![make_import("fmt")];
        let right = vec![make_import("fmt")];
        let result = merge_import_declarations(&left, &right, None);
        assert!(result.is_none());
    }

    #[test]
    fn three_way_respects_deletion() {
        let base = vec![make_import("fmt"), make_import("os"), make_import("io")];
        let left = vec![make_import("fmt"), make_import("io")]; // removed os
        let right = vec![make_import("fmt"), make_import("os"), make_import("io")]; // unchanged
        let (decls, _) = merge_import_declarations(&left, &right, Some(&base)).unwrap();
        let keys = collect_import_keys(&decls);
        assert!(
            !keys.contains(&ImportKey {
                path: "os".to_string(),
                kind: ImportKind::Normal
            }),
            "os should be deleted"
        );
        assert!(keys.contains(&ImportKey {
            path: "fmt".to_string(),
            kind: ImportKind::Normal
        }));
        assert!(keys.contains(&ImportKey {
            path: "io".to_string(),
            kind: ImportKind::Normal
        }));
    }

    #[test]
    fn three_way_adds_from_both_sides() {
        let base = vec![make_import("fmt")];
        let left = vec![make_import("fmt"), make_import("os")];
        let right = vec![make_import("fmt"), make_import("io")];
        let (decls, _) = merge_import_declarations(&left, &right, Some(&base)).unwrap();
        let keys = collect_import_keys(&decls);
        assert_eq!(keys.len(), 3);
    }

    #[test]
    fn stdlib_external_grouping() {
        let left = vec![make_import("fmt"), make_import("github.com/pkg/errors")];
        let right = vec![
            make_import("os"),
            make_import("github.com/stretchr/testify"),
        ];
        let (decls, _) = merge_import_declarations(&left, &right, None).unwrap();
        let text = &decls[0].source_text;
        // Verify grouping structure
        assert!(text.contains("import ("));
        assert!(text.contains("\"fmt\""));
        assert!(text.contains("\"os\""));
        assert!(text.contains("\"github.com/pkg/errors\""));
        assert!(text.contains("\"github.com/stretchr/testify\""));

        // Verify stdlib comes before external (with blank line separator)
        let fmt_pos = text.find("\"fmt\"").unwrap();
        let errors_pos = text.find("\"github.com/pkg/errors\"").unwrap();
        assert!(fmt_pos < errors_pos, "stdlib should come before external");
    }

    #[test]
    fn named_import_preserved() {
        let left = vec![make_named_import("fmt", "f")];
        let right = vec![make_import("os")];
        let (decls, _) = merge_import_declarations(&left, &right, None).unwrap();
        let keys = collect_import_keys(&decls);
        assert!(keys.contains(&ImportKey {
            path: "fmt".to_string(),
            kind: ImportKind::Named("f".to_string()),
        }));
    }

    #[test]
    fn blank_import_preserved() {
        let left = vec![make_blank_import("database/sql")];
        let right = vec![make_import("fmt")];
        let (decls, _) = merge_import_declarations(&left, &right, None).unwrap();
        let keys = collect_import_keys(&decls);
        assert!(keys.contains(&ImportKey {
            path: "database/sql".to_string(),
            kind: ImportKind::Blank,
        }));
    }

    #[test]
    fn dot_import_causes_bailout() {
        let left = vec![make_dot_import("fmt")];
        let right = vec![make_import("os")];
        let result = merge_import_declarations(&left, &right, None);
        assert!(result.is_none(), "dot imports should bail out");
    }

    #[test]
    fn deterministic_ordering() {
        let left = vec![make_import("os"), make_import("fmt")];
        let right = vec![make_import("io"), make_import("fmt")];
        let (decls1, _) = merge_import_declarations(&left, &right, None).unwrap();
        let (decls2, _) = merge_import_declarations(&right, &left, None).unwrap();
        // Both should produce the same sorted output
        let keys1 = collect_import_keys(&decls1);
        let keys2 = collect_import_keys(&decls2);
        assert_eq!(keys1, keys2);
    }
}
