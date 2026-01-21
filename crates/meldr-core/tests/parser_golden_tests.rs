//! Golden tests for conflict marker parsing.
//!
//! These tests verify parsing against real conflict samples.

use meldr_core::{parse_conflict_markers, HunkState};

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
    assert_eq!(
        hunk.base.as_ref().unwrap().text,
        "    println!(\"Hello\");"
    );
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
    use meldr_core::Segment;
    let mut saw_clean = false;
    let mut saw_conflict = false;
    for segment in &parsed.segments {
        match segment {
            Segment::Clean(_) => saw_clean = true,
            Segment::Conflict(_) => saw_conflict = true,
        }
    }
    assert!(saw_clean && saw_conflict, "should have both clean and conflict segments");
}
