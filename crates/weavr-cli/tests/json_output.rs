//! Integration tests for `--format=json` across modes.

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
fn headless_json_output() {
    let f = temp_file(CONFLICTED_CONTENT);

    let output = weavr_cmd()
        .args([
            "--headless",
            "--strategy=left",
            "--format=json",
            f.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success(), "exit code was not 0");
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    let results = json["results"].as_array().expect("results array");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["hunks_resolved"], 1);
    assert_eq!(results[0]["strategy"], "left");
    assert_eq!(results[0]["written"], true);
}

#[test]
fn headless_json_dry_run() {
    let f = temp_file(CONFLICTED_CONTENT);
    let original = std::fs::read_to_string(f.path()).unwrap();

    let output = weavr_cmd()
        .args([
            "--headless",
            "--dry-run",
            "--strategy=left",
            "--format=json",
            f.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    let results = json["results"].as_array().expect("results array");
    assert_eq!(results[0]["written"], false);

    // File should be unchanged in dry-run mode
    let after = std::fs::read_to_string(f.path()).unwrap();
    assert_eq!(original, after);
}

#[test]
fn format_json_rejected_in_tui_mode() {
    let f = temp_file(CONFLICTED_CONTENT);

    weavr_cmd()
        .args(["--format=json", f.path().to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicates::str::contains("not supported in TUI mode"));
}
