//! Integration tests for VCS backend auto-detection.
//!
//! These tests use `tempfile` and `git init` / `jj init` to create
//! real (but tiny) repositories and verify that the correct backend
//! is discovered.

use std::process::Command;

/// Returns `true` if the `jj` binary is available on `$PATH`.
fn jj_available() -> bool {
    Command::new("jj")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

// -----------------------------------------------------------------------
// Git-only repo
// -----------------------------------------------------------------------

#[test]
fn auto_selects_git_in_git_repo() {
    let dir = tempfile::tempdir().unwrap();
    let status = Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir.path())
        .status()
        .unwrap();
    assert!(status.success(), "git init failed");

    let repo = weavr_git::GitRepo::discover_from(dir.path());
    assert!(repo.is_ok(), "should discover git repo");
}

// -----------------------------------------------------------------------
// jj-only repo
// -----------------------------------------------------------------------

#[cfg(feature = "jj")]
#[test]
fn auto_selects_jj_in_jj_repo() {
    if !jj_available() {
        eprintln!("skipping: jj not available");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    let status = Command::new("jj")
        .args(["git", "init"])
        .current_dir(dir.path())
        .status()
        .unwrap();
    assert!(status.success(), "jj git init failed");

    let repo = weavr_jj::JjRepo::discover_from(dir.path());
    assert!(repo.is_ok(), "should discover jj repo");
}

// -----------------------------------------------------------------------
// Colocated repo (jj + git) — jj should be preferred
// -----------------------------------------------------------------------

#[cfg(feature = "jj")]
#[test]
fn auto_prefers_jj_in_colocated_repo() {
    if !jj_available() {
        eprintln!("skipping: jj not available");
        return;
    }

    let dir = tempfile::tempdir().unwrap();

    // Create a git repo first, then colocate jj
    let status = Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir.path())
        .status()
        .unwrap();
    assert!(status.success(), "git init failed");

    let status = Command::new("jj")
        .args(["git", "init", "--colocate"])
        .current_dir(dir.path())
        .status()
        .unwrap();
    assert!(status.success(), "jj git init --colocate failed");

    // Both should be discoverable
    let jj_repo = weavr_jj::JjRepo::discover_from(dir.path());
    let git_repo = weavr_git::GitRepo::discover_from(dir.path());
    assert!(jj_repo.is_ok(), "jj should be discoverable");
    assert!(git_repo.is_ok(), "git should be discoverable");
}

// -----------------------------------------------------------------------
// --vcs git forces git even in colocated repo
// -----------------------------------------------------------------------

#[test]
fn vcs_git_forces_git_backend() {
    let dir = tempfile::tempdir().unwrap();
    let status = Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir.path())
        .status()
        .unwrap();
    assert!(status.success(), "git init failed");

    let repo = weavr_git::GitRepo::discover_from(dir.path());
    assert!(repo.is_ok(), "git should be discoverable when forced");
}
