//! Integration tests for `--check` mode.

use std::io::Write;

use assert_cmd::Command;
use serde_json::Value;
use tempfile::NamedTempFile;

const CONFLICTED_CONTENT: &str = "\
before
<<<<<<< HEAD
left content
=======
right content
>>>>>>> branch
after
";

const CLEAN_CONTENT: &str = "just some clean text\n";

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
fn check_file_with_conflicts_exits_1() {
    let f = temp_file(CONFLICTED_CONTENT);

    weavr_cmd()
        .args(["--check", f.path().to_str().unwrap()])
        .assert()
        .code(1)
        .stdout(predicates::str::contains("1 conflict(s)"));
}

#[test]
fn check_clean_file_exits_0() {
    let f = temp_file(CLEAN_CONTENT);

    weavr_cmd()
        .args(["--check", f.path().to_str().unwrap()])
        .assert()
        .code(0)
        .stdout(predicates::str::contains("0 conflict(s)"));
}

#[test]
fn check_quiet_with_conflicts_exits_1_no_stdout() {
    let f = temp_file(CONFLICTED_CONTENT);

    weavr_cmd()
        .args(["--check", "--quiet", f.path().to_str().unwrap()])
        .assert()
        .code(1)
        .stdout(predicates::str::is_empty());
}

#[test]
fn check_conflicts_with_headless() {
    weavr_cmd()
        .args(["--check", "--headless"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("cannot be used with"));
}

#[test]
fn check_multiple_files_mixed() {
    let conflicted = temp_file(CONFLICTED_CONTENT);
    let clean = temp_file(CLEAN_CONTENT);

    weavr_cmd()
        .args([
            "--check",
            conflicted.path().to_str().unwrap(),
            clean.path().to_str().unwrap(),
        ])
        .assert()
        .code(1)
        .stdout(predicates::str::contains("1 conflict(s) in 1 file(s)"));
}

#[test]
fn check_json_output_with_conflicts() {
    let f = temp_file(CONFLICTED_CONTENT);

    let output = weavr_cmd()
        .args(["--check", "--format=json", f.path().to_str().unwrap()])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    assert_eq!(json["total_conflicts"], 1);
    assert_eq!(json["files"].as_array().unwrap().len(), 1);
    assert_eq!(json["files"][0]["conflict_count"], 1);
}

#[test]
fn check_json_output_clean() {
    let f = temp_file(CLEAN_CONTENT);

    let output = weavr_cmd()
        .args(["--check", "--format=json", f.path().to_str().unwrap()])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    assert_eq!(json["total_conflicts"], 0);
}

#[test]
fn check_json_quiet_still_suppresses() {
    let f = temp_file(CONFLICTED_CONTENT);

    weavr_cmd()
        .args([
            "--check",
            "--quiet",
            "--format=json",
            f.path().to_str().unwrap(),
        ])
        .assert()
        .code(1)
        .stdout(predicates::str::is_empty());
}
