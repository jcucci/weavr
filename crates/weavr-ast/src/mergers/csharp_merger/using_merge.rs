//! Using-directive flattening, deduplication, sorting, and merging.
//!
//! C# `using` directives are a common source of merge conflicts.
//! This module handles them by extracting normalized paths into sets,
//! then performing set operations (union / difference) for deterministic,
//! sorted output.

use std::collections::BTreeSet;

use super::parse::{is_complex_using, CSharpDeclaration, DeclarationKind};

/// Extracts all simple using paths from a list of declarations.
pub(super) fn collect_using_paths(decls: &[CSharpDeclaration]) -> BTreeSet<String> {
    let mut paths = BTreeSet::new();
    for decl in decls {
        if decl.kind == DeclarationKind::UsingDirective {
            if let super::identity::CSharpIdentity::Using(ref path) = decl.identity {
                paths.insert(path.clone());
            }
        }
    }
    paths
}

/// Returns `true` if any using declaration is complex (static, alias, global, attributed).
fn has_complex_usings(decls: &[CSharpDeclaration]) -> bool {
    decls
        .iter()
        .any(|d| d.kind == DeclarationKind::UsingDirective && is_complex_using(d))
}

/// Merges using directives from two (or three) sides.
///
/// - **2-way** (no base): takes the union of all using paths.
/// - **3-way**: respects deletions -- if a path was in base but removed by one side,
///   it stays removed unless the other side also added it fresh.
///
/// Returns `None` if both sides have identical using paths, or if any using directive
/// is complex (static, alias, global) and would lose semantics during flattening.
pub(super) fn merge_using_directives(
    left: &[CSharpDeclaration],
    right: &[CSharpDeclaration],
    base: Option<&[CSharpDeclaration]>,
) -> Option<(Vec<CSharpDeclaration>, String)> {
    // Bail out when usings carry metadata we can't preserve through flattening
    if has_complex_usings(left)
        || has_complex_usings(right)
        || base.as_ref().is_some_and(|b| has_complex_usings(b))
    {
        return None;
    }

    let left_paths = collect_using_paths(left);
    let right_paths = collect_using_paths(right);

    let merged_paths = if let Some(base_decls) = base {
        let base_paths = collect_using_paths(base_decls);
        three_way_merge_paths(&base_paths, &left_paths, &right_paths)
    } else {
        // 2-way: union
        left_paths.union(&right_paths).cloned().collect()
    };

    if merged_paths == left_paths && merged_paths == right_paths {
        return None;
    }

    let count = merged_paths.len();
    let decls: Vec<CSharpDeclaration> = merged_paths
        .iter()
        .map(|path| CSharpDeclaration {
            kind: DeclarationKind::UsingDirective,
            identity: super::identity::CSharpIdentity::Using(path.clone()),
            source_text: format!("using {path};"),
            children: Vec::new(),
        })
        .collect();

    let description = format!("Merged {count} using directives");
    Some((decls, description))
}

/// Three-way merge for using paths: union of both sides' additions,
/// minus paths deleted by either side (that weren't re-added by the other).
fn three_way_merge_paths(
    base: &BTreeSet<String>,
    left: &BTreeSet<String>,
    right: &BTreeSet<String>,
) -> BTreeSet<String> {
    let mut result = base.clone();

    // Add new paths from left (not in base)
    for path in left.difference(base) {
        result.insert(path.clone());
    }

    // Add new paths from right (not in base)
    for path in right.difference(base) {
        result.insert(path.clone());
    }

    // Remove paths deleted by left (in base but not in left)
    for path in base.difference(left) {
        result.remove(path);
    }

    // Remove paths deleted by right (in base but not in right)
    for path in base.difference(right) {
        result.remove(path);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mergers::csharp_merger::identity::CSharpIdentity;

    fn make_using(path: &str) -> CSharpDeclaration {
        CSharpDeclaration {
            kind: DeclarationKind::UsingDirective,
            identity: CSharpIdentity::Using(path.to_string()),
            source_text: format!("using {path};"),
            children: Vec::new(),
        }
    }

    fn make_static_using(path: &str) -> CSharpDeclaration {
        CSharpDeclaration {
            kind: DeclarationKind::UsingDirective,
            identity: CSharpIdentity::Using(format!("static {path}")),
            source_text: format!("using static {path};"),
            children: Vec::new(),
        }
    }

    #[test]
    fn two_way_union() {
        let left = vec![make_using("System"), make_using("System.IO")];
        let right = vec![make_using("System"), make_using("System.Linq")];
        let (decls, desc) = merge_using_directives(&left, &right, None).unwrap();
        assert_eq!(decls.len(), 3);
        assert!(desc.contains('3'));
    }

    #[test]
    fn two_way_identical_returns_none() {
        let left = vec![make_using("System")];
        let right = vec![make_using("System")];
        let result = merge_using_directives(&left, &right, None);
        assert!(result.is_none());
    }

    #[test]
    fn three_way_respects_deletion() {
        let base = vec![
            make_using("System"),
            make_using("System.IO"),
            make_using("System.Linq"),
        ];
        let left = vec![make_using("System"), make_using("System.Linq")]; // removed IO
        let right = vec![
            make_using("System"),
            make_using("System.IO"),
            make_using("System.Linq"),
        ]; // unchanged
        let (decls, _) = merge_using_directives(&left, &right, Some(&base)).unwrap();
        let paths = collect_using_paths(&decls);
        assert!(!paths.contains("System.IO"), "IO should be deleted");
        assert!(paths.contains("System"));
        assert!(paths.contains("System.Linq"));
    }

    #[test]
    fn three_way_adds_from_both_sides() {
        let base = vec![make_using("System")];
        let left = vec![make_using("System"), make_using("System.IO")];
        let right = vec![make_using("System"), make_using("System.Linq")];
        let (decls, _) = merge_using_directives(&left, &right, Some(&base)).unwrap();
        let paths = collect_using_paths(&decls);
        assert_eq!(paths.len(), 3);
        assert!(paths.contains("System"));
        assert!(paths.contains("System.IO"));
        assert!(paths.contains("System.Linq"));
    }

    #[test]
    fn bails_out_on_static_using() {
        let left = vec![make_static_using("System.Math")];
        let right = vec![make_using("System.IO")];
        let result = merge_using_directives(&left, &right, None);
        assert!(result.is_none(), "should bail out for static using");
    }

    #[test]
    fn bails_out_on_alias_using() {
        let alias = CSharpDeclaration {
            kind: DeclarationKind::UsingDirective,
            identity: CSharpIdentity::Using("Alias = System.IO".to_string()),
            source_text: "using Alias = System.IO;".to_string(),
            children: Vec::new(),
        };
        let right = vec![make_using("System")];
        let result = merge_using_directives(&[alias], &right, None);
        assert!(result.is_none(), "should bail out for alias using");
    }

    #[test]
    fn bails_out_on_global_using() {
        let global = CSharpDeclaration {
            kind: DeclarationKind::UsingDirective,
            identity: CSharpIdentity::Using("System".to_string()),
            source_text: "global using System;".to_string(),
            children: Vec::new(),
        };
        let right = vec![make_using("System.IO")];
        let result = merge_using_directives(&[global], &right, None);
        assert!(result.is_none(), "should bail out for global using");
    }

    #[test]
    fn deterministic_ordering() {
        let left = vec![make_using("System.Linq"), make_using("System.IO")];
        let right = vec![make_using("System.IO"), make_using("System.Collections")];
        let (decls, _) = merge_using_directives(&left, &right, None).unwrap();
        let paths: Vec<_> = decls
            .iter()
            .filter_map(|d| {
                if let CSharpIdentity::Using(ref p) = d.identity {
                    Some(p.clone())
                } else {
                    None
                }
            })
            .collect();
        // Should be sorted
        let mut sorted = paths.clone();
        sorted.sort();
        assert_eq!(paths, sorted);
    }
}
