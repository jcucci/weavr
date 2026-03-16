//! Integration tests for the `merge-driver` subcommand.

use std::io::Write;

use assert_cmd::Command;
use tempfile::NamedTempFile;

fn weavr_cmd() -> Command {
    Command::cargo_bin("weavr").unwrap()
}

fn temp_file(content: &str) -> NamedTempFile {
    let mut f = NamedTempFile::new().unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f.flush().unwrap();
    f
}

#[test]
fn clean_merge_exits_0() {
    let base = temp_file("line1\ncommon\nline3\n");
    let ours = temp_file("line1\ncommon\nline3\n");
    let theirs = temp_file("line1\ncommon\nline3-changed\n");

    weavr_cmd()
        .args([
            "merge-driver",
            base.path().to_str().unwrap(),
            ours.path().to_str().unwrap(),
            theirs.path().to_str().unwrap(),
        ])
        .assert()
        .success();

    let result = std::fs::read_to_string(ours.path()).unwrap();
    assert!(result.contains("line3-changed"));
    assert!(!result.contains("<<<<<<<"));
}

#[test]
fn conflicting_merge_with_left_strategy_exits_0() {
    let base = temp_file("line1\ncommon\nline3\n");
    let ours = temp_file("line1\nours-change\nline3\n");
    let theirs = temp_file("line1\ntheirs-change\nline3\n");

    weavr_cmd()
        .args([
            "merge-driver",
            base.path().to_str().unwrap(),
            ours.path().to_str().unwrap(),
            theirs.path().to_str().unwrap(),
            "7",
            "test.txt",
            "--strategy=left",
        ])
        .assert()
        .success();

    let result = std::fs::read_to_string(ours.path()).unwrap();
    assert!(result.contains("ours-change"));
    assert!(!result.contains("theirs-change"));
    assert!(!result.contains("<<<<<<<"));
}

#[test]
fn conflicting_merge_with_right_strategy_exits_0() {
    let base = temp_file("line1\ncommon\nline3\n");
    let ours = temp_file("line1\nours-change\nline3\n");
    let theirs = temp_file("line1\ntheirs-change\nline3\n");

    weavr_cmd()
        .args([
            "merge-driver",
            base.path().to_str().unwrap(),
            ours.path().to_str().unwrap(),
            theirs.path().to_str().unwrap(),
            "7",
            "test.txt",
            "--strategy=right",
        ])
        .assert()
        .success();

    let result = std::fs::read_to_string(ours.path()).unwrap();
    assert!(result.contains("theirs-change"));
    assert!(!result.contains("ours-change"));
}

#[test]
fn conflicting_merge_with_both_strategy_exits_0() {
    let base = temp_file("line1\ncommon\nline3\n");
    let ours = temp_file("line1\nours-change\nline3\n");
    let theirs = temp_file("line1\ntheirs-change\nline3\n");

    weavr_cmd()
        .args([
            "merge-driver",
            base.path().to_str().unwrap(),
            ours.path().to_str().unwrap(),
            theirs.path().to_str().unwrap(),
            "7",
            "test.txt",
            "--strategy=both",
        ])
        .assert()
        .success();

    let result = std::fs::read_to_string(ours.path()).unwrap();
    assert!(result.contains("ours-change"));
    assert!(result.contains("theirs-change"));
}

#[test]
fn default_strategy_resolves_conflicts() {
    // Without --strategy, should fall back to config default (left)
    let base = temp_file("line1\ncommon\nline3\n");
    let ours = temp_file("line1\nours-change\nline3\n");
    let theirs = temp_file("line1\ntheirs-change\nline3\n");

    weavr_cmd()
        .env_remove("WEAVR_MERGE_STRATEGY")
        .args([
            "merge-driver",
            base.path().to_str().unwrap(),
            ours.path().to_str().unwrap(),
            theirs.path().to_str().unwrap(),
        ])
        .assert()
        .success();

    let result = std::fs::read_to_string(ours.path()).unwrap();
    // Default strategy is "left", so ours-change should be kept
    assert!(result.contains("ours-change"));
    assert!(!result.contains("<<<<<<<"));
}

#[test]
fn merge_driver_output_flag_writes_to_separate_file() {
    let base = temp_file("line1\ncommon\nline3\n");
    let ours = temp_file("line1\ncommon\nline3\n");
    let theirs = temp_file("line1\ncommon\nline3-changed\n");

    let output_file = tempfile::NamedTempFile::new().unwrap();

    weavr_cmd()
        .args([
            "merge-driver",
            base.path().to_str().unwrap(),
            ours.path().to_str().unwrap(),
            theirs.path().to_str().unwrap(),
            "--output",
            output_file.path().to_str().unwrap(),
        ])
        .assert()
        .success();

    // Result written to output file, not ours
    let result = std::fs::read_to_string(output_file.path()).unwrap();
    assert!(result.contains("line3-changed"));

    // Ours should be untouched
    let ours_content = std::fs::read_to_string(ours.path()).unwrap();
    assert_eq!(ours_content, "line1\ncommon\nline3\n");
}

#[test]
fn bare_weavr_still_works() {
    // Ensure `weavr` with no subcommand still parses successfully.
    // It may exit 0 ("No conflicted files found") or non-zero depending
    // on VCS context, but the key is that it doesn't fail with a parse error.
    let output = weavr_cmd().output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("error: unrecognized subcommand"),
        "bare weavr should not require a subcommand"
    );
}
