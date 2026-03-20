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
    // Verify hunks array is present with per-hunk metadata
    let hunks = results[0]["hunks"].as_array().expect("hunks array");
    assert_eq!(hunks.len(), 1);
    assert_eq!(hunks[0]["hunk_id"], 0);
    assert_eq!(hunks[0]["strategy"], "left");
    // Non-AI hunks should not have provider/confidence/explanation
    assert!(hunks[0].get("provider").is_none());
    assert!(hunks[0].get("confidence").is_none());
    assert!(hunks[0].get("explanation").is_none());
    // No old ai field
    assert!(results[0].get("ai").is_none());
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

const MULTI_HUNK_CONTENT: &str = "\
before
<<<<<<< HEAD
left one
=======
right one
>>>>>>> branch
middle
<<<<<<< HEAD
left two
=======
right two
>>>>>>> branch
after
";

#[test]
fn headless_json_multi_hunk_left_strategy() {
    let f = temp_file(MULTI_HUNK_CONTENT);

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
    assert_eq!(results[0]["hunks_resolved"], 2);

    let hunks = results[0]["hunks"].as_array().expect("hunks array");
    assert_eq!(hunks.len(), 2);
    for (i, hunk) in hunks.iter().enumerate() {
        assert_eq!(hunk["hunk_id"], i as u64);
        assert_eq!(hunk["strategy"], "left");
        assert!(hunk.get("provider").is_none());
    }
}

#[test]
fn resolve_json_output_has_hunks_array() {
    let f = temp_file(CONFLICTED_CONTENT);

    // Write resolutions JSON to a temp file
    let mut res = NamedTempFile::new().unwrap();
    write!(
        res,
        r#"{{"resolutions": [{{"hunk_id": 0, "strategy": "left"}}]}}"#
    )
    .unwrap();
    res.flush().unwrap();

    let output = weavr_cmd()
        .args([
            "resolve",
            "--format=json",
            "--dry-run",
            "--resolutions",
            res.path().to_str().unwrap(),
            f.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success(), "exit code was not 0");
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    let results = json["results"].as_array().expect("results array");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["resolved_hunks"], 1);
    assert_eq!(results[0]["written"], false);

    // Verify hunks array is present
    let hunks = results[0]["hunks"].as_array().expect("hunks array");
    assert_eq!(hunks.len(), 1);
    assert_eq!(hunks[0]["hunk_id"], 0);
    assert_eq!(hunks[0]["strategy"], "left");

    // No legacy ai field
    assert!(results[0].get("ai").is_none());
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

// --- Merge driver JSON integration tests ---

#[test]
fn merge_driver_json_on_stderr_not_stdout() {
    let base = temp_file("line one\nline two\n");
    let ours = temp_file("line one\nline two\n");
    let theirs = temp_file("line one\nline two\n");

    let output = weavr_cmd()
        .args([
            "merge-driver",
            "--format=json",
            base.path().to_str().unwrap(),
            ours.path().to_str().unwrap(),
            theirs.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success(), "exit code was not 0");
    // stdout should be empty — git owns stdout for merge drivers
    assert!(
        output.stdout.is_empty(),
        "merge driver JSON should not appear on stdout"
    );
    // stderr should contain valid JSON
    let json: Value =
        serde_json::from_slice(&output.stderr).expect("stderr should contain valid JSON");
    assert_eq!(json["clean_merge"], true);
    assert_eq!(json["written"], true);
    // Clean merge should not have a strategy field
    assert!(json.get("strategy").is_none());
}

#[test]
fn merge_driver_json_log_file() {
    let base = temp_file("line one\nline two\n");
    let ours = temp_file("line one\nline two\n");
    let theirs = temp_file("line one\nline two\n");
    let log_file = NamedTempFile::new().unwrap();
    let log_path = log_file.path().to_path_buf();
    // Close the temp file so the merge driver can write to it
    drop(log_file);

    let output = weavr_cmd()
        .args([
            "merge-driver",
            "--format=json",
            "--log-file",
            log_path.to_str().unwrap(),
            base.path().to_str().unwrap(),
            ours.path().to_str().unwrap(),
            theirs.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success(), "exit code was not 0");
    // Log file should contain valid JSON
    let log_content = std::fs::read_to_string(&log_path).unwrap();
    let json: Value =
        serde_json::from_str(&log_content).expect("log file should contain valid JSON");
    assert_eq!(json["clean_merge"], true);

    // Clean up
    let _ = std::fs::remove_file(&log_path);
}

#[test]
fn merge_driver_log_file_without_json_format_fails() {
    let base = temp_file("base\n");
    let ours = temp_file("ours\n");
    let theirs = temp_file("theirs\n");

    weavr_cmd()
        .args([
            "merge-driver",
            "--log-file",
            "/tmp/weavr-test.log",
            base.path().to_str().unwrap(),
            ours.path().to_str().unwrap(),
            theirs.path().to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "--log-file requires --format=json",
        ));
}
