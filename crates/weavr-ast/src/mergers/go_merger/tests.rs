//! Integration tests for `GoMerger::try_merge`.

use std::path::Path;

use weavr_core::Language;

use crate::mergers::test_utils::make_hunk;
use crate::{AstMergeResult, AstMerger};

use super::GoMerger;

fn merge(left: &str, right: &str) -> Option<AstMergeResult> {
    let merger = GoMerger::new();
    merger.try_merge(&make_hunk(left, right, None)).unwrap()
}

fn merge_three_way(base: &str, left: &str, right: &str) -> Option<AstMergeResult> {
    let merger = GoMerger::new();
    merger
        .try_merge(&make_hunk(left, right, Some(base)))
        .unwrap()
}

// --- Import tests ---

#[test]
fn import_dedup_identical() {
    let result = merge("import \"fmt\"", "import \"fmt\"");
    // Identical imports -- should return None since both sides are the same
    assert!(result.is_none());
}

#[test]
fn import_merge_disjoint() {
    let result = merge("import \"fmt\"", "import \"os\"").unwrap();
    assert!(result.content.contains("\"fmt\""));
    assert!(result.content.contains("\"os\""));
}

#[test]
fn import_merge_overlapping() {
    let left = "import (\n\t\"fmt\"\n\t\"os\"\n)";
    let right = "import (\n\t\"fmt\"\n\t\"io\"\n)";
    let result = merge(left, right).unwrap();
    assert!(result.content.contains("\"fmt\""));
    assert!(result.content.contains("\"os\""));
    assert!(result.content.contains("\"io\""));
}

#[test]
fn import_three_way() {
    let base = "import \"fmt\"";
    let left = "import (\n\t\"fmt\"\n\t\"os\"\n)";
    let right = "import (\n\t\"fmt\"\n\t\"io\"\n)";
    let result = merge_three_way(base, left, right).unwrap();
    assert!(result.content.contains("\"fmt\""));
    assert!(result.content.contains("\"os\""));
    assert!(result.content.contains("\"io\""));
}

#[test]
fn import_three_way_deletion_respected() {
    let base = "import (\n\t\"fmt\"\n\t\"os\"\n\t\"io\"\n)";
    let left = "import (\n\t\"fmt\"\n\t\"io\"\n)"; // removed os
    let right = "import (\n\t\"fmt\"\n\t\"os\"\n\t\"io\"\n)"; // unchanged
    let result = merge_three_way(base, left, right).unwrap();
    assert!(result.content.contains("\"fmt\""));
    assert!(!result.content.contains("\"os\""), "os should be deleted");
    assert!(result.content.contains("\"io\""));
}

#[test]
fn import_stdlib_external_grouping() {
    let left = "import (\n\t\"fmt\"\n\t\"github.com/pkg/errors\"\n)";
    let right = "import (\n\t\"os\"\n\t\"github.com/stretchr/testify\"\n)";
    let result = merge(left, right).unwrap();

    // Verify all imports present
    assert!(result.content.contains("\"fmt\""));
    assert!(result.content.contains("\"os\""));
    assert!(result.content.contains("\"github.com/pkg/errors\""));
    assert!(result.content.contains("\"github.com/stretchr/testify\""));

    // Verify grouping: stdlib comes before external
    let fmt_pos = result.content.find("\"fmt\"").unwrap();
    let errors_pos = result.content.find("\"github.com/pkg/errors\"").unwrap();
    assert!(
        fmt_pos < errors_pos,
        "stdlib should come before external imports"
    );
}

#[test]
fn import_named_preserved() {
    let left = "import f \"fmt\"";
    let right = "import \"os\"";
    let result = merge(left, right).unwrap();
    assert!(result.content.contains("f \"fmt\""));
    assert!(result.content.contains("\"os\""));
}

#[test]
fn import_blank_preserved() {
    let left = "import _ \"database/sql\"";
    let right = "import \"fmt\"";
    let result = merge(left, right).unwrap();
    assert!(result.content.contains("_ \"database/sql\""));
    assert!(result.content.contains("\"fmt\""));
}

#[test]
fn import_dot_causes_bailout() {
    let result = merge("import . \"fmt\"", "import \"os\"");
    assert!(result.is_none(), "dot imports should bail out");
}

#[test]
fn import_deterministic_ordering() {
    let r1 = merge("import (\n\t\"os\"\n\t\"fmt\"\n)", "import \"io\"").unwrap();
    let r2 = merge("import (\n\t\"fmt\"\n\t\"os\"\n)", "import \"io\"").unwrap();
    // Both should produce the same sorted output
    assert_eq!(r1.content, r2.content);
}

// --- Function tests ---

#[test]
fn function_disjoint_merge() {
    let left = "func Alpha() {\n\treturn\n}";
    let right = "func Beta() {\n\treturn\n}";
    let result = merge(left, right).unwrap();
    assert!(result.content.contains("Alpha"));
    assert!(result.content.contains("Beta"));
}

#[test]
fn function_identical_dedup() {
    let left = "func Alpha() {}";
    let right = "func Alpha() {}";
    let result = merge(left, right);
    if let Some(r) = result {
        assert!(r.content.contains("Alpha"));
    }
}

#[test]
fn function_conflicting_body_fallback() {
    let left = "func Alpha() int { return 1 }";
    let right = "func Alpha() int { return 999 }";
    let result = merge(left, right);
    assert!(result.is_none(), "conflicting functions should return None");
}

#[test]
fn function_three_way_one_side_modified() {
    let base = "func Alpha() int { return 1 }";
    let left = "func Alpha() int { return 42 }";
    let right = "func Alpha() int { return 1 }";
    let result = merge_three_way(base, left, right).unwrap();
    assert!(result.content.contains("42"));
}

// --- Struct field merge tests ---

#[test]
fn struct_disjoint_field_additions() {
    let left = "type Config struct {\n\tName string\n\tHost string\n}";
    let right = "type Config struct {\n\tName string\n\tPort int\n}";
    let result = merge(left, right).unwrap();
    assert!(result.content.contains("Host"));
    assert!(result.content.contains("Port"));
    assert!(result.content.contains("Name"));
}

#[test]
fn struct_identical_dedup() {
    let left = "type Config struct {\n\tName string\n}";
    let right = "type Config struct {\n\tName string\n}";
    let result = merge(left, right);
    if let Some(r) = result {
        assert!(r.content.contains("Name"));
    }
}

#[test]
fn struct_conflicting_field_type_fallback() {
    let left = "type Config struct {\n\tPort int\n}";
    let right = "type Config struct {\n\tPort string\n}";
    let result = merge(left, right);
    assert!(
        result.is_none(),
        "conflicting field types should return None"
    );
}

// --- Interface method merge tests ---

#[test]
fn interface_disjoint_method_additions() {
    let left = "type Handler interface {\n\tServeHTTP()\n\tInit()\n}";
    let right = "type Handler interface {\n\tServeHTTP()\n\tClose()\n}";
    let result = merge(left, right).unwrap();
    assert!(result.content.contains("Init"));
    assert!(result.content.contains("Close"));
    assert!(result.content.contains("ServeHTTP"));
}

// --- Method tests ---

#[test]
fn method_identity_includes_receiver() {
    let left = "func (s *MyStruct) Foo() {}";
    let right = "func (s *OtherStruct) Foo() {}";
    let result = merge(left, right).unwrap();
    assert!(result.content.contains("MyStruct"));
    assert!(result.content.contains("OtherStruct"));
}

// --- Bailout tests ---

#[test]
fn build_directive_returns_none() {
    let result = merge("//go:build linux\n\nimport \"fmt\"", "import \"os\"");
    assert!(result.is_none());
}

#[test]
fn legacy_build_tag_returns_none() {
    let result = merge("// +build linux\n\nimport \"fmt\"", "import \"os\"");
    assert!(result.is_none());
}

#[test]
fn cgo_import_returns_none() {
    let result = merge("import \"C\"", "import \"fmt\"");
    assert!(result.is_none());
}

#[test]
fn cgo_import_with_comment_returns_none() {
    let result = merge("import \"C\" // required for cgo", "import \"fmt\"");
    assert!(result.is_none());
}

#[test]
fn unparsable_input_returns_none() {
    let result = merge("this is not Go @#$%", "import \"fmt\"");
    assert!(result.is_none());
}

// --- Edge cases ---

#[test]
fn empty_left_side() {
    let result = merge("", "import \"fmt\"").unwrap();
    assert!(result.content.contains("fmt"));
}

#[test]
fn empty_right_side() {
    let result = merge("import \"fmt\"", "").unwrap();
    assert!(result.content.contains("fmt"));
}

#[test]
fn supports_correctness() {
    let merger = GoMerger::new();
    assert!(merger.supports(Path::new("main.go"), Language::Go));
    assert!(!merger.supports(Path::new("main.rs"), Language::Rust));
    assert!(!merger.supports(Path::new("Program.cs"), Language::CSharp));
    assert!(!merger.supports(Path::new("main.ts"), Language::TypeScript));
}

#[test]
fn supported_languages_returns_go() {
    let merger = GoMerger::new();
    let langs = merger.supported_languages();
    assert_eq!(langs, &[Language::Go]);
}

#[test]
fn confidence_bounds() {
    // Import merge should be high confidence
    let result = merge("import \"fmt\"", "import \"os\"").unwrap();
    assert!(result.confidence >= 0.0);
    assert!(result.confidence <= 1.0);
    assert!(
        result.confidence >= 0.9,
        "pure import merge should be high confidence, got {}",
        result.confidence
    );
}

#[test]
fn deterministic_output() {
    let left = "import \"fmt\"\n\nfunc Foo() {}";
    let right = "import \"os\"\n\nfunc Bar() {}";
    let r1 = merge(left, right).unwrap();
    let r2 = merge(left, right).unwrap();
    assert_eq!(r1.content, r2.content);
    assert!((r1.confidence - r2.confidence).abs() < f32::EPSILON);
}

#[test]
fn mixed_import_and_function() {
    let left = "import \"fmt\"\n\nfunc Foo() {}";
    let right = "import \"os\"\n\nfunc Bar() {}";
    let result = merge(left, right).unwrap();
    assert!(result.content.contains("\"fmt\""));
    assert!(result.content.contains("\"os\""));
    assert!(result.content.contains("Foo"));
    assert!(result.content.contains("Bar"));
}

#[test]
fn const_disjoint() {
    let left = "const MaxRetries = 3";
    let right = "const Timeout = 30";
    let result = merge(left, right).unwrap();
    assert!(result.content.contains("MaxRetries"));
    assert!(result.content.contains("Timeout"));
}

#[test]
fn var_disjoint() {
    let left = "var ErrNotFound = errors.New(\"not found\")";
    let right = "var ErrTimeout = errors.New(\"timeout\")";
    let result = merge(left, right).unwrap();
    assert!(result.content.contains("ErrNotFound"));
    assert!(result.content.contains("ErrTimeout"));
}
