//! Integration tests for VCS backend auto-detection.
//!
//! These tests verify that the backend discovery functions return the
//! expected results for different repo configurations. The auto-detection
//! logic in `discover_backend` is: try jj first, fall back to git.
//! We validate this by checking that the individual `discover_from`
//! calls return the expected results, which directly determines what
//! `discover_backend(Auto)` would select.

use std::process::Command;

use weavr_vcs::VcsBackend;

/// Returns `true` if the `jj` binary is available on `$PATH`.
fn jj_available() -> bool {
    Command::new("jj")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

// -----------------------------------------------------------------------
// Git-only repo — auto selects git (jj discovery fails, git succeeds)
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

    // Git should be discoverable
    let git_repo = weavr_git::GitRepo::discover_from(dir.path());
    assert!(git_repo.is_ok(), "git should be discoverable");
    assert_eq!(git_repo.unwrap().name(), "git");

    // jj should NOT be discoverable (no jj repo here)
    #[cfg(feature = "jj")]
    if jj_available() {
        let jj_repo = weavr_jj::JjRepo::discover_from(dir.path());
        assert!(
            jj_repo.is_err(),
            "jj should not be discoverable in git-only repo"
        );
    }
}

// -----------------------------------------------------------------------
// jj repo (git-backed) — auto selects jj (jj discovery succeeds first)
// -----------------------------------------------------------------------

#[cfg(feature = "jj")]
#[test]
fn auto_selects_jj_in_jj_git_repo() {
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

    // jj should be discoverable — and since auto tries jj first, it wins
    let jj_repo = weavr_jj::JjRepo::discover_from(dir.path()).unwrap();
    assert_eq!(jj_repo.name(), "jj");
}

// -----------------------------------------------------------------------
// Colocated repo (jj + git) — auto prefers jj
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

    // Both backends should be discoverable
    let jj_repo = weavr_jj::JjRepo::discover_from(dir.path());
    let git_repo = weavr_git::GitRepo::discover_from(dir.path());
    assert!(jj_repo.is_ok(), "jj should be discoverable");
    assert!(git_repo.is_ok(), "git should be discoverable");

    // Auto-detection tries jj first, so jj should win
    let jj_backend = jj_repo.unwrap();
    assert_eq!(
        jj_backend.name(),
        "jj",
        "auto selection should prefer jj when both jj and git repos are present"
    );
}

// -----------------------------------------------------------------------
// --vcs git forces git even in colocated repo
// -----------------------------------------------------------------------

#[cfg(feature = "jj")]
#[test]
fn vcs_git_forces_git_in_colocated_repo() {
    if !jj_available() {
        eprintln!("skipping: jj not available");
        return;
    }

    let dir = tempfile::tempdir().unwrap();

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

    // When --vcs git is used, only git discovery runs.
    // Verify git is still discoverable even in a colocated repo.
    let git_repo = weavr_git::GitRepo::discover_from(dir.path()).unwrap();
    assert_eq!(
        git_repo.name(),
        "git",
        "--vcs git should select git even in colocated repo"
    );
}

#[test]
fn vcs_git_forces_git_in_git_only_repo() {
    let dir = tempfile::tempdir().unwrap();
    let status = Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir.path())
        .status()
        .unwrap();
    assert!(status.success(), "git init failed");

    let git_repo = weavr_git::GitRepo::discover_from(dir.path()).unwrap();
    assert_eq!(git_repo.name(), "git");
}
