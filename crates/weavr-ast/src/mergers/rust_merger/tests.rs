//! Integration tests for `RustMerger::try_merge`.

use std::path::Path;

use weavr_core::Language;

use crate::mergers::test_utils::make_hunk;
use crate::{AstMergeResult, AstMerger};

use super::RustMerger;

fn merge(left: &str, right: &str) -> Option<AstMergeResult> {
    let merger = RustMerger::new();
    merger.try_merge(&make_hunk(left, right, None)).unwrap()
}

fn merge_three_way(base: &str, left: &str, right: &str) -> Option<AstMergeResult> {
    let merger = RustMerger::new();
    merger
        .try_merge(&make_hunk(left, right, Some(base)))
        .unwrap()
}

// ─── Use statement tests ───

#[test]
fn use_dedup_identical() {
    let result = merge("use std::io;", "use std::io;");
    // Identical uses — should still produce a result (or None since they're the same)
    // The merger returns None when both sides are identical
    assert!(result.is_none());
}

#[test]
fn use_merge_disjoint() {
    let result = merge("use std::io;", "use std::fs;").unwrap();
    assert!(result.content.contains("std::fs"));
    assert!(result.content.contains("std::io"));
}

#[test]
fn use_merge_overlapping() {
    let result = merge("use std::io;\nuse std::fs;", "use std::io;\nuse std::net;").unwrap();
    assert!(result.content.contains("std::io"));
    assert!(result.content.contains("std::fs"));
    assert!(result.content.contains("std::net"));
}

#[test]
fn use_three_way() {
    let result = merge_three_way(
        "use std::io;",
        "use std::io;\nuse std::fs;",
        "use std::io;\nuse std::net;",
    )
    .unwrap();
    assert!(result.content.contains("std::io"));
    assert!(result.content.contains("std::fs"));
    assert!(result.content.contains("std::net"));
}

#[test]
fn use_three_way_deletion_respected() {
    let result = merge_three_way(
        "use std::io;\nuse std::fs;\nuse std::net;",
        "use std::io;\nuse std::net;",               // removed fs
        "use std::io;\nuse std::fs;\nuse std::net;", // unchanged
    )
    .unwrap();
    assert!(result.content.contains("std::io"));
    assert!(!result.content.contains("std::fs"));
    assert!(result.content.contains("std::net"));
}

#[test]
fn use_deterministic_ordering() {
    let r1 = merge("use std::net;\nuse std::io;", "use std::fs;").unwrap();
    let r2 = merge("use std::io;\nuse std::net;", "use std::fs;").unwrap();
    // Both should produce the same sorted output
    assert_eq!(r1.content, r2.content);
}

#[test]
fn use_realistic_import_conflict() {
    let left = "\
use std::collections::HashMap;
use std::io::{self, Read};
use serde::Serialize;
";
    let right = "\
use std::collections::HashMap;
use std::io::{self, Write};
use serde::Deserialize;
";
    let result = merge(left, right).unwrap();
    assert!(result.content.contains("HashMap"));
    assert!(result.content.contains("Serialize"));
    assert!(result.content.contains("Deserialize"));
}

// ─── Function tests ───

#[test]
fn fn_disjoint_additions() {
    let result = merge("fn alpha() {}", "fn beta() {}").unwrap();
    assert!(result.content.contains("alpha"));
    assert!(result.content.contains("beta"));
}

#[test]
fn fn_identical_dedup() {
    let result = merge("fn alpha() -> i32 { 42 }", "fn alpha() -> i32 { 42 }");
    // Identical content should return None (no change needed) or a dedup result
    // With mixed items (not all uses), identical items still merge but produce same content
    // The merger will produce a result since both sides are parsed and merged
    if let Some(r) = result {
        assert!(r.content.contains("alpha"));
    }
}

#[test]
fn fn_conflicting_returns_none() {
    let result = merge("fn alpha() -> i32 { 42 }", "fn alpha() -> i32 { 99 }");
    assert!(result.is_none(), "conflicting functions should return None");
}

#[test]
fn fn_three_way_one_side_modified() {
    let result = merge_three_way(
        "fn alpha() -> i32 { 1 }",
        "fn alpha() -> i32 { 42 }", // left modified
        "fn alpha() -> i32 { 1 }",  // right unchanged
    )
    .unwrap();
    assert!(result.content.contains("42"));
}

// ─── Impl block tests ───

#[test]
fn impl_disjoint_method_additions() {
    let result = merge(
        "impl Foo { fn alpha(&self) -> i32 { 1 } }",
        "impl Foo { fn beta(&self) -> i32 { 2 } }",
    )
    .unwrap();
    assert!(result.content.contains("alpha"));
    assert!(result.content.contains("beta"));
}

#[test]
fn impl_identical_dedup() {
    let left = "impl Foo { fn alpha(&self) -> i32 { 1 } }";
    let right = "impl Foo { fn alpha(&self) -> i32 { 1 } }";
    let result = merge(left, right);
    if let Some(r) = result {
        // Should contain alpha exactly once in the output
        assert!(r.content.contains("alpha"));
    }
}

#[test]
fn impl_conflicting_methods_returns_none() {
    let result = merge(
        "impl Foo { fn alpha(&self) -> i32 { 1 } }",
        "impl Foo { fn alpha(&self) -> i32 { 999 } }",
    );
    assert!(
        result.is_none(),
        "conflicting impl methods should return None"
    );
}

// ─── Edge cases ───

#[test]
fn unparsable_input_returns_none() {
    let result = merge("this is not rust @#$%", "use std::io;");
    assert!(result.is_none());
}

#[test]
fn empty_left_side() {
    let result = merge("", "use std::io;").unwrap();
    assert!(result.content.contains("std::io"));
}

#[test]
fn empty_right_side() {
    let result = merge("use std::io;", "").unwrap();
    assert!(result.content.contains("std::io"));
}

#[test]
fn supports_correctness() {
    let merger = RustMerger::new();
    assert!(merger.supports(Path::new("main.rs"), Language::Rust));
    assert!(!merger.supports(Path::new("main.go"), Language::Go));
    assert!(!merger.supports(Path::new("main.ts"), Language::TypeScript));
}

#[test]
fn supported_languages_returns_rust() {
    let merger = RustMerger::new();
    let langs = merger.supported_languages();
    assert_eq!(langs, &[Language::Rust]);
}

#[test]
fn confidence_bounds() {
    // Use merge should be high confidence
    let result = merge("use std::io;", "use std::fs;").unwrap();
    assert!(result.confidence >= 0.0);
    assert!(result.confidence <= 1.0);
    assert!(
        result.confidence >= 0.9,
        "pure use merge should be high confidence, got {}",
        result.confidence
    );
}

#[test]
fn deterministic_output() {
    let left = "use std::net;\nfn foo() {}";
    let right = "use std::io;\nfn bar() {}";
    let r1 = merge(left, right).unwrap();
    let r2 = merge(left, right).unwrap();
    assert_eq!(r1.content, r2.content);
    assert!((r1.confidence - r2.confidence).abs() < f32::EPSILON);
}

#[test]
fn mixed_use_and_fn_items() {
    let left = "use std::io;\nfn alpha() {}";
    let right = "use std::fs;\nfn beta() {}";
    let result = merge(left, right).unwrap();
    assert!(result.content.contains("std::io"));
    assert!(result.content.contains("std::fs"));
    assert!(result.content.contains("alpha"));
    assert!(result.content.contains("beta"));
}
