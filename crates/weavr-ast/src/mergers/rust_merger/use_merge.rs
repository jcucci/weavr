//! Use-statement flattening, deduplication, sorting, and merging.
//!
//! Rust `use` statements are the most common source of merge conflicts.
//! This module handles them specially by flattening grouped imports into
//! individual paths, then performing set operations (union / difference)
//! for deterministic, sorted output.

use std::collections::BTreeSet;

use syn::{Item, UseTree};

use crate::AstError;

/// Recursively flattens a `UseTree` into individual path strings.
///
/// For example, `use std::{io, fs}` becomes `["std::io", "std::fs"]`.
fn flatten_use_tree(prefix: &str, tree: &UseTree) -> BTreeSet<String> {
    let mut paths = BTreeSet::new();
    match tree {
        UseTree::Path(p) => {
            let new_prefix = if prefix.is_empty() {
                p.ident.to_string()
            } else {
                format!("{prefix}::{}", p.ident)
            };
            paths.extend(flatten_use_tree(&new_prefix, &p.tree));
        }
        UseTree::Name(n) => {
            let path = if prefix.is_empty() {
                n.ident.to_string()
            } else {
                format!("{prefix}::{}", n.ident)
            };
            paths.insert(path);
        }
        UseTree::Rename(r) => {
            let path = if prefix.is_empty() {
                format!("{} as {}", r.ident, r.rename)
            } else {
                format!("{prefix}::{} as {}", r.ident, r.rename)
            };
            paths.insert(path);
        }
        UseTree::Glob(_) => {
            let path = if prefix.is_empty() {
                "*".to_string()
            } else {
                format!("{prefix}::*")
            };
            paths.insert(path);
        }
        UseTree::Group(g) => {
            for item in &g.items {
                paths.extend(flatten_use_tree(prefix, item));
            }
        }
    }
    paths
}

/// Extracts all flattened use paths from a list of items.
pub(super) fn collect_use_paths(items: &[Item]) -> BTreeSet<String> {
    let mut paths = BTreeSet::new();
    for item in items {
        if let Item::Use(u) = item {
            paths.extend(flatten_use_tree("", &u.tree));
        }
    }
    paths
}

/// Builds a `syn::Item::Use` from a fully-qualified path string like `"std::io::Read"`.
fn path_to_use_item(path: &str) -> Result<Item, AstError> {
    let code = format!("use {path};");
    let file = syn::parse_file(&code).map_err(|e| {
        AstError::Internal(format!("failed to reconstruct use item for `{path}`: {e}"))
    })?;
    file.items
        .into_iter()
        .next()
        .ok_or_else(|| AstError::Internal(format!("no item produced for use path `{path}`")))
}

/// Merges use statements from two (or three) sides.
///
/// - **2-way** (no base): takes the union of all use paths.
/// - **3-way**: respects deletions — if a path was in base but removed by one side,
///   it stays removed unless the other side also added it fresh.
///
/// Returns `None` if both sides have identical use paths.
pub(super) fn merge_use_items(
    left: &[Item],
    right: &[Item],
    base: Option<&[Item]>,
) -> Result<Option<(Vec<Item>, String)>, AstError> {
    let left_paths = collect_use_paths(left);
    let right_paths = collect_use_paths(right);

    let merged_paths = if let Some(base_items) = base {
        let base_paths = collect_use_paths(base_items);
        three_way_merge_paths(&base_paths, &left_paths, &right_paths)
    } else {
        // 2-way: union
        left_paths.union(&right_paths).cloned().collect()
    };

    if merged_paths == left_paths && merged_paths == right_paths {
        return Ok(None);
    }

    let count = merged_paths.len();
    let mut items = Vec::with_capacity(count);
    for path in &merged_paths {
        items.push(path_to_use_item(path)?);
    }

    let description = format!("Merged {count} use statements");
    Ok(Some((items, description)))
}

/// Three-way merge for use paths: union of both sides' additions,
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

    // Remove paths deleted by left (in base but not in left),
    // unless right explicitly added them (not in base but in right — already handled above)
    for path in base.difference(left) {
        if !right.difference(base).any(|p| p == path) {
            result.remove(path);
        }
    }

    // Remove paths deleted by right (in base but not in right),
    // unless left explicitly added them
    for path in base.difference(right) {
        if !left.difference(base).any(|p| p == path) {
            result.remove(path);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_items(code: &str) -> Vec<Item> {
        syn::parse_file(code).unwrap().items
    }

    #[test]
    fn flatten_simple_use() {
        let items = parse_items("use std::io;");
        let paths = collect_use_paths(&items);
        assert_eq!(paths, BTreeSet::from(["std::io".to_string()]));
    }

    #[test]
    fn flatten_grouped_use() {
        let items = parse_items("use std::{io, fs};");
        let paths = collect_use_paths(&items);
        assert_eq!(
            paths,
            BTreeSet::from(["std::fs".to_string(), "std::io".to_string()])
        );
    }

    #[test]
    fn flatten_glob_use() {
        let items = parse_items("use std::io::*;");
        let paths = collect_use_paths(&items);
        assert_eq!(paths, BTreeSet::from(["std::io::*".to_string()]));
    }

    #[test]
    fn flatten_rename_use() {
        let items = parse_items("use std::io::Read as IoRead;");
        let paths = collect_use_paths(&items);
        assert_eq!(
            paths,
            BTreeSet::from(["std::io::Read as IoRead".to_string()])
        );
    }

    #[test]
    fn two_way_union() {
        let left = parse_items("use std::io;\nuse std::fs;");
        let right = parse_items("use std::io;\nuse std::net;");
        let result = merge_use_items(&left, &right, None).unwrap();
        let (items, desc) = result.unwrap();
        assert_eq!(items.len(), 3);
        assert!(desc.contains('3'));
    }

    #[test]
    fn two_way_identical_returns_none() {
        let left = parse_items("use std::io;");
        let right = parse_items("use std::io;");
        let result = merge_use_items(&left, &right, None).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn three_way_respects_deletion() {
        let base = parse_items("use std::io;\nuse std::fs;\nuse std::net;");
        let left = parse_items("use std::io;\nuse std::net;"); // removed fs
        let right = parse_items("use std::io;\nuse std::fs;\nuse std::net;"); // unchanged
        let result = merge_use_items(&left, &right, Some(&base)).unwrap();
        let (items, _) = result.unwrap();
        let paths = collect_use_paths(&items);
        assert!(!paths.contains("std::fs"), "fs should be deleted");
        assert!(paths.contains("std::io"));
        assert!(paths.contains("std::net"));
    }

    #[test]
    fn three_way_adds_from_both_sides() {
        let base = parse_items("use std::io;");
        let left = parse_items("use std::io;\nuse std::fs;");
        let right = parse_items("use std::io;\nuse std::net;");
        let result = merge_use_items(&left, &right, Some(&base)).unwrap();
        let (items, _) = result.unwrap();
        let paths = collect_use_paths(&items);
        assert_eq!(paths.len(), 3);
        assert!(paths.contains("std::io"));
        assert!(paths.contains("std::fs"));
        assert!(paths.contains("std::net"));
    }
}
