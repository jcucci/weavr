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

fn jj_available() -> bool {
    std::process::Command::new("jj")
        .arg("version")
        .output()
        .is_ok_and(|o| o.status.success())
}

fn init_jj_repo(dir: &TempDir) {
    std::process::Command::new("jj")
        .args(["git", "init"])
        .current_dir(dir.path())
        .output()
        .expect("failed to init jj repo");
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

#[test]
fn init_no_jj_skips_jj_setup() {
    let dir = TempDir::new().unwrap();
    init_git_repo(&dir);

    weavr_cmd()
        .args(["init", "--no-jj"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("created .weavr.toml"));

    // Should not mention jj in output
    let output = weavr_cmd()
        .args(["init", "--no-jj"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains("jj merge tool"));
}

#[test]
fn init_in_jj_repo_configures_merge_tool() {
    if !jj_available() {
        eprintln!("skipping: jj not available");
        return;
    }

    let dir = TempDir::new().unwrap();
    init_jj_repo(&dir);

    weavr_cmd()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicates::str::contains(
            "configured jj merge tool (repo scope)",
        ));

    // Verify via jj config get
    let output = std::process::Command::new("jj")
        .args(["config", "get", "merge-tools.weavr.program"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let value = String::from_utf8(output.stdout).unwrap();
    assert_eq!(value.trim(), "weavr");
}

#[test]
fn init_in_colocated_repo_configures_both() {
    if !jj_available() {
        eprintln!("skipping: jj not available");
        return;
    }

    let dir = TempDir::new().unwrap();
    // jj git init creates a colocated git+jj repo
    init_jj_repo(&dir);

    weavr_cmd()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("configured merge driver"))
        .stdout(predicates::str::contains("configured jj merge tool"));
}

#[test]
fn init_jj_idempotent() {
    if !jj_available() {
        eprintln!("skipping: jj not available");
        return;
    }

    let dir = TempDir::new().unwrap();
    init_jj_repo(&dir);

    // First run
    weavr_cmd()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("configured jj merge tool"));

    // Second run — nothing to do
    weavr_cmd()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("nothing to do"));
}

#[test]
fn init_jj_scope_user() {
    if !jj_available() {
        eprintln!("skipping: jj not available");
        return;
    }

    let dir = TempDir::new().unwrap();
    init_jj_repo(&dir);

    // Use JJ_CONFIG to isolate user-scope config
    let jj_config = dir.path().join("jj-user-config.toml");
    std::fs::write(&jj_config, "").unwrap();

    weavr_cmd()
        .args(["init", "--jj-scope", "user"])
        .current_dir(dir.path())
        .env("JJ_CONFIG", &jj_config)
        .assert()
        .success()
        .stdout(predicates::str::contains(
            "configured jj merge tool (user scope)",
        ));
}
