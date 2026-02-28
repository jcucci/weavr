//! Integration tests for `CSharpMerger::try_merge`.

use std::path::Path;

use weavr_core::{ConflictHunk, HunkContent, HunkContext, HunkId, HunkState, Language};

use crate::{AstMergeResult, AstMerger};

use super::CSharpMerger;

fn make_hunk(left: &str, right: &str, base: Option<&str>) -> ConflictHunk {
    ConflictHunk {
        id: HunkId(1),
        left: HunkContent {
            text: left.to_string(),
        },
        right: HunkContent {
            text: right.to_string(),
        },
        base: base.map(|b| HunkContent {
            text: b.to_string(),
        }),
        context: HunkContext::default(),
        state: HunkState::default(),
    }
}

fn merge(left: &str, right: &str) -> Option<AstMergeResult> {
    let merger = CSharpMerger::new();
    merger.try_merge(&make_hunk(left, right, None)).unwrap()
}

fn merge_three_way(base: &str, left: &str, right: &str) -> Option<AstMergeResult> {
    let merger = CSharpMerger::new();
    merger
        .try_merge(&make_hunk(left, right, Some(base)))
        .unwrap()
}

// --- Using directive tests ---

#[test]
fn using_dedup_identical() {
    let result = merge("using System;", "using System;");
    // Identical usings -- should return None since both sides are the same
    assert!(result.is_none());
}

#[test]
fn using_merge_disjoint() {
    let result = merge("using System;", "using System.IO;").unwrap();
    assert!(result.content.contains("using System;"));
    assert!(result.content.contains("using System.IO;"));
}

#[test]
fn using_merge_overlapping() {
    let result = merge(
        "using System;\nusing System.IO;",
        "using System;\nusing System.Linq;",
    )
    .unwrap();
    assert!(result.content.contains("using System;"));
    assert!(result.content.contains("using System.IO;"));
    assert!(result.content.contains("using System.Linq;"));
}

#[test]
fn using_three_way() {
    let result = merge_three_way(
        "using System;",
        "using System;\nusing System.IO;",
        "using System;\nusing System.Linq;",
    )
    .unwrap();
    assert!(result.content.contains("using System;"));
    assert!(result.content.contains("using System.IO;"));
    assert!(result.content.contains("using System.Linq;"));
}

#[test]
fn using_three_way_deletion_respected() {
    let result = merge_three_way(
        "using System;\nusing System.IO;\nusing System.Linq;",
        "using System;\nusing System.Linq;", // removed IO
        "using System;\nusing System.IO;\nusing System.Linq;", // unchanged
    )
    .unwrap();
    assert!(result.content.contains("using System;"));
    assert!(
        !result.content.contains("System.IO"),
        "IO should be deleted"
    );
    assert!(result.content.contains("using System.Linq;"));
}

#[test]
fn using_deterministic_ordering() {
    let r1 = merge(
        "using System.Linq;\nusing System.IO;",
        "using System.Collections;",
    )
    .unwrap();
    let r2 = merge(
        "using System.IO;\nusing System.Linq;",
        "using System.Collections;",
    )
    .unwrap();
    // Both should produce the same sorted output
    assert_eq!(r1.content, r2.content);
}

#[test]
fn using_static_bail_out() {
    let result = merge("using static System.Math;", "using System.IO;");
    assert!(result.is_none(), "should bail out for static using");
}

#[test]
fn using_alias_bail_out() {
    let result = merge("using Alias = System.IO;", "using System;");
    assert!(result.is_none(), "should bail out for alias using");
}

#[test]
fn using_global_bail_out() {
    let result = merge("global using System;", "using System.IO;");
    assert!(result.is_none(), "should bail out for global using");
}

#[test]
fn using_realistic_system_microsoft() {
    let left = "\
using System;
using System.Collections.Generic;
using Microsoft.Extensions.Logging;
";
    let right = "\
using System;
using System.Linq;
using Microsoft.Extensions.DependencyInjection;
";
    let result = merge(left, right).unwrap();
    assert!(result.content.contains("System.Collections.Generic"));
    assert!(result.content.contains("System.Linq"));
    assert!(result.content.contains("Microsoft.Extensions.Logging"));
    assert!(result
        .content
        .contains("Microsoft.Extensions.DependencyInjection"));
}

// --- Class member tests ---

#[test]
fn class_disjoint_method_additions() {
    let left = "class Foo {\n    public void Alpha() { }\n}";
    let right = "class Foo {\n    public void Beta() { }\n}";
    let result = merge(left, right).unwrap();
    assert!(result.content.contains("Alpha"));
    assert!(result.content.contains("Beta"));
}

#[test]
fn class_identical_method_dedup() {
    let left = "class Foo {\n    public void Alpha() { }\n}";
    let right = "class Foo {\n    public void Alpha() { }\n}";
    let result = merge(left, right);
    if let Some(r) = result {
        assert!(r.content.contains("Alpha"));
    }
}

#[test]
fn class_conflicting_method_fallback() {
    let left = "class Foo {\n    public int Alpha() { return 1; }\n}";
    let right = "class Foo {\n    public int Alpha() { return 999; }\n}";
    let result = merge(left, right);
    assert!(result.is_none(), "conflicting methods should return None");
}

#[test]
fn class_three_way_one_side_modified() {
    let base = "class Foo {\n    public int Alpha() { return 1; }\n}";
    let left = "class Foo {\n    public int Alpha() { return 42; }\n}";
    let right = "class Foo {\n    public int Alpha() { return 1; }\n}";
    let result = merge_three_way(base, left, right).unwrap();
    assert!(result.content.contains("42"));
}

// --- Property tests ---

#[test]
fn property_disjoint_additions() {
    let left = "class Foo {\n    public int X { get; set; }\n}";
    let right = "class Foo {\n    public int Y { get; set; }\n}";
    let result = merge(left, right).unwrap();
    assert!(result.content.contains('X'));
    assert!(result.content.contains('Y'));
}

#[test]
fn property_identical_dedup() {
    let left = "class Foo {\n    public int X { get; set; }\n}";
    let right = "class Foo {\n    public int X { get; set; }\n}";
    let result = merge(left, right);
    if let Some(r) = result {
        assert!(r.content.contains('X'));
    }
}

#[test]
fn property_conflicting_fallback() {
    let left = "class Foo {\n    public int X { get; set; }\n}";
    let right = "class Foo {\n    public string X { get; set; }\n}";
    let result = merge(left, right);
    assert!(
        result.is_none(),
        "conflicting properties should return None"
    );
}

// --- Namespace tests ---

#[test]
fn same_namespace_merge() {
    let left = "namespace MyApp {\n    class Foo { }\n}";
    let right = "namespace MyApp {\n    class Bar { }\n}";
    let result = merge(left, right).unwrap();
    assert!(result.content.contains("Foo"));
    assert!(result.content.contains("Bar"));
}

#[test]
fn different_namespace_disjoint() {
    let left = "namespace Foo { }";
    let right = "namespace Bar { }";
    let result = merge(left, right).unwrap();
    assert!(result.content.contains("Foo"));
    assert!(result.content.contains("Bar"));
}

// --- Edge cases ---

#[test]
fn unparsable_input_returns_none() {
    let result = merge("this is not C# @#$%", "using System;");
    assert!(result.is_none());
}

#[test]
fn empty_left_side() {
    let result = merge("", "using System;").unwrap();
    assert!(result.content.contains("System"));
}

#[test]
fn empty_right_side() {
    let result = merge("using System;", "").unwrap();
    assert!(result.content.contains("System"));
}

#[test]
fn supports_correctness() {
    let merger = CSharpMerger::new();
    assert!(merger.supports(Path::new("Program.cs"), Language::CSharp));
    assert!(!merger.supports(Path::new("main.rs"), Language::Rust));
    assert!(!merger.supports(Path::new("main.go"), Language::Go));
    assert!(!merger.supports(Path::new("main.ts"), Language::TypeScript));
}

#[test]
fn supported_languages_returns_csharp() {
    let merger = CSharpMerger::new();
    let langs = merger.supported_languages();
    assert_eq!(langs, &[Language::CSharp]);
}

#[test]
fn confidence_bounds() {
    // Using merge should be high confidence
    let result = merge("using System;", "using System.IO;").unwrap();
    assert!(result.confidence >= 0.0);
    assert!(result.confidence <= 1.0);
    assert!(
        result.confidence >= 0.9,
        "pure using merge should be high confidence, got {}",
        result.confidence
    );
}

#[test]
fn deterministic_output() {
    let left = "using System.Linq;\nclass Foo { }";
    let right = "using System.IO;\nclass Bar { }";
    let r1 = merge(left, right).unwrap();
    let r2 = merge(left, right).unwrap();
    assert_eq!(r1.content, r2.content);
    assert!((r1.confidence - r2.confidence).abs() < f32::EPSILON);
}

#[test]
fn mixed_using_and_class() {
    let left = "using System;\nclass Foo { }";
    let right = "using System.IO;\nclass Bar { }";
    let result = merge(left, right).unwrap();
    assert!(result.content.contains("System"));
    assert!(result.content.contains("System.IO"));
    assert!(result.content.contains("Foo"));
    assert!(result.content.contains("Bar"));
}

#[test]
fn preprocessor_returns_none() {
    let result = merge("#if DEBUG\nusing System;\n#endif", "using System.IO;");
    assert!(result.is_none());
}

#[test]
fn enum_disjoint() {
    let left = "enum Color { Red, Blue }";
    let right = "enum Shape { Circle, Square }";
    let result = merge(left, right).unwrap();
    assert!(result.content.contains("Color"));
    assert!(result.content.contains("Shape"));
}
