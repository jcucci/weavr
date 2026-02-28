//! Integration tests for `TypeScriptMerger::try_merge`.

use std::path::Path;

use weavr_core::Language;

use crate::mergers::test_utils::make_hunk;
use crate::{AstMergeResult, AstMerger};

use super::TypeScriptMerger;

fn merge(left: &str, right: &str) -> Option<AstMergeResult> {
    let merger = TypeScriptMerger::new();
    merger.try_merge(&make_hunk(left, right, None)).unwrap()
}

fn merge_three_way(base: &str, left: &str, right: &str) -> Option<AstMergeResult> {
    let merger = TypeScriptMerger::new();
    merger
        .try_merge(&make_hunk(left, right, Some(base)))
        .unwrap()
}

// --- Import merge tests ---

#[test]
fn import_disjoint_named_from_same_module() {
    let result = merge(
        "import { useState } from 'react';",
        "import { useEffect } from 'react';",
    )
    .unwrap();
    assert!(result.content.contains("useState"));
    assert!(result.content.contains("useEffect"));
    assert!(result.content.contains("react"));
}

#[test]
fn import_overlapping_named_dedup() {
    let result = merge(
        "import { useState, useEffect } from 'react';",
        "import { useState, useMemo } from 'react';",
    )
    .unwrap();
    assert!(result.content.contains("useState"));
    assert!(result.content.contains("useEffect"));
    assert!(result.content.contains("useMemo"));
    // Should only appear once
    let count = result.content.matches("useState").count();
    assert_eq!(count, 1, "useState should appear exactly once");
}

#[test]
fn import_different_modules_preserved() {
    let result = merge(
        "import { useState } from 'react';",
        "import { render } from 'react-dom';",
    )
    .unwrap();
    assert!(result.content.contains("react"));
    assert!(result.content.contains("react-dom"));
    assert!(result.content.contains("useState"));
    assert!(result.content.contains("render"));
}

#[test]
fn import_type_only_separate_from_value() {
    let result = merge(
        "import { useState } from 'react';",
        "import type { FC } from 'react';",
    )
    .unwrap();
    // Both should be present as separate imports
    assert!(result.content.contains("useState"));
    assert!(result.content.contains("FC"));
    assert!(result.content.contains("import type"));
}

#[test]
fn import_identical_returns_none() {
    let result = merge(
        "import { useState } from 'react';",
        "import { useState } from 'react';",
    );
    assert!(result.is_none(), "identical imports should return None");
}

#[test]
fn import_three_way_additions_from_both() {
    let result = merge_three_way(
        "import { useState } from 'react';",
        "import { useState, useEffect } from 'react';",
        "import { useState, useMemo } from 'react';",
    )
    .unwrap();
    assert!(result.content.contains("useState"));
    assert!(result.content.contains("useEffect"));
    assert!(result.content.contains("useMemo"));
}

#[test]
fn import_three_way_deletion_respected() {
    let result = merge_three_way(
        "import { useState, useEffect } from 'react';",
        "import { useState } from 'react';", // removed useEffect
        "import { useState, useEffect } from 'react';", // unchanged
    )
    .unwrap();
    assert!(result.content.contains("useState"));
    assert!(
        !result.content.contains("useEffect"),
        "useEffect should be deleted"
    );
}

#[test]
fn import_default_named_combo_bail_out() {
    let result = merge(
        "import React, { useState } from 'react';",
        "import React, { useEffect } from 'react';",
    );
    assert!(result.is_none(), "default+named combo should bail out");
}

#[test]
fn import_namespace_preserved() {
    let left = "import * as React from 'react';";
    let right = "import { useState } from 'react';";
    let result = merge(left, right).unwrap();
    // Both should be present (namespace and named are separate identities)
    assert!(result.content.contains("useState"));
}

#[test]
fn import_side_effect_preserved() {
    let result = merge("import './polyfill';", "import { useState } from 'react';").unwrap();
    assert!(result.content.contains("polyfill"));
    assert!(result.content.contains("useState"));
}

// --- Non-import declaration tests ---

#[test]
fn function_disjoint() {
    let result = merge(
        "function foo() { return 1; }",
        "function bar() { return 2; }",
    )
    .unwrap();
    assert!(result.content.contains("foo"));
    assert!(result.content.contains("bar"));
}

#[test]
fn function_identical_dedup() {
    let result = merge(
        "function foo() { return 1; }",
        "function foo() { return 1; }",
    );
    if let Some(r) = result {
        assert!(r.content.contains("foo"));
        let count = r.content.matches("function foo").count();
        assert_eq!(count, 1);
    }
}

#[test]
fn function_conflicting_fallback() {
    let result = merge(
        "function foo() { return 1; }",
        "function foo() { return 999; }",
    );
    assert!(result.is_none(), "conflicting functions should return None");
}

#[test]
fn class_disjoint() {
    let result = merge("class Foo {}", "class Bar {}").unwrap();
    assert!(result.content.contains("Foo"));
    assert!(result.content.contains("Bar"));
}

#[test]
fn interface_disjoint() {
    let result = merge(
        "interface IFoo { x: number; }",
        "interface IBar { y: string; }",
    )
    .unwrap();
    assert!(result.content.contains("IFoo"));
    assert!(result.content.contains("IBar"));
}

#[test]
fn three_way_one_side_modified_function() {
    let result = merge_three_way(
        "function foo() { return 1; }",
        "function foo() { return 42; }",
        "function foo() { return 1; }",
    )
    .unwrap();
    assert!(result.content.contains("42"));
}

// --- Mixed imports + declarations ---

#[test]
fn mixed_imports_and_functions() {
    let result = merge(
        "import { A } from 'x';\nfunction foo() {}",
        "import { B } from 'x';\nfunction bar() {}",
    )
    .unwrap();
    assert!(result.content.contains('A'));
    assert!(result.content.contains('B'));
    assert!(result.content.contains("foo"));
    assert!(result.content.contains("bar"));
}

// --- Edge cases ---

#[test]
fn unparsable_input_returns_none() {
    let result = merge("@#$%^&*()!!! not valid", "import { A } from 'x';");
    assert!(result.is_none());
}

#[test]
fn empty_left_side() {
    let result = merge("", "import { useState } from 'react';").unwrap();
    assert!(result.content.contains("useState"));
}

#[test]
fn empty_right_side() {
    let result = merge("import { useState } from 'react';", "").unwrap();
    assert!(result.content.contains("useState"));
}

#[test]
fn supports_correctness() {
    let merger = TypeScriptMerger::new();
    assert!(merger.supports(Path::new("index.ts"), Language::TypeScript));
    assert!(merger.supports(Path::new("App.tsx"), Language::TypeScript));
    assert!(!merger.supports(Path::new("main.rs"), Language::Rust));
    assert!(!merger.supports(Path::new("Program.cs"), Language::CSharp));
    assert!(!merger.supports(Path::new("main.go"), Language::Go));
}

#[test]
fn supported_languages_returns_typescript() {
    let merger = TypeScriptMerger::new();
    let langs = merger.supported_languages();
    assert_eq!(langs, &[Language::TypeScript]);
}

#[test]
fn confidence_bounds() {
    let result = merge(
        "import { useState } from 'react';",
        "import { useEffect } from 'react';",
    )
    .unwrap();
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
    let left = "import { useState } from 'react';\nfunction foo() {}";
    let right = "import { useEffect } from 'react';\nfunction bar() {}";
    let r1 = merge(left, right).unwrap();
    let r2 = merge(left, right).unwrap();
    assert_eq!(r1.content, r2.content);
    assert!((r1.confidence - r2.confidence).abs() < f32::EPSILON);
}

#[test]
fn triple_slash_reference_returns_none() {
    let result = merge(
        "/// <reference path=\"types.d.ts\" />\nimport { Foo } from './foo';",
        "import { Bar } from './bar';",
    );
    assert!(result.is_none());
}

#[test]
fn import_realistic_react_hooks() {
    let left = "\
import { useState, useCallback } from 'react';
import type { FC, ReactNode } from 'react';
";
    let right = "\
import { useState, useMemo } from 'react';
import type { FC, PropsWithChildren } from 'react';
";
    let result = merge(left, right).unwrap();
    assert!(result.content.contains("useState"));
    assert!(result.content.contains("useCallback"));
    assert!(result.content.contains("useMemo"));
    assert!(result.content.contains("FC"));
    assert!(result.content.contains("ReactNode"));
    assert!(result.content.contains("PropsWithChildren"));
}

#[test]
fn import_deterministic_ordering() {
    let r1 = merge(
        "import { useEffect, useState } from 'react';",
        "import { useMemo } from 'react';",
    )
    .unwrap();
    let r2 = merge(
        "import { useState, useEffect } from 'react';",
        "import { useMemo } from 'react';",
    )
    .unwrap();
    // Both should produce the same sorted output
    assert_eq!(r1.content, r2.content);
}

#[test]
fn enum_disjoint() {
    let result = merge("enum Color { Red, Blue }", "enum Shape { Circle, Square }").unwrap();
    assert!(result.content.contains("Color"));
    assert!(result.content.contains("Shape"));
}

#[test]
fn type_alias_disjoint() {
    let result = merge("type ID = string;", "type Name = string;").unwrap();
    assert!(result.content.contains("ID"));
    assert!(result.content.contains("Name"));
}
