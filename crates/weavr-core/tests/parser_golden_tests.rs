//! Golden tests for conflict marker parsing.
//!
//! These tests verify parsing against real conflict samples.

use weavr_core::{parse_conflict_markers, ConflictFormat, HunkState, Segment};

#[test]
fn golden_simple_two_way() {
    let input = include_str!("golden/simple_two_way.conflict");
    let parsed = parse_conflict_markers(input).expect("should parse simple two-way conflict");

    assert_eq!(parsed.hunks.len(), 1, "should have exactly one hunk");

    let hunk = &parsed.hunks[0];
    assert_eq!(hunk.left.text, "    println!(\"Hello from HEAD\");");
    assert_eq!(hunk.right.text, "    println!(\"Hello from feature\");");
    assert!(hunk.base.is_none(), "two-way conflict should have no base");
    assert_eq!(hunk.state, HunkState::Unresolved);

    // Context should include surrounding lines
    assert_eq!(hunk.context.before.len(), 1);
    assert_eq!(hunk.context.before[0], "fn main() {");
    assert_eq!(hunk.context.after.len(), 1);
    assert_eq!(hunk.context.after[0], "}");
}

#[test]
fn golden_diff3_three_way() {
    let input = include_str!("golden/diff3_three_way.conflict");
    let parsed = parse_conflict_markers(input).expect("should parse diff3 conflict");

    assert_eq!(parsed.hunks.len(), 1, "should have exactly one hunk");

    let hunk = &parsed.hunks[0];
    assert_eq!(hunk.left.text, "    println!(\"Hello from HEAD\");");
    assert_eq!(hunk.right.text, "    println!(\"Hello from feature\");");
    assert!(hunk.base.is_some(), "diff3 conflict should have base");
    assert_eq!(hunk.base.as_ref().unwrap().text, "    println!(\"Hello\");");
}

#[test]
fn golden_multi_hunk() {
    let input = include_str!("golden/multi_hunk.conflict");
    let parsed = parse_conflict_markers(input).expect("should parse multi-hunk conflict");

    assert_eq!(parsed.hunks.len(), 2, "should have exactly two hunks");

    // First hunk (foo function)
    let hunk1 = &parsed.hunks[0];
    assert!(hunk1.left.text.contains("foo from HEAD"));
    assert!(hunk1.right.text.contains("foo from feature"));

    // Second hunk (baz function)
    let hunk2 = &parsed.hunks[1];
    assert!(hunk2.left.text.contains("baz from HEAD"));
    assert!(hunk2.right.text.contains("baz from feature"));

    // Verify sequential IDs
    assert_eq!(hunk1.id.0, 0);
    assert_eq!(hunk2.id.0, 1);
}

#[test]
fn golden_edge_cases() {
    let input = include_str!("golden/edge_cases.conflict");
    let parsed = parse_conflict_markers(input).expect("should parse edge case conflicts");

    assert_eq!(parsed.hunks.len(), 3, "should have exactly three hunks");

    // First hunk: conflict at file start
    let hunk1 = &parsed.hunks[0];
    assert!(
        hunk1.context.before.is_empty(),
        "conflict at start should have no before context"
    );

    // Second hunk: empty left side
    let hunk2 = &parsed.hunks[1];
    assert!(hunk2.left.text.is_empty(), "left side should be empty");
    assert_eq!(hunk2.right.text, "empty left side");

    // Third hunk: empty right side
    let hunk3 = &parsed.hunks[2];
    assert_eq!(hunk3.left.text, "empty right side");
    assert!(hunk3.right.text.is_empty(), "right side should be empty");
}

#[test]
fn golden_segments_structure() {
    let input = include_str!("golden/multi_hunk.conflict");
    let parsed = parse_conflict_markers(input).expect("should parse");

    // Should have: Clean, Conflict, Clean, Conflict, Clean
    // Or similar structure preserving file order
    assert!(!parsed.segments.is_empty());

    // Verify segments alternate between clean and conflict
    let mut saw_clean = false;
    let mut saw_conflict = false;
    for segment in &parsed.segments {
        match segment {
            Segment::Clean(_) => saw_clean = true,
            Segment::Conflict(_) => saw_conflict = true,
        }
    }
    assert!(
        saw_clean && saw_conflict,
        "should have both clean and conflict segments"
    );
}

// --- jj Snapshot Golden Tests ---

#[test]
fn golden_jj_snapshot_basic() {
    let input = include_str!("golden/jj_snapshot_basic.conflict");
    let parsed = parse_conflict_markers(input).expect("should parse jj snapshot conflict");

    assert_eq!(parsed.hunks.len(), 1, "should have exactly one hunk");
    assert_eq!(parsed.format, Some(ConflictFormat::JjSnapshot));

    let hunk = &parsed.hunks[0];
    assert_eq!(hunk.left.text, "    println!(\"Hello from side 1\");");
    assert_eq!(hunk.right.text, "    println!(\"Hello from side 2\");");
    assert!(hunk.base.is_some(), "should have base");
    assert_eq!(hunk.base.as_ref().unwrap().text, "    println!(\"Hello\");");
    assert_eq!(hunk.state, HunkState::Unresolved);

    // Context
    assert_eq!(hunk.context.before.len(), 1);
    assert_eq!(hunk.context.before[0], "fn main() {");
    assert_eq!(hunk.context.after.len(), 1);
    assert_eq!(hunk.context.after[0], "}");
}

#[test]
fn golden_jj_snapshot_multi_hunk() {
    let input = include_str!("golden/jj_snapshot_multi_hunk.conflict");
    let parsed = parse_conflict_markers(input).expect("should parse jj multi-hunk conflict");

    assert_eq!(parsed.hunks.len(), 2, "should have exactly two hunks");
    assert_eq!(parsed.format, Some(ConflictFormat::JjSnapshot));

    let hunk1 = &parsed.hunks[0];
    assert!(hunk1.left.text.contains("foo from side 1"));
    assert!(hunk1.right.text.contains("foo from side 2"));
    assert!(hunk1.base.is_some());

    let hunk2 = &parsed.hunks[1];
    assert!(hunk2.left.text.contains("baz from side 1"));
    assert!(hunk2.right.text.contains("baz from side 2"));
    assert!(hunk2.base.is_some());

    assert_eq!(hunk1.id.0, 0);
    assert_eq!(hunk2.id.0, 1);

    // Verify segments structure
    let mut saw_clean = false;
    let mut saw_conflict = false;
    for segment in &parsed.segments {
        match segment {
            Segment::Clean(_) => saw_clean = true,
            Segment::Conflict(_) => saw_conflict = true,
        }
    }
    assert!(saw_clean && saw_conflict);
}

// --- jj Diff Golden Tests ---

#[test]
fn golden_jj_diff_mixed() {
    let input = include_str!("golden/jj_diff_mixed.conflict");
    let parsed = parse_conflict_markers(input).expect("should parse jj diff mixed conflict");

    assert_eq!(parsed.hunks.len(), 1, "should have exactly one hunk");
    assert_eq!(parsed.format, Some(ConflictFormat::JjDiff));

    let hunk = &parsed.hunks[0];
    assert_eq!(hunk.left.text, "    println!(\"Hello from side 1\");");
    assert_eq!(
        hunk.right.text,
        "    println!(\"Hello\");\n    println!(\"Hello from side 2\");"
    );
    assert!(
        hunk.base.is_some(),
        "should have base from diff reconstruction"
    );
    assert_eq!(
        hunk.base.as_ref().unwrap().text,
        "    println!(\"Hello\");\n    println!(\"Hello\");"
    );
    assert_eq!(hunk.state, HunkState::Unresolved);

    // Context
    assert_eq!(hunk.context.before.len(), 1);
    assert_eq!(hunk.context.before[0], "fn main() {");
    assert_eq!(hunk.context.after.len(), 1);
    assert_eq!(hunk.context.after[0], "}");
}

#[test]
fn golden_jj_diff_pure() {
    let input = include_str!("golden/jj_diff_pure.conflict");
    let parsed = parse_conflict_markers(input).expect("should parse jj diff pure conflict");

    assert_eq!(parsed.hunks.len(), 1, "should have exactly one hunk");
    assert_eq!(parsed.format, Some(ConflictFormat::JjDiff));

    let hunk = &parsed.hunks[0];
    assert_eq!(
        hunk.left.text,
        "    println!(\"Hello\");\n    println!(\"Hello from side 1\");"
    );
    assert_eq!(
        hunk.right.text,
        "    println!(\"Hello\");\n    println!(\"Hello from side 2\");"
    );
    assert!(hunk.base.is_some(), "pure diff should reconstruct base");
    assert_eq!(
        hunk.base.as_ref().unwrap().text,
        "    println!(\"Hello\");\n    println!(\"Hello\");"
    );
}

#[test]
fn golden_jj_diff_multi_hunk() {
    let input = include_str!("golden/jj_diff_multi_hunk.conflict");
    let parsed = parse_conflict_markers(input).expect("should parse jj diff multi-hunk conflict");

    assert_eq!(parsed.hunks.len(), 2, "should have exactly two hunks");
    assert_eq!(parsed.format, Some(ConflictFormat::JjDiff));

    let hunk1 = &parsed.hunks[0];
    assert!(hunk1.left.text.contains("foo from side 1"));
    assert!(hunk1.right.text.contains("foo from side 2"));
    assert!(hunk1.base.is_some());

    let hunk2 = &parsed.hunks[1];
    assert!(hunk2.left.text.contains("baz from side 1"));
    assert!(hunk2.right.text.contains("baz from side 2"));
    assert!(hunk2.base.is_some());

    assert_eq!(hunk1.id.0, 0);
    assert_eq!(hunk2.id.0, 1);

    // Verify segments structure
    let mut saw_clean = false;
    let mut saw_conflict = false;
    for segment in &parsed.segments {
        match segment {
            Segment::Clean(_) => saw_clean = true,
            Segment::Conflict(_) => saw_conflict = true,
        }
    }
    assert!(saw_clean && saw_conflict);
}
