//! Integration tests for the `init` subcommand.

use assert_cmd::Command;
use tempfile::TempDir;

fn weavr_cmd() -> Command {
    Command::cargo_bin("weavr").unwrap()
}

fn init_git_repo(dir: &TempDir) {
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .expect("failed to init git repo");
}

#[test]
fn init_creates_all_files() {
    let dir = TempDir::new().unwrap();
    init_git_repo(&dir);

    weavr_cmd()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("initialized successfully"))
        .stdout(predicates::str::contains("created .weavr.toml"))
        .stdout(predicates::str::contains("configured merge driver"))
        .stdout(predicates::str::contains("*.rs merge=weavr"));

    // Verify .weavr.toml exists
    let config = std::fs::read_to_string(dir.path().join(".weavr.toml")).unwrap();
    assert!(config.contains("[theme]"));

    // Verify .gitattributes exists
    let gitattributes = std::fs::read_to_string(dir.path().join(".gitattributes")).unwrap();
    assert!(gitattributes.contains("*.rs merge=weavr"));
}

#[test]
fn init_idempotent_second_run() {
    let dir = TempDir::new().unwrap();
    init_git_repo(&dir);

    // First run
    weavr_cmd()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Second run — nothing to do
    weavr_cmd()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("nothing to do"));
}

#[test]
fn init_force_overwrites() {
    let dir = TempDir::new().unwrap();
    init_git_repo(&dir);

    // First run
    weavr_cmd()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Overwrite config
    std::fs::write(dir.path().join(".weavr.toml"), "custom content").unwrap();

    // Force run
    weavr_cmd()
        .args(["init", "--force"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("overwrote .weavr.toml"));

    let config = std::fs::read_to_string(dir.path().join(".weavr.toml")).unwrap();
    assert!(config.contains("[theme]"));
}

#[test]
fn init_custom_patterns() {
    let dir = TempDir::new().unwrap();
    init_git_repo(&dir);

    weavr_cmd()
        .args(["init", "--patterns", "*.rs,*.go"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("*.rs merge=weavr"))
        .stdout(predicates::str::contains("*.go merge=weavr"));

    let gitattributes = std::fs::read_to_string(dir.path().join(".gitattributes")).unwrap();
    assert!(gitattributes.contains("*.rs merge=weavr"));
    assert!(gitattributes.contains("*.go merge=weavr"));
}

#[test]
fn init_no_git_skips_driver() {
    let dir = TempDir::new().unwrap();

    weavr_cmd()
        .args(["init", "--no-git"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("created .weavr.toml"));

    assert!(dir.path().join(".weavr.toml").exists());
    assert!(!dir.path().join(".gitattributes").exists());
}
