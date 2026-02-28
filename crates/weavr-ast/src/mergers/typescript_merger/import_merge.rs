//! Import-specific set-based merge logic for TypeScript.
//!
//! TypeScript `import` statements are a common source of merge conflicts.
//! This module handles them by extracting specifiers into ordered sets,
//! then performing set operations (union / three-way merge) for deterministic,
//! sorted output.

use std::collections::BTreeSet;

use super::identity::{ImportKey, ImportKind, ImportSpecifier, TsIdentity};
use super::parse::{DeclarationKind, TsDeclaration};
use crate::mergers::common::three_way_merge_sets;

/// Extracts all import declarations grouped by their `ImportKey`.
fn collect_imports_by_key(
    decls: &[TsDeclaration],
) -> std::collections::BTreeMap<ImportKey, BTreeSet<ImportSpecifier>> {
    let mut map = std::collections::BTreeMap::new();
    for decl in decls {
        if decl.kind == DeclarationKind::ImportStatement {
            if let TsIdentity::Import(ref key) = decl.identity {
                let entry = map.entry(key.clone()).or_insert_with(BTreeSet::new);
                for spec in &decl.specifiers {
                    entry.insert(spec.clone());
                }
            }
        }
    }
    map
}

/// Merges import declarations from two (or three) sides.
///
/// - **2-way** (no base): takes the union of all import specifiers per key.
/// - **3-way**: respects deletions using `three_way_merge_sets()`.
///
/// Returns `None` if both sides have identical imports.
pub(super) fn merge_import_declarations(
    left: &[TsDeclaration],
    right: &[TsDeclaration],
    base: Option<&[TsDeclaration]>,
) -> Option<(Vec<TsDeclaration>, String)> {
    let left_map = collect_imports_by_key(left);
    let right_map = collect_imports_by_key(right);

    // Collect all import keys from both sides
    let mut all_keys: BTreeSet<ImportKey> = BTreeSet::new();
    for key in left_map.keys() {
        all_keys.insert(key.clone());
    }
    for key in right_map.keys() {
        all_keys.insert(key.clone());
    }

    let base_map = base.map(collect_imports_by_key);
    if let Some(ref bm) = base_map {
        for key in bm.keys() {
            all_keys.insert(key.clone());
        }
    }

    let empty = BTreeSet::new();
    let mut merged_decls = Vec::new();
    let mut any_change = false;

    for key in &all_keys {
        let in_left = left_map.contains_key(key);
        let in_right = right_map.contains_key(key);
        let left_specs = left_map.get(key).unwrap_or(&empty);
        let right_specs = right_map.get(key).unwrap_or(&empty);

        let merged_specs = if let Some(ref bm) = base_map {
            let in_base = bm.contains_key(key);
            let base_specs = bm.get(key).unwrap_or(&empty);
            let result = three_way_merge_sets(base_specs, left_specs, right_specs);

            // For side-effect/namespace imports (no specifiers), check presence instead
            if is_specifier_free_import(key.kind) {
                if in_base && !in_left && !in_right {
                    continue; // Deleted by both sides
                }
                if in_base && (!in_left || !in_right) {
                    continue; // Deleted by one side
                }
            } else if result.is_empty() {
                continue; // All specifiers deleted
            }

            result
        } else {
            // 2-way: union -- both sides are present by definition of all_keys
            left_specs.union(right_specs).cloned().collect()
        };

        if &merged_specs != left_specs || &merged_specs != right_specs {
            any_change = true;
        }

        // For specifier-free imports in 2-way, check if one side is new
        if !any_change && is_specifier_free_import(key.kind) && (in_left != in_right) {
            any_change = true;
        }

        merged_decls.push(reconstruct_import(key, &merged_specs));
    }

    if !any_change {
        return None;
    }

    let count = merged_decls.len();
    let description = format!("Merged {count} import statements");
    Some((merged_decls, description))
}

/// Returns `true` for import kinds that inherently have no specifiers.
fn is_specifier_free_import(kind: ImportKind) -> bool {
    matches!(kind, ImportKind::SideEffect | ImportKind::Namespace)
}

/// Reconstructs an import declaration from a key and merged specifier set.
fn reconstruct_import(key: &ImportKey, specifiers: &BTreeSet<ImportSpecifier>) -> TsDeclaration {
    let source_text = format_import(key, specifiers);

    TsDeclaration {
        kind: DeclarationKind::ImportStatement,
        identity: TsIdentity::Import(key.clone()),
        source_text,
        specifiers: specifiers.clone(),
    }
}

/// Formats an import statement from its key and specifiers.
fn format_import(key: &ImportKey, specifiers: &BTreeSet<ImportSpecifier>) -> String {
    let module = &key.module;
    let quote = '\'';

    match key.kind {
        ImportKind::SideEffect => {
            format!("import {quote}{module}{quote};")
        }
        ImportKind::Namespace => {
            // Namespace imports don't have specifiers; we preserve the original name
            // by formatting a generic `* as <module_name>` pattern.
            // The actual alias is lost during set merging, but namespace imports
            // are identity-matched and not specifier-merged.
            format!("import * as {module} from {quote}{module}{quote};")
        }
        ImportKind::TypeOnly => {
            if specifiers.is_empty() {
                format!("import type {{}} from {quote}{module}{quote};")
            } else {
                let specs = format_specifiers(specifiers);
                format!("import type {{ {specs} }} from {quote}{module}{quote};")
            }
        }
        ImportKind::Value => {
            if specifiers.is_empty() {
                format!("import {{}} from {quote}{module}{quote};")
            } else {
                let specs = format_specifiers(specifiers);
                format!("import {{ {specs} }} from {quote}{module}{quote};")
            }
        }
    }
}

/// Formats import specifiers as a comma-separated string.
fn format_specifiers(specifiers: &BTreeSet<ImportSpecifier>) -> String {
    specifiers
        .iter()
        .map(|s| {
            let mut result = String::new();
            if s.is_type {
                result.push_str("type ");
            }
            result.push_str(&s.name);
            if let Some(ref alias) = s.alias {
                result.push_str(" as ");
                result.push_str(alias);
            }
            result
        })
        .collect::<Vec<_>>()
        .join(", ")
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

    fn import_only(decls: &[TsDeclaration]) -> Vec<TsDeclaration> {
        decls
            .iter()
            .filter(|d| d.kind == DeclarationKind::ImportStatement)
            .cloned()
            .collect()
    }

    #[test]
    fn two_way_union_same_module() {
        let left = parse("import { useState } from 'react';");
        let right = parse("import { useEffect } from 'react';");
        let (decls, desc) =
            merge_import_declarations(&import_only(&left), &import_only(&right), None).unwrap();
        assert_eq!(decls.len(), 1);
        assert!(decls[0].specifiers.len() >= 2);
        assert!(desc.contains('1'));
    }

    #[test]
    fn two_way_dedup() {
        let left = parse("import { useState } from 'react';");
        let right = parse("import { useState } from 'react';");
        let result = merge_import_declarations(&import_only(&left), &import_only(&right), None);
        assert!(result.is_none(), "identical imports should return None");
    }

    #[test]
    fn two_way_different_modules() {
        let left = parse("import { useState } from 'react';");
        let right = parse("import { render } from 'react-dom';");
        let (decls, _) =
            merge_import_declarations(&import_only(&left), &import_only(&right), None).unwrap();
        assert_eq!(decls.len(), 2);
    }

    #[test]
    fn three_way_addition_from_both() {
        let base = parse("import { useState } from 'react';");
        let left = parse("import { useState, useEffect } from 'react';");
        let right = parse("import { useState, useMemo } from 'react';");
        let (decls, _) = merge_import_declarations(
            &import_only(&left),
            &import_only(&right),
            Some(&import_only(&base)),
        )
        .unwrap();
        assert_eq!(decls.len(), 1);
        let names: BTreeSet<_> = decls[0]
            .specifiers
            .iter()
            .map(|s| s.name.as_str())
            .collect();
        assert!(names.contains("useState"));
        assert!(names.contains("useEffect"));
        assert!(names.contains("useMemo"));
    }

    #[test]
    fn three_way_deletion_respected() {
        let base = parse("import { useState, useEffect } from 'react';");
        let left = parse("import { useState } from 'react';"); // removed useEffect
        let right = parse("import { useState, useEffect } from 'react';"); // unchanged
        let (decls, _) = merge_import_declarations(
            &import_only(&left),
            &import_only(&right),
            Some(&import_only(&base)),
        )
        .unwrap();
        assert_eq!(decls.len(), 1);
        let names: BTreeSet<_> = decls[0]
            .specifiers
            .iter()
            .map(|s| s.name.as_str())
            .collect();
        assert!(names.contains("useState"));
        assert!(!names.contains("useEffect"), "useEffect should be deleted");
    }

    #[test]
    fn type_only_kept_separate() {
        let left = parse("import { useState } from 'react';");
        let right = parse("import type { FC } from 'react';");
        let (decls, _) =
            merge_import_declarations(&import_only(&left), &import_only(&right), None).unwrap();
        assert_eq!(
            decls.len(),
            2,
            "value and type imports should stay separate"
        );
    }

    #[test]
    fn deterministic_ordering() {
        let left = parse("import { useEffect, useState } from 'react';");
        let right = parse("import { useMemo } from 'react';");
        let (decls1, _) =
            merge_import_declarations(&import_only(&left), &import_only(&right), None).unwrap();
        let (decls2, _) =
            merge_import_declarations(&import_only(&left), &import_only(&right), None).unwrap();
        assert_eq!(decls1[0].source_text, decls2[0].source_text);
    }

    #[test]
    fn format_import_value() {
        let key = ImportKey {
            module: "react".to_string(),
            kind: ImportKind::Value,
        };
        let mut specs = BTreeSet::new();
        specs.insert(ImportSpecifier {
            name: "useState".to_string(),
            alias: None,
            is_type: false,
        });
        let result = format_import(&key, &specs);
        assert_eq!(result, "import { useState } from 'react';");
    }

    #[test]
    fn format_import_type_only() {
        let key = ImportKey {
            module: "./types".to_string(),
            kind: ImportKind::TypeOnly,
        };
        let mut specs = BTreeSet::new();
        specs.insert(ImportSpecifier {
            name: "Props".to_string(),
            alias: None,
            is_type: false,
        });
        let result = format_import(&key, &specs);
        assert_eq!(result, "import type { Props } from './types';");
    }

    #[test]
    fn format_import_side_effect() {
        let key = ImportKey {
            module: "./polyfill".to_string(),
            kind: ImportKind::SideEffect,
        };
        let result = format_import(&key, &BTreeSet::new());
        assert_eq!(result, "import './polyfill';");
    }
}
