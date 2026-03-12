//! Integration tests for weavr-jj.
//!
//! These tests require jj to be installed. They are skipped at runtime
//! if the `jj` binary is not available.

use std::path::PathBuf;
use std::process::Command;

use tempfile::TempDir;
use weavr_jj::{JjRepo, VcsBackend};

/// Returns true if jj is available on the system.
fn jj_available() -> bool {
    Command::new("jj")
        .arg("version")
        .output()
        .is_ok_and(|o| o.status.success())
}

/// Helper macro to skip tests when jj is not installed.
macro_rules! require_jj {
    () => {
        if !jj_available() {
            eprintln!("skipping test: jj not installed");
            return;
        }
    };
}

/// Helper to create a jj repository in a temp directory.
fn setup_jj_repo() -> TempDir {
    let dir = TempDir::new().expect("create temp dir");

    // jj init creates a new repo; use git backend for compatibility
    Command::new("jj")
        .args(["git", "init"])
        .current_dir(dir.path())
        .output()
        .expect("jj git init");

    dir
}

/// Helper to canonicalize paths for comparison (handles macOS /var -> /private/var).
fn canonicalize_for_comparison(path: &std::path::Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

#[test]
fn discover_from_root() {
    require_jj!();
    let dir = setup_jj_repo();

    let repo = JjRepo::discover_from(dir.path()).expect("should discover repo");
    assert_eq!(
        canonicalize_for_comparison(repo.root()),
        canonicalize_for_comparison(dir.path())
    );
}

#[test]
fn discover_from_subdirectory() {
    require_jj!();
    let dir = setup_jj_repo();

    let subdir = dir.path().join("deep/nested/directory");
    std::fs::create_dir_all(&subdir).expect("create subdirs");

    let repo = JjRepo::discover_from(&subdir).expect("should discover repo from subdirectory");
    assert_eq!(
        canonicalize_for_comparison(repo.root()),
        canonicalize_for_comparison(dir.path())
    );
}

#[test]
fn discover_not_jj_repo() {
    require_jj!();
    let dir = TempDir::new().expect("create temp dir");
    let result = JjRepo::discover_from(dir.path());
    assert!(result.is_err());
}

#[test]
fn no_conflicts_when_clean() {
    require_jj!();
    let dir = setup_jj_repo();

    // Create a file so there's some content
    std::fs::write(dir.path().join("file.txt"), "content").expect("write file");

    let repo = JjRepo::discover_from(dir.path()).expect("discover repo");
    let conflicts = repo.conflicted_files().expect("get conflicts");
    assert!(conflicts.is_empty());
}

#[test]
fn vcs_backend_name_returns_jj() {
    require_jj!();
    let dir = setup_jj_repo();

    let repo = JjRepo::discover_from(dir.path()).expect("discover repo");
    let backend: &dyn VcsBackend = &repo;
    assert_eq!(backend.name(), "jj");
}

#[test]
fn vcs_backend_stage_file_is_noop() {
    require_jj!();
    let dir = setup_jj_repo();

    std::fs::write(dir.path().join("file.txt"), "content").expect("write file");

    let repo = JjRepo::discover_from(dir.path()).expect("discover repo");
    let backend: &dyn VcsBackend = &repo;

    // stage_file should succeed as a no-op
    backend
        .stage_file(&PathBuf::from("file.txt"))
        .expect("stage_file should be a no-op");
}
